use crate::{
    ffi::{
        bindings::callbacks,
    },
    proto,
    proto::VarInt,
    proto_impl::endpoint::EndpointEvent,
};
use quinn_proto::{
    Dir,
    StreamEvent,
};
use std::{sync::{
    mpsc,
    mpsc::Sender,
}, time::Instant, io};
use crate::proto_impl::result::QuinnErrorKind;


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
}

impl ConnectionInner {
    pub fn poll(&mut self)  -> Result<(), QuinnErrorKind> {
        let transmit = self.inner.poll_transmit(Instant::now(), 1);
        let _next_instant = self.inner.poll_timeout();
        let event = self.inner.poll_endpoint_events();

        if let Some(event) = event {
            self.endpoint_events
                .send((self.connection_handle, EndpointEvent::Proto(event)))?;
        }

        if let Some(event) = transmit {
            self.endpoint_events
                .send((self.connection_handle, EndpointEvent::Transmit(event)))?;
        }

        self.handle_app_events();
        self.handle_connection_events()?;

        Ok(())
    }

    fn handle_connection_events(&mut self) -> Result<(), QuinnErrorKind> {
        let event = self.connection_events.try_recv()?;

        match event {
            ConnectionEvent::Close { .. } => {
                // close
            }
            ConnectionEvent::Proto(proto) => {
                self.inner.handle_event(proto);
            }
            ConnectionEvent::Ping => {
                // ping
            }
        }

        Ok(())
    }

    fn handle_app_events(&mut self) {
        while let Some(event) = self.inner.poll() {
            use quinn_proto::Event::*;
            match event {
                HandshakeDataReady => {
                    // Handshake data ready
                }
                Connected => {
                    self.connected = true;
                    callbacks::on_connected(self.connection_id())
                }
                ConnectionLost { reason: _ } => {
                    //self.terminate(reason);
                    callbacks::on_connection_lost(self.connection_id())
                }
                Stream(StreamEvent::Writable { id }) => {
                    callbacks::on_stream_writable(self.connection_id(), id)
                }
                Stream(StreamEvent::Opened { dir: Dir::Uni }) => {
                    callbacks::on_stream_opened(self.connection_id(), Dir::Uni);
                    println!("!!!on uni stream open!!!")
                }
                Stream(StreamEvent::Opened { dir: Dir::Bi }) => {
                    callbacks::on_stream_opened(self.connection_id(), Dir::Bi);
                    println!("!!!on bi stream open!!!")
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
