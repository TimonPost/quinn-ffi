use crate::{
    ffi::callbacks,
    proto,
    proto::VarInt,
    proto_impl::{
        endpoint::EndpointEvent,
        result::FFIErrorKind,
    },
};
use bytes::Bytes;
use quinn_proto::StreamEvent;
use std::{
    sync::{
        mpsc,
        mpsc::Sender,
    },
    time::Instant,
};

/// Events for the connection.
#[derive(Debug)]
pub enum ConnectionEvent {
    /// Connection should close.
    Close { error_code: VarInt, reason: Vec<u8> },
    /// Protocol logic.
    Proto(proto::ConnectionEvent),
    /// Connection should ping.
    Ping,
}

/// A QUIC connection using quinn-proto.
pub struct ConnectionImpl {
    pub(crate) inner: proto::Connection,
    pub(crate) connection_handle: proto::ConnectionHandle,
    connection_events: mpsc::Receiver<ConnectionEvent>,
    endpoint_events: Sender<(proto::ConnectionHandle, EndpointEvent)>,
    timer_deadline: Option<Instant>,
    last_poll: Instant,
    endpoint_poll_notifier: Option<Sender<i8>>,
}

impl ConnectionImpl {
    pub(crate) fn new(
        inner: proto::Connection,
        handle: proto::ConnectionHandle,
        recv: mpsc::Receiver<ConnectionEvent>,
        endpoint_events_tx: Sender<(proto::ConnectionHandle, EndpointEvent)>,
        endpoint_poll_notifier: Option<Sender<i8>>,
    ) -> ConnectionImpl {
        ConnectionImpl {
            inner,
            connection_events: recv,
            endpoint_events: endpoint_events_tx,
            connection_handle: handle,
            timer_deadline: None,
            last_poll: Instant::now(),
            endpoint_poll_notifier,
        }
    }
}

impl ConnectionImpl {
    /// Polls the connection.
    ///
    /// 1. Handles connection events.
    /// 2. Handles transmits.
    /// 3. Handles timeout
    /// 4. Handles endpoint events.
    /// 5. Handles app events.
    ///
    /// Polling the connection might result in callbacks to the client application.
    pub fn poll(&mut self) -> Result<(), FFIErrorKind> {
        let _ = self.handle_connection_events();

        let mut poll_again = self.handle_timer();
        let _ = self.handle_endpoint_events();
        self.handle_app_events();
        poll_again |= self.handle_transmits()?;

        Ok(())
    }

    /// Marks the connection as pollable.
    /// Connection should be polled when IO operations are performed, and timeout happened.
    ///
    /// This will poll the connection if `auto-poll` feature is enabled, else it will invoke the client application set callback.
    pub fn mark_pollable(&mut self) -> Result<(), FFIErrorKind> {
        if cfg!(feature = "auto-poll") {
            self.poll()?;
            // is initialized when auto-poll is enabled.
            self.endpoint_poll_notifier.as_ref().unwrap().send(0)?;
        } else {
            callbacks::on_connection_pollable(self.connection_id())
        }

        Ok(())
    }

    pub fn close(&mut self, error_code: VarInt, reason: &[u8]) {
        self.inner
            .close(Instant::now(), error_code, Bytes::copy_from_slice(reason));
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

    fn handle_transmits(&mut self) -> Result<bool, FFIErrorKind> {
        let mut should_notify = false;
        while let Some(t) = self.inner.poll_transmit(Instant::now(), 1) {
            self.endpoint_events
                .send((self.connection_handle, EndpointEvent::Transmit(t)))?;
            should_notify = true;
            // TODO: when max transmits return true.
        }

        return Ok(should_notify);
    }

    fn handle_endpoint_events(&mut self) -> Result<(), FFIErrorKind> {
        while let Some(event) = self.inner.poll_endpoint_events() {
            self.endpoint_events
                .send((self.connection_handle, EndpointEvent::Proto(event)))?;

            if cfg!(feature = "auto-poll") {
                self.endpoint_poll_notifier.as_ref().unwrap().send(0)?;
            }
        }
        Ok(())
    }
    fn handle_connection_events(&mut self) -> Result<(), FFIErrorKind> {
        let event = self.connection_events.try_recv()?;

        match event {
            ConnectionEvent::Close { error_code, reason } => callbacks::on_connection_close(
                self.connection_id(),
                error_code.into_inner(),
                reason.as_ptr(),
                reason.len() as u32,
            ),
            ConnectionEvent::Proto(proto) => {
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
                Connected => callbacks::on_connected(self.connection_id()),
                ConnectionLost { reason: _ } => {
                    // TODO: self.terminate(reason);
                    callbacks::on_connection_lost(self.connection_id())
                }
                Stream(StreamEvent::Writable { id }) => {
                    callbacks::on_stream_writable(self.connection_id(), id)
                }
                Stream(StreamEvent::Opened { dir }) => {
                    if let Some(stream_id) = self.inner.streams().accept(dir) {
                        callbacks::on_stream_opened(
                            self.connection_id(),
                            VarInt::from(stream_id).into_inner(),
                            dir as u8,
                        );
                    }
                }
                DatagramReceived => {
                    callbacks::on_datagram_received(self.connection_id());
                }
                Stream(StreamEvent::Readable { id }) => {
                    callbacks::on_stream_readable(self.connection_id(), id);
                }
                Stream(StreamEvent::Available { dir }) => {
                    callbacks::on_stream_available(self.connection_id(), dir as u8);
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
