use crate::proto::{
    ReadError,
    WriteError,
};
use std::{
    fmt,
    io,
};

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
use crate::ffi::{QuinnResult, Kind};

#[doc(hidden)]
#[macro_export]
macro_rules! impl_error {
    ($from:path) => {
        impl From<$from> for QuinnErrorKind {
            fn from(error: $from) -> Self {
                QuinnErrorKind::IoError(io::Error::new(io::ErrorKind::Other, error.to_string()))
            }
        }
    };
}

#[derive(Debug)]
pub enum QuinnErrorKind {
    QuinErrorKind(Kind),
    QuinnError { code: u32, reason: String },
    FFIError,
    IoError(io::Error),
}

impl Error for QuinnErrorKind {}

impl fmt::Display for QuinnErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QuinnErrorKind::QuinnError { code, reason } => {
                write!(f, "QuinnError Error; code: {} reason: {}", code, reason)
            }
            QuinnErrorKind::FFIError => write!(f, "Error occurred in the FFI layer"),
            QuinnErrorKind::IoError(err) => write!(f, "Io Error Occurred: {}", err.to_string()),
            QuinnErrorKind::QuinErrorKind(kind) => {write!(f, "Quinn error kind Occurred: {:?}", kind)}
        }
    }
}

// For now all protocol errors are treated as IO errors
impl_error!(quinn_proto::ConnectionError);
impl_error!(quinn_proto::TransportError);
impl_error!(io::Error);
impl_error!(TryRecvError);
impl_error!(RecvError);
impl_error!(ReadError);
impl_error!(WriteError);
impl_error!(ReadableError);
impl_error!(VarIntBoundsExceeded);


impl<T> From<SendError<T>> for QuinnErrorKind {
    fn from(error: SendError<T>) -> Self {
        QuinnErrorKind::IoError(io::Error::new(io::ErrorKind::Other, error.to_string()))
    }
}
