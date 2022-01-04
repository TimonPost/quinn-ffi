#![feature(option_result_unwrap_unchecked)]
#![feature(box_into_inner)]

use crate::ffi::{HandleExclusive, QuinnError, QuinnResult, HandleSync};
use std::{
    sync::Mutex,
};

use crate::{
    proto_impl::{
        ConnectionInner,
        EndpointInner,
    },
};
pub use quinn_proto as proto;
use std::ffi::CString;

mod ffi;
mod proto_impl;

fn error() -> QuinnResult {
    let error = QuinnError {
        code: u64::MAX,
        reason: CString::new("this is an error").unwrap(),
    };
    QuinnResult::err().context(error)
}

pub type RustlsServerConfigHandle<'a> = HandleExclusive<'a, quinn_proto::ServerConfig>;
pub type EndpointHandle<'a> = HandleSync<'a, Mutex<EndpointInner>>;
pub type ConnectionHandle<'a> = HandleExclusive<'a, ConnectionInner>;

