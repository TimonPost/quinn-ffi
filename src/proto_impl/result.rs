use std::{error, fmt, io};
use crate::proto::{ConnectionError, ReadError};
use crate::ffi::{QuinnError, QuinnResult};
use std::sync::mpsc::{SendError, RecvError, TryRecvError};
use quinn_proto::ReadableError;
use std::error::Error;

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
    QuinnError {code: u32, reason: String},
    FFIError,
    IoError(io::Error),
}

impl Error for QuinnErrorKind { }

impl fmt::Display for QuinnErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QuinnErrorKind::QuinnError{code,reason} => write!(f, "QuinnError Error; code: {} reason: {}", code, reason),
            QuinnErrorKind::FFIError => write!(f, "Error occurred in the FFI layer"),
            QuinnErrorKind::IoError(err) => write!(f, "Error Occurred: {}", err.to_string()),
        }
    }
}

impl_error!(quinn_proto::ConnectionError);
impl_error!(quinn_proto::TransportError);
impl_error!(io::Error);
impl_error!(TryRecvError);
impl_error!(RecvError);
impl_error!(ReadError);
impl_error!(ReadableError);

impl<T> From<SendError<T>> for QuinnErrorKind {
    fn from(error: SendError<T>) -> Self {
        QuinnErrorKind::IoError(io::Error::new(io::ErrorKind::Other, error.to_string()))
    }
}
