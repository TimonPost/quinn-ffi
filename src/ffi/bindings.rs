use crate::{
    error,
    ffi::{
        Kind,
        Out,
        QuinnError,
        QuinnResult,
        Ref,
    },
    proto::{
        Chunk,
        DatagramEvent,
        Dir,
        Endpoint,
        EndpointConfig,
        ReadError,
        StreamId,
    },
    proto_impl::{
        ConnectionInner,
        EndpointInner,
        EndpointPoller,
        IpAddr,
        QuinnErrorKind,
    },
    ConnectionHandle,
    EndpointHandle,
    RustlsClientConfigHandle,
    RustlsServerConfigHandle,
};
use bytes::BytesMut;
use libc::size_t;
use quinn_proto::{
    VarInt,
    VarIntBoundsExceeded,
};
use std::{
    io::Write,
    net::SocketAddr,
    sync::{
        mpsc,
        Arc,
        Mutex,
    },
    thread,
    time::{
        Duration,
        Instant,
    },
};
use Into;

/// ===== Endpoint API'S ======

#[no_mangle]
pub extern "cdecl" fn create_server_endpoint(
    handle: RustlsServerConfigHandle,
    mut out_endpoint_id: Out<u8>,
    mut out_endpoint_handle: Out<EndpointHandle>,
) -> QuinnResult {
    let endpoint_config = Arc::new(EndpointConfig::default());
    let server_config = handle.clone();

    let endpoint = Endpoint::new(endpoint_config, Some(Arc::from(server_config)));
    let endpoint = EndpointInner::new(endpoint);
    let endpoint_id = endpoint.id;
    let endpoint = Arc::new(Mutex::new(endpoint));

    let endpoint_handle = EndpointHandle::alloc(endpoint.clone());

    let (poller, poll_notifier) = EndpointPoller::new(endpoint.clone());
    poller.start_polling();

    let mut endpoint_lock = endpoint_handle.lock().unwrap();
    endpoint_lock.set_poll_notifier(poll_notifier);
    drop(endpoint_lock);

    unsafe {
        out_endpoint_id.init(endpoint_id);
        out_endpoint_handle.init(endpoint_handle)
    }

    QuinnResult::ok()
}

#[no_mangle]
pub extern "cdecl" fn create_client_endpoint(
    handle: RustlsClientConfigHandle,
    mut endpoint_id: Out<u8>,
    mut endpoint_handle: Out<EndpointHandle>,
) -> QuinnResult {
    let endpoint_config = Arc::new(EndpointConfig::default());
    let client_config = handle.clone();

    let mut proto_endpoint = Endpoint::new(endpoint_config, None);
    let mut endpoint = EndpointInner::new(proto_endpoint);
    endpoint.set_default_client_config(client_config);

    let endpoint_identifier = endpoint.id;

    let shared_ref = Arc::new(Mutex::new(endpoint));
    let endpoint = EndpointHandle::alloc(shared_ref.clone());

    let (poller, poll_notifier) = EndpointPoller::new(shared_ref.clone());
    poller.start_polling();

    let mut endpoint_lock = shared_ref.lock().unwrap();
    endpoint_lock.set_poll_notifier(poll_notifier);

    unsafe {
        endpoint_id.init(endpoint_identifier);
        endpoint_handle.init(endpoint)
    }

    QuinnResult::ok()
}

#[no_mangle]
pub extern "cdecl" fn connect_client(
    handle: EndpointHandle,
    address: IpAddr,
    mut out_connection: Out<ConnectionHandle>,
    mut out_connection_id: Out<u32>,
) -> QuinnResult {
    let mut endpoint = handle.lock().unwrap();
    let connection = endpoint.connect(address.into(), "localhost").unwrap();

    unsafe {
        out_connection_id.init(connection.connection_handle.0 as u32);
        out_connection.init(ConnectionHandle::alloc(connection))
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

    match endpoint
        .inner
        .handle(Instant::now(), addr, None, None, BytesMut::from(slice))
    {
        Some((handle, DatagramEvent::NewConnection(conn))) => {
            let connection = endpoint.add_connection(handle, conn);

            callbacks::on_new_connection(handle.0 as u32, connection);
            callbacks::on_connection_pollable(handle.0 as u32)
        }
        Some((handle, DatagramEvent::ConnectionEvent(event))) => {
            endpoint.forward_event_to_connection(handle, event).unwrap();
            callbacks::on_connection_pollable(handle.0 as u32);
        }
        None => {
            println!("None handled");
        }
    }

    return QuinnResult::ok();
}

/// ===== Connection API'S ======

#[no_mangle]
pub extern "cdecl" fn poll_connection(mut handle: ConnectionHandle) -> QuinnResult {
    handle.poll().into()
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
pub extern "cdecl" fn accept_stream(
    mut handle: ConnectionHandle,
    stream_direction: u8,
    mut stream_id_out: Out<u64>,
) -> QuinnResult {
    let dir = dir_from_u8(stream_direction);

    if let Some(stream_id) = handle.inner.streams().accept(dir) {
        handle.mark_pollable();

        unsafe {
            stream_id_out.init(VarInt::from(stream_id).into());
        }
        QuinnResult::ok()
    } else {
        QuinnResult::err().context(QuinnError::new(0, "No stream to accept!".to_string()))
    }
}

#[no_mangle]
pub extern "cdecl" fn read_stream(
    mut handle: ConnectionHandle,
    stream_id: u64,
    message_buf: Out<u8>,
    message_buf_len: size_t,
    actual_message_len: Out<size_t>,
) -> QuinnResult {
    _read_stream(
        &mut handle,
        stream_id,
        message_buf,
        message_buf_len,
        actual_message_len,
    )
    .into()
}

fn _read_stream(
    handle: &mut ConnectionInner,
    stream_id: u64,
    mut message_buf: Out<u8>,
    message_buf_len: size_t,
    mut actual_message_len: Out<size_t>,
) -> Result<(), QuinnErrorKind> {
    let mut stream = handle.inner.recv_stream(_stream_id(stream_id)?);

    let mut result = stream.read(true)?;

    match result.next(message_buf_len) {
        Ok(Some(chunk)) => unsafe {
            let mut buffer = unsafe { message_buf.as_uninit_bytes_mut(message_buf_len) };

            let written = buffer.write(&chunk.bytes)?;

            actual_message_len.init(written);
        },
        Err(e) => {
            if result.finalize().should_transmit() {
                handle.mark_pollable();
            }
            if e == ReadError::Blocked {
                return Err(QuinnErrorKind::QuinErrorKind(Kind::BufferBlocked));
            }

            return Err(e.into());
        }
        _ => {}
    }

    if result.finalize().should_transmit() {
        handle.mark_pollable();
    }

    Ok(())
}

#[no_mangle]
pub extern "cdecl" fn write_stream(
    mut handle: ConnectionHandle,
    stream_id: u64,
    buffer: Ref<u8>,
    buf_len: size_t,
    written_bytes: Out<size_t>,
) -> QuinnResult {
    _write_stream(&mut handle, stream_id, buffer, buf_len, written_bytes).into()
}

fn _write_stream(
    handle: &mut ConnectionInner,
    stream_id: u64,
    buffer: Ref<u8>,
    buf_len: size_t,
    mut written_bytes: Out<size_t>,
) -> Result<(), QuinnErrorKind> {
    let mut stream = handle.inner.send_stream(_stream_id(stream_id)?);

    let bytes = unsafe { buffer.as_bytes(buf_len) };
    let result = stream.write(bytes)?;
    unsafe {
        written_bytes.init(result);
    }
    handle.mark_pollable();

    Ok(())
}

#[no_mangle]
pub extern "cdecl" fn open_stream(
    mut handle: ConnectionHandle,
    stream_direction: u8,
    mut opened_stream_id: Out<u64>,
) -> QuinnResult {
    let opened_stream = handle.inner.streams().open(dir_from_u8(stream_direction));

    if let Some(stream_id) = opened_stream {
        unsafe { opened_stream_id.init(_stream_id_to_u64(stream_id)) }
        QuinnResult::ok()
    } else {
        QuinnResult::from("Streams in the given direction are currently exhausted")
    }
}

fn dir_from_u8(dir: u8) -> Dir {
    if dir == 0 {
        Dir::Bi
    } else {
        Dir::Uni
    }
}

fn _stream_id_to_u64(stream_id: StreamId) -> u64 {
    VarInt::from(stream_id).into_inner()
}

fn _stream_id(stream_id: u64) -> Result<StreamId, VarIntBoundsExceeded> {
    Ok(StreamId::from(VarInt::from_u64(stream_id)?))
}

pub mod callbacks {
    use crate::{
        ffi::QuinnResult,
        proto::{
            Dir,
            StreamId,
            Transmit,
        },
        proto_impl::{
            ConnectionInner,
            IpAddr,
        },
    };
    use libc::size_t;
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
    static mut ON_CONNECTION_POLLABLE: Option<extern "C" fn(u32)> = None;

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
            ON_STREAM_READABLE.unwrap_unchecked()(
                con,
                VarInt::from(stream_id).into(),
                stream_id.dir() as u8,
            );
        }
    }

    pub(crate) fn on_stream_writable(con: u32, stream_id: StreamId) {
        unsafe {
            ON_STREAM_WRITABLE.unwrap_unchecked()(
                con,
                VarInt::from(stream_id).into(),
                stream_id.dir() as u8,
            );
        }
    }

    pub(crate) fn on_stream_finished(con: u32, stream_id: StreamId) {
        unsafe {
            ON_STREAM_FINISHED.unwrap_unchecked()(
                con,
                VarInt::from(stream_id).into(),
                stream_id.dir() as u8,
            );
        }
    }

    pub(crate) fn on_stream_stopped(con: u32, stream_id: StreamId) {
        unsafe {
            ON_STREAM_STOPPED.unwrap_unchecked()(
                con,
                VarInt::from(stream_id).into(),
                stream_id.dir() as u8,
            );
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

    pub(crate) fn on_connection_pollable(con: u32) {
        unsafe {
            ON_CONNECTION_POLLABLE.unwrap_unchecked()(con);
        }
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_new_connection(
        cb: extern "C" fn(super::ConnectionHandle, u32),
    ) -> QuinnResult {
        unsafe {
            ON_NEW_CONNECTION = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_connected(cb: extern "C" fn(u32)) -> QuinnResult {
        unsafe {
            ON_CONNECTED = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_connection_lost(cb: extern "C" fn(u32)) -> QuinnResult {
        unsafe {
            ON_CONNECTION_LOST = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_stream_writable(cb: extern "C" fn(u32, u64, u8)) -> QuinnResult {
        unsafe {
            ON_STREAM_WRITABLE = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_stream_readable(cb: extern "C" fn(u32, u64, u8)) -> QuinnResult {
        unsafe {
            ON_STREAM_READABLE = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_stream_finished(cb: extern "C" fn(u32, u64, u8)) -> QuinnResult {
        unsafe {
            ON_STREAM_FINISHED = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub extern "cdecl" fn set_on_stream_stopped(cb: extern "C" fn(u32, u64, u8)) -> QuinnResult {
        unsafe {
            ON_STREAM_STOPPED = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub(crate) fn set_on_stream_available(cb: extern "C" fn(u32, u8)) -> QuinnResult {
        unsafe {
            ON_STREAM_AVAILABLE = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub(crate) fn set_on_datagram_received(cb: extern "C" fn(u32)) -> QuinnResult {
        unsafe {
            ON_DATAGRAM_RECEIVED = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub(crate) fn set_on_stream_opened(cb: extern "C" fn(u32, u8)) -> QuinnResult {
        unsafe {
            ON_STREAM_OPENED = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub(crate) fn set_on_transmit(
        cb: extern "C" fn(u8, *const u8, size_t, *const IpAddr),
    ) -> QuinnResult {
        unsafe {
            ON_TRANSMIT = Some(cb);
        }
        QuinnResult::ok()
    }

    #[no_mangle]
    pub(crate) fn set_on_pollable_connection(cb: extern "C" fn(u32)) -> QuinnResult {
        unsafe {
            ON_CONNECTION_POLLABLE = Some(cb);
        }
        QuinnResult::ok()
    }
}
