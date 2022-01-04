use crate::{error, ffi::{
    QuinnResult,
    Ref,
}, proto::{
    DatagramEvent,
    Endpoint,
    EndpointConfig,
}, proto_impl::{
    EndpointInner,
    IpAddr,
}, ConnectionHandle, EndpointHandle, RustlsServerConfigHandle, proto};
use bytes::{BytesMut, Bytes};
use libc::size_t;
use std::{
    sync::{
        Arc,
        Mutex,
    },
    time::Instant,
};
use Into;
use crate::ffi::{Out, QuinnError};
use crate::proto::{StreamId, RecvStream, Dir};
use std::io::{Write, Error};
use std::net::SocketAddr;
use crate::proto_impl::{ConnectionInner, QuinnErrorKind};
use quinn_proto::VarInt;

/// ===== Endpoint API'S ======

#[no_mangle]
pub extern "cdecl" fn create_endpoint(
    handle: RustlsServerConfigHandle,
    mut endpoint_id: Out<u8>,
    mut endpoint_handle: Out<EndpointHandle>,
) -> QuinnResult {
    let endpoint_config = Arc::new(EndpointConfig::default());
    let server_config = handle.clone();

    let proto_endpoint = Endpoint::new(endpoint_config, Some(Arc::from(server_config)));
    let endpoint = EndpointInner::new(proto_endpoint);
    unsafe {
        endpoint_id.init(endpoint.id);
        endpoint_handle.init(EndpointHandle::alloc(Mutex::new(endpoint)))
    }

    QuinnResult::ok()
}

#[no_mangle]
pub extern "cdecl" fn poll_endpoint(handle: EndpointHandle) -> QuinnResult {
    let mut endpoint = handle.lock().unwrap();
    endpoint.poll();

    QuinnResult::ok()
}

#[no_mangle]
pub extern "cdecl" fn handle_datagram(
    endpoint_handle: EndpointHandle,
    data: Ref<u8>,
    length: size_t,
    address: IpAddr,
) -> QuinnResult {
    let mut endpoint = endpoint_handle.lock().unwrap();

    let slice = unsafe { data.as_bytes(length) };

    let addr: SocketAddr = address.into();

    println!("handle: {}", addr);

    match endpoint.inner.handle(
        Instant::now(),
        addr,
        None,
        None,
        BytesMut::from(slice),
    ) {
        Some((handle, DatagramEvent::NewConnection(conn))) => {
            let connection = endpoint.add_connection(handle, conn);

            callbacks::on_new_connection(handle.0 as u32, connection);
        }
        Some((handle, DatagramEvent::ConnectionEvent(event))) => {
            endpoint.forward_event_to_connection(handle, event);
        }
        None => {}
    }

    return QuinnResult::ok();
}

/// ===== Connection API'S ======

#[no_mangle]
pub extern "cdecl" fn get_connection(handle: ConnectionHandle) -> QuinnResult {
    let _id = handle.connection_handle;
    QuinnResult::ok()
}

#[no_mangle]
pub extern "cdecl" fn poll_connection(mut handle: ConnectionHandle) -> QuinnResult {
    if let Err(reason) = handle.poll() {
        QuinnResult::err()
            .context(QuinnError::new(0, reason.to_string()))
    } else {
        QuinnResult::ok()
    }
}

/// ===== Error API'S ======
#[no_mangle]
pub extern "cdecl" fn throw_error() -> QuinnResult {
    error()
}

#[no_mangle]
pub extern "cdecl" fn last_error(
    mut message_buf: Out<u8>,
    message_buf_len: size_t,
    mut actual_message_len: Out<size_t>,
) -> QuinnResult {
    QuinnResult::with_last_result(|last_result| {
        if let Some(error_msg) = last_result {
            let error_as_bytes = error_msg.reason.as_bytes();

            // "The out pointer is valid and not mutably aliased elsewhere"
            unsafe {
                actual_message_len.init(error_as_bytes.len());
            }

            if message_buf_len < error_as_bytes.len() {
                return QuinnResult::buffer_too_small();
            }

            // "The buffer is valid for writes and the length is within the buffer"
            unsafe {
                message_buf.init_bytes(error_as_bytes);
            }
        }
        QuinnResult::ok()
    })
}


/// ===== Stream API'S ======

#[no_mangle]
pub extern "cdecl" fn accept_stream(mut handle: ConnectionHandle, stream_direction: u8, mut stream_id_out: Out<u64>) -> QuinnResult {

    let dir = if stream_direction == 0 {Dir::Bi} else {Dir::Uni};

    if let Some(stream_id) = handle.inner.streams().accept(dir) {
        unsafe {stream_id_out.init(VarInt::from(stream_id).into());}
        QuinnResult::ok()
    } else {
        QuinnResult::err().context(QuinnError::new(0, "No stream to accept!".to_string()))
    }
}


#[no_mangle]
pub extern "cdecl" fn read_stream(mut handle: ConnectionHandle, stream_id: u64, mut message_buf: Out<u8>, message_buf_len: size_t, mut actual_message_len: Out<size_t>) -> QuinnResult {
    _read_stream(&mut handle, stream_id, message_buf, message_buf_len, actual_message_len).into()
}

fn _read_stream(handle: &mut ConnectionInner, stream_id: u64, mut message_buf: Out<u8>, message_buf_len: size_t, mut actual_message_len: Out<size_t>) -> Result<(), QuinnErrorKind>{
    let mut stream = handle.inner.recv_stream(StreamId(stream_id));

    let mut result = stream.read(true)?;

    if let Some(chunk) = result.next(message_buf_len)? {
        unsafe {
            let mut buffer = unsafe { message_buf.as_uninit_bytes_mut(message_buf_len) };

            let written = buffer.write(&chunk.bytes)?;

            unsafe {actual_message_len.init(written);}
        }
    }

    result.finalize();

    Ok(())
}

pub mod callbacks {
    use crate::{
        proto::{
            Dir,
            Transmit,
        },
        proto_impl::{
            ConnectionInner,
            IpAddr,
        },
    };
    use libc::size_t;
    use crate::proto::StreamId;
    use quinn_proto::VarInt;

    // Callbacks should be initialized before applications runs. Therefore we can unwrap unchecked and allow statics to be mutable.
    static mut ON_NEW_CONNECTION: Option<extern "C" fn(super::ConnectionHandle, u32)> = None;
    static mut ON_CONNECTED: Option<extern "C" fn(u32)> = None;
    static mut ON_CONNECTION_LOST: Option<extern "C" fn(u32)> = None;
    static mut ON_STREAM_WRITABLE: Option<extern "C" fn(u32, u64, u8)> = None;
    static mut ON_STREAM_READABLE: Option<extern "C" fn(u32, u64, u8)> = None;
    static mut ON_STREAM_FINISHED: Option<extern "C" fn(u32, u64, u8)> = None;
    static mut ON_STREAM_STOPPED: Option<extern "C" fn(u32, u64, u8)> = None;
    static mut ON_STREAM_AVAILABLE: Option<extern "C" fn(u32, u8)> = None;
    static mut ON_DATAGRAM_RECEIVED: Option<extern "C" fn(u32)> = None;
    static mut ON_STREAM_OPENED: Option<extern "C" fn(u32, u8)> = None;
    static mut ON_TRANSMIT: Option<extern "C" fn(u8, *const u8, size_t, *const IpAddr)> = None;


    pub(crate) fn on_new_connection(con: u32, handle: ConnectionInner) {
        unsafe {
            ON_NEW_CONNECTION.unwrap_unchecked()(super::ConnectionHandle::alloc(handle), con);
        }
    }

    pub(crate) fn on_connected(con: u32) {
        unsafe {
            ON_CONNECTED.unwrap_unchecked()(con);
        }
    }

    pub(crate) fn on_connection_lost(con: u32) {
        unsafe {
            ON_CONNECTION_LOST.unwrap_unchecked()(con);
        }
    }

    pub(crate) fn on_stream_readable(con: u32, stream_id: StreamId) {
        unsafe {
            ON_STREAM_READABLE.unwrap_unchecked()(con, VarInt::from(stream_id).into(), stream_id.dir() as u8);
        }
    }

    pub(crate) fn on_stream_writable(con: u32, stream_id: StreamId) {
        unsafe {
            ON_STREAM_WRITABLE.unwrap_unchecked()(con, VarInt::from(stream_id).into(), stream_id.dir() as u8);
        }
    }

    pub(crate) fn on_stream_finished(con: u32, stream_id: StreamId) {
        unsafe {
            ON_STREAM_FINISHED.unwrap_unchecked()(con, VarInt::from(stream_id).into(), stream_id.dir() as u8);
        }
    }

    pub(crate) fn on_stream_stopped(con: u32, stream_id: StreamId) {
        unsafe {
            ON_STREAM_STOPPED.unwrap_unchecked()(con, VarInt::from(stream_id).into(), stream_id.dir() as u8);
        }
    }

    pub(crate) fn on_stream_available(con: u32, dir: Dir) {
        unsafe {
            ON_STREAM_AVAILABLE.unwrap_unchecked()(con, dir as u8);
        }
    }

    pub(crate) fn on_datagram_received(con: u32) {
        unsafe {
            ON_DATAGRAM_RECEIVED.unwrap_unchecked()(con);
        }
    }

    pub(crate) fn on_stream_opened(con: u32, dir: Dir) {
        unsafe {
            ON_STREAM_OPENED.unwrap_unchecked()(con, dir as u8);
        }
    }

    pub(crate) fn on_transmit(endpoint_id: u8, transmit: Transmit) {
        unsafe {
            let addr = transmit.destination.into();
            ON_TRANSMIT.unwrap_unchecked()(
                endpoint_id,
                transmit.contents.as_ptr(),
                transmit.contents.len(),
                &addr,
            );
        }
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_new_connection(cb: extern "C" fn(super::ConnectionHandle, u32)) {
        unsafe {
            ON_NEW_CONNECTION = Some(cb);
        }
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_connected(cb: extern "C" fn(u32)) {
        unsafe {
            ON_CONNECTED = Some(cb);
        }
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_connection_lost(cb: extern "C" fn(u32)) {
        unsafe {
            ON_CONNECTION_LOST = Some(cb);
        }
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_stream_writable(cb: extern "C" fn(u32, u64, u8)) {
        unsafe {
            ON_STREAM_WRITABLE = Some(cb);
        }
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_stream_readable(cb: extern "C" fn(u32, u64, u8)) {
        unsafe {
            ON_STREAM_READABLE = Some(cb);
        }
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_stream_finished(cb: extern "C" fn(u32, u64, u8)) {
        unsafe {
            ON_STREAM_FINISHED = Some(cb);
        }
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_stream_stopped(cb: extern "C" fn(u32, u64, u8)) {
        unsafe {
            ON_STREAM_STOPPED = Some(cb);
        }
    }

    #[no_mangle]
    pub(crate) fn set_on_stream_available(cb: extern "C" fn(u32, u8)) {
        unsafe {
            ON_STREAM_AVAILABLE = Some(cb);
        }
    }

    #[no_mangle]
    pub(crate) fn set_on_datagram_received(cb: extern "C" fn(u32)) {
        unsafe {
            ON_DATAGRAM_RECEIVED = Some(cb);
        }
    }

    #[no_mangle]
    pub(crate) fn set_on_stream_opened(cb: extern "C" fn(u32, u8)) {
        unsafe {
            ON_STREAM_OPENED = Some(cb);
        }
    }

    #[no_mangle]
    pub(crate) fn set_on_transmit(cb: extern "C" fn(u8, *const u8, size_t, *const IpAddr)) {
        unsafe {
            ON_TRANSMIT = Some(cb);
        }
    }
}
