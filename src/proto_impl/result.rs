use crate::proto::{
    ReadError,
    WriteError,
};
use std::{
    fmt,
    io,
};

use crate::ffi::FFIResultKind;
use quinn_proto::{
    ReadableError,
    VarIntBoundsExceeded,
};
use std::{
    error::Error,
    sync::mpsc::{
        RecvError,
        SendError,
        TryRecvError,
    },
};

#[doc(hidden)]
#[macro_export]
macro_rules! impl_io_error {
    ($from:path) => {
        impl From<$from> for FFIErrorKind {
            fn from(error: $from) -> Self {
                FFIErrorKind::IoError(io::Error::new(io::ErrorKind::Other, error.to_string()))
            }
        }
    };
}

/// An `Error` implementing type that can be returned in a `Result`.
#[derive(Debug)]
pub enum FFIErrorKind {
    /// A quinn error kind.
    FFIResultKind(FFIResultKind),
    /// A quinn error with error code and reason.
    QuinnError { code: u32, reason: String },
    /// FFI related error.
    FFIError,
    /// IO Error.
    IoError(io::Error),
}

impl FFIErrorKind {
    pub fn io_error(str: &str) -> FFIErrorKind {
        FFIErrorKind::IoError(io::Error::new(io::ErrorKind::Other, str))
    }
}

impl Error for FFIErrorKind {}

impl fmt::Display for FFIErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FFIErrorKind::QuinnError { code, reason } => {
                write!(f, "QuinnError Error; code: {} reason: {}", code, reason)
            }
            FFIErrorKind::FFIError => write!(f, "Error occurred in the FFI layer"),
            FFIErrorKind::IoError(err) => write!(f, "Io Error Occurred: {}", err.to_string()),
            FFIErrorKind::FFIResultKind(kind) => {
                write!(f, "Quinn error kind Occurred: {:?}", kind)
            }
        }
    }
}

// For now most protocol errors are treated as IO errors
impl_io_error!(quinn_proto::ConnectionError);
impl_io_error!(quinn_proto::TransportError);
impl_io_error!(io::Error);
impl_io_error!(TryRecvError);
impl_io_error!(RecvError);
impl_io_error!(ReadError);
impl_io_error!(WriteError);
impl_io_error!(ReadableError);
impl_io_error!(VarIntBoundsExceeded);

impl<T> From<SendError<T>> for FFIErrorKind {
    fn from(error: SendError<T>) -> Self {
        FFIErrorKind::IoError(io::Error::new(io::ErrorKind::Other, error.to_string()))
    }
}
