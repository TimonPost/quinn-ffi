#![feature(option_result_unwrap_unchecked)]
#![feature(box_into_inner)]

pub use quinn_proto as proto;

#[macro_use]
pub mod ffi;
pub mod proto_impl;
