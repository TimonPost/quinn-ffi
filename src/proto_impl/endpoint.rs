use crate::{
    ffi::callbacks,
    proto,
    proto_impl::connection::{
        ConnectionEvent,
        ConnectionImpl,
    },
};

use quinn_proto::Transmit;

use crate::{
    ffi::Handle,
    proto::{
        ClientConfig,
        ConnectError,
    },
    proto_impl::QuinnErrorKind,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        atomic::{
            AtomicU8,
            Ordering,
        },
        mpsc,
        Arc,
        Mutex,
    },
    thread,
};

use crate::{
    ffi::EndpointHandle,
    proto::ConnectionHandle,
};
use std::sync::{
    MutexGuard,
    TryLockError,
};

/// Maximum number of datagrams processed in send/recv calls to make before moving on to other processing
///
/// This helps ensure we don't starve anything when the CPU is slower than the link.
/// Value is selected by picking a low number which didn't degrade throughput in benchmarks.
const IO_LOOP_BOUND: usize = 160;

/// An endpoint id that is increased for each created endpoint.
static ENDPOINT_ID: AtomicU8 = AtomicU8::new(0);

/// Events for the endpoint.
#[derive(Debug)]
pub enum EndpointEvent {
    /// Protocol logic.
    Proto(proto::EndpointEvent),
    /// Transmit ready.
    Transmit(proto::Transmit),
}

/// Polls the endpoint when notified to do so.
/// This polling happens on its own thread.
pub struct EndpointPoller {
    receiver: mpsc::Receiver<u8>,
    loop_again: bool,
    endpoint_ref: Arc<Mutex<EndpointImpl>>,
}

impl EndpointPoller {
    /// Creates a new `EndpointPoller`.
    pub fn new(endpoint_ref: Arc<Mutex<EndpointImpl>>) -> (Self, mpsc::Sender<u8>) {
        let (sender, receiver) = mpsc::channel();
        (
            EndpointPoller {
                endpoint_ref,
                receiver,
                loop_again: false,
            },
            sender,
        )
    }

    /// Starts polling the endpoint.
    /// This will start a new thread.
    pub fn start_polling(mut self) {
        thread::spawn(move || {
            let mut spin_counter = 0;
            loop {
                if !self.loop_again {
                    let _ = self.receiver.recv();
                }

                if self.loop_again {
                    spin_counter += 1;
                }

                if spin_counter == 0 || (self.loop_again && spin_counter % 1000 == 0) {
                    let lock = self.endpoint_ref.try_lock();
                    match lock {
                        Err(TryLockError::WouldBlock) => {
                            // if blocking, spin thread a bit till lock is released.
                            self.loop_again = true;
                            spin_counter += 1;
                        }
                        Ok(mut e) => {
                            spin_counter = 0;
                            e.poll().expect("Endpoint polling thread panicked!");
                            self.loop_again = false;
                        }
                        _ => {}
                    }
                } else {
                    //println!("spin");
                }
            }
        });
    }
}

/// A QUIC endpoint using quinn-proto.
pub struct EndpointImpl {
    /// The endpoint id.
    pub id: u8,
    pub(crate) inner: proto::Endpoint,
    endpoint_events_rx: mpsc::Receiver<(proto::ConnectionHandle, EndpointEvent)>,
    endpoint_events_tx: mpsc::Sender<(proto::ConnectionHandle, EndpointEvent)>,
    endpoint_poll_notifier: Option<mpsc::Sender<u8>>,
    default_client_config: Option<ClientConfig>,
    connections: HashMap<proto::ConnectionHandle, mpsc::Sender<ConnectionEvent>>,
    // use the refs strictly for polling operations only.
    // Locking a connection could result in deadlocks if the application is already using the lock.
    connection_refs: HashMap<proto::ConnectionHandle, Arc<Mutex<ConnectionImpl>>>,
}

impl EndpointImpl {
    pub fn new(endpoint: proto::Endpoint) -> Self {
        let (tx, rx) = mpsc::channel();

        let id = ENDPOINT_ID.load(Ordering::Relaxed).wrapping_add(1);

        return EndpointImpl {
            inner: endpoint,
            connections: HashMap::new(),
            endpoint_events_tx: tx,
            endpoint_events_rx: rx,
            endpoint_poll_notifier: None,
            id,
            default_client_config: None,
            connection_refs: HashMap::new(),
        };
    }

    /// Sets the endpoint poll notifier.
    /// This sender can be used to trigger a endpoint poll operation.
    pub fn set_poll_notifier(&mut self, notifer: mpsc::Sender<u8>) {
        self.endpoint_poll_notifier = Some(notifer);
    }

    /// Polls the endpoint.
    ///
    /// - Triggers a callback for all outgoing transmits.
    /// - Handles all connection sent endpoint events.
    pub fn poll(&mut self) -> Result<bool, QuinnErrorKind> {
        while let Some(transmit) = self.inner.poll_transmit() {
            // TODO: batch transmits
            self.notify_transmit(transmit);
        }

        // TODO limit max outgoing, invoke callback to poll again.

        self.handle_connection_events()
    }

    /// Creates and adds a connection for this endpoint.
    pub fn add_connection(
        &mut self,
        handle: proto::ConnectionHandle,
        connection: proto::Connection,
    ) -> ConnectionImpl {
        let (send, recv) = mpsc::channel();
        let _ = self.connections.insert(handle, send);

        ConnectionImpl::new(
            connection,
            handle,
            recv,
            self.endpoint_events_tx.clone(),
            self.endpoint_poll_notifier.clone().unwrap(),
        )
    }

    /// Registers a connection for polling.
    /// This is required for auto polling connections.
    pub fn register_pollable_connection(
        &mut self,
        handle: proto::ConnectionHandle,
        connection: Arc<Mutex<ConnectionImpl>>,
    ) {
        self.connection_refs.insert(handle, connection);
    }

    /// Polls a connection by the given connection handle.
    pub fn poll_connection(&self, handle: ConnectionHandle) {
        // if lock is blocked its oke to skip one poll since this function is triggered in various cases.
        if let Some(connection) = self.connection_refs.get(&handle) {
            if let Ok(mut conn) = connection.try_lock() {
                conn.mark_pollable();
            } else {
                println!("Locked endpoint connection lock");
            }
        }
    }

    /// Sends a `ConnectionEvent` to a particular connection.
    pub fn forward_event_to_connection(
        &mut self,
        handle: proto::ConnectionHandle,
        event: proto::ConnectionEvent,
    ) -> Result<(), QuinnErrorKind> {
        self.connections
            .get_mut(&handle)
            .unwrap()
            .send(ConnectionEvent::Proto(event))?;

        Ok(())
    }

    /// Set the client configuration used by `connect`.
    pub fn set_default_client_config(&mut self, config: ClientConfig) {
        self.default_client_config = Some(config);
    }

    /// Connects to a remote endpoint
    ///
    /// `server_name` must be covered by the certificate presented by the server. This prevents a
    /// connection from being intercepted by an attacker with a valid certificate for some other
    /// server.
    ///
    /// May fail immediately due to configuration errors, or in the future if the connection could
    /// not be established.
    pub fn connect(
        &mut self,
        addr: SocketAddr,
        server_name: &str,
    ) -> Result<ConnectionImpl, ConnectError> {
        let config = match &self.default_client_config {
            Some(config) => config.clone(),
            None => return Err(ConnectError::NoDefaultClientConfig),
        };

        self.connect_with(config, addr, server_name)
    }

    /// Connects to a remote endpoint using a custom configuration.
    ///
    /// See [`connect()`] for details.
    ///
    /// [`connect()`]: EndpointImpl::connect
    pub fn connect_with(
        &mut self,
        config: ClientConfig,
        addr: SocketAddr,
        server_name: &str,
    ) -> Result<ConnectionImpl, ConnectError> {
        let (ch, conn) = self.inner.connect(config, addr, server_name)?;

        Ok(self.add_connection(ch, conn))
    }

    /// Handles events sent by connections which in turn might trigger new events for connections.
    fn handle_connection_events(&mut self) -> Result<bool, QuinnErrorKind> {
        for _ in 0..IO_LOOP_BOUND {
            match self.endpoint_events_rx.try_recv() {
                Ok((handle, event)) => {
                    match event {
                        EndpointEvent::Proto(proto) => {
                            if proto.is_drained() {
                                self.connections.remove(&handle);
                                if self.connections.is_empty() {
                                    //self.idle.notify_waiters();
                                }
                            }

                            if let Some(event) = self.inner.handle_event(handle, proto) {
                                // Ignoring errors from dropped connections that haven't yet been cleaned up
                                self.connections
                                    .get_mut(&handle)
                                    .unwrap()
                                    .send(ConnectionEvent::Proto(event))?;
                            }
                        }
                        EndpointEvent::Transmit(transmit) => {
                            self.notify_transmit(transmit);
                        }
                    }
                }
                Err(_) => {
                    // No more messages to be read.
                    return Ok(false);
                }
            }
        }

        return Ok(true);
    }

    /// Invokes a initialized callback by the client application.
    fn notify_transmit(&mut self, transmit: Transmit) {
        callbacks::on_transmit(self.id, transmit);
    }
}
