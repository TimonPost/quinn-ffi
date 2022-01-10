#![feature(option_result_unwrap_unchecked)]
#![feature(box_into_inner)]

use crate::ffi::{
    HandleExclusive,
    HandleSync,
    QuinnError,
    QuinnResult,
};
use std::sync::{Mutex, Arc};

use crate::proto_impl::{
    ConnectionInner,
    EndpointInner,
};
pub use quinn_proto as proto;
use std::ffi::CString;

mod ffi;
mod proto_impl;

#[no_mangle]
pub extern "cdecl" fn add(a: u32, b: u32) -> u32{
    return a + b;
}

fn error() -> QuinnResult {
    let error = QuinnError {
        code: u64::MAX,
        reason: CString::new("this is an error").unwrap(),
    };
    QuinnResult::err().context(error)
}

pub type RustlsClientConfigHandle<'a> = HandleExclusive<'a, quinn_proto::ClientConfig>;
pub type RustlsServerConfigHandle<'a> = HandleExclusive<'a, quinn_proto::ServerConfig>;
pub type EndpointHandle<'a> = HandleSync<'a, Arc<Mutex<EndpointInner>>>;
pub type ConnectionHandle<'a> = HandleExclusive<'a, ConnectionInner>;
