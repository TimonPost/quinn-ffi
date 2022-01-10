use crate::{
    ffi::bindings::callbacks,
    proto,
    proto::VarInt,
    proto_impl::{
        endpoint::EndpointEvent,
        result::QuinnErrorKind,
    },
};
use quinn_proto::{
    Dir,
    StreamEvent,
};
use std::{
    sync::{
        mpsc,
        mpsc::Sender,
    },
    time::Instant,
};

#[derive(Debug)]
pub enum ConnectionEvent {
    Close { error_code: VarInt, reason: Vec<u8> },
    Proto(proto::ConnectionEvent),
    Ping,
}

pub struct ConnectionInner {
    pub(crate) inner: proto::Connection,
    pub connected: bool,
    pub connection_events: mpsc::Receiver<ConnectionEvent>,
    pub endpoint_events: Sender<(proto::ConnectionHandle, EndpointEvent)>,
    pub connection_handle: proto::ConnectionHandle,

    handle_event_called: bool,

    timer_deadline: Option<Instant>,
    last_poll: Instant,
    pub endpoint_poll_notifier: Sender<u8>,
}

impl ConnectionInner {
    pub(crate) fn new(
        connection: proto::Connection,
        handle: proto::ConnectionHandle,
        recv: mpsc::Receiver<ConnectionEvent>,
        endpoint_events_tx: Sender<(proto::ConnectionHandle, EndpointEvent)>,
        endpoint_poll_notifier: Sender<u8>,
    ) -> ConnectionInner {
        ConnectionInner {
            inner: connection,
            connected: false,
            connection_events: recv,
            endpoint_events: endpoint_events_tx,
            connection_handle: handle,
            handle_event_called: false,
            timer_deadline: None,
            last_poll: Instant::now(),
            endpoint_poll_notifier,
        }
    }
}

impl ConnectionInner {
    pub fn poll(&mut self) -> Result<(), QuinnErrorKind> {
        let _ = self.handle_connection_events();
        let mut poll_again = self.handle_transmits()?;
        poll_again |= self.handle_timer();

        let _ = self.handle_endpoint_events();
        self.handle_app_events();

        if poll_again {
            self.mark_pollable();
        }

        Ok(())
    }

    /// Mark the connection as pollable.
    /// Connection should be polled when IO operations are performed, and timeout happened.
    ///
    /// This will invoke a callback.
    pub fn mark_pollable(&self) {
        callbacks::on_connection_pollable(self.connection_id())
    }

    fn handle_timer(&mut self) -> bool {
        match self.inner.poll_timeout() {
            Some(deadline) => {
                self.timer_deadline = Some(deadline);
            }
            None => {
                self.timer_deadline = None;
                return false;
            }
        }

        let now = Instant::now();

        if now > self.timer_deadline.expect("timer deadline is initialized") {
            self.inner.handle_timeout(Instant::now());
            self.timer_deadline = None;
            return true;
        }

        return false;
    }

    fn handle_transmits(&mut self) -> Result<bool, QuinnErrorKind> {
        while let Some(t) = self.inner.poll_transmit(Instant::now(), 1) {
            self.endpoint_events
                .send((self.connection_handle, EndpointEvent::Transmit(t)))?;

            self.endpoint_poll_notifier.send(0);
            // TODO: when max transmits return true.
        }

        return Ok(false);
    }

    fn handle_endpoint_events(&mut self) -> Result<(), QuinnErrorKind> {
        if let Some(event) = self.inner.poll_endpoint_events() {
            self.endpoint_events
                .send((self.connection_handle, EndpointEvent::Proto(event)))?;
            self.endpoint_poll_notifier.send(0);
        }
        Ok(())
    }
    fn handle_connection_events(&mut self) -> Result<(), QuinnErrorKind> {
        let event = self.connection_events.try_recv()?;

        match event {
            ConnectionEvent::Close { .. } => {
                // TODO: terminate connection
            }
            ConnectionEvent::Proto(proto) => {
                self.handle_event_called = true;
                self.inner.handle_event(proto);
            }
            ConnectionEvent::Ping => {
                self.inner.ping();
            }
        }

        Ok(())
    }

    fn handle_app_events(&mut self) {
        while let Some(event) = self.inner.poll() {
            use quinn_proto::Event::*;
            match event {
                HandshakeDataReady => {
                    // ignore for now
                }
                Connected => {
                    self.connected = true;
                    callbacks::on_connected(self.connection_id())
                }
                ConnectionLost { reason: _ } => {
                    // TODO: self.terminate(reason);
                    callbacks::on_connection_lost(self.connection_id())
                }
                Stream(StreamEvent::Writable { id }) => {
                    callbacks::on_stream_writable(self.connection_id(), id)
                }
                Stream(StreamEvent::Opened { dir: Dir::Uni }) => {
                    callbacks::on_stream_opened(self.connection_id(), Dir::Uni);
                }
                Stream(StreamEvent::Opened { dir: Dir::Bi }) => {
                    println!("opened bi!");
                    callbacks::on_stream_opened(self.connection_id(), Dir::Bi);
                }
                DatagramReceived => {
                    callbacks::on_datagram_received(self.connection_id());
                }
                Stream(StreamEvent::Readable { id }) => {
                    callbacks::on_stream_readable(self.connection_id(), id);
                }
                Stream(StreamEvent::Available { dir }) => {
                    callbacks::on_stream_available(self.connection_id(), dir);
                }
                Stream(StreamEvent::Finished { id }) => {
                    callbacks::on_stream_finished(self.connection_id(), id);
                }
                Stream(StreamEvent::Stopped { id, error_code: _ }) => {
                    callbacks::on_stream_stopped(self.connection_id(), id);
                }
            }
        }
    }

    fn connection_id(&self) -> u32 {
        return self.connection_handle.0 as u32;
    }
}
