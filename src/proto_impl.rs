//! Quinn-proto implementation, similar to `quinn` but without the async runtime and some differences to make it fit with FFi applications.

pub use addr::IpAddr;
pub use connection::{
    ConnectionEvent,
    ConnectionImpl,
};
pub use endpoint::{
    EndpointEvent,
    EndpointImpl,
    EndpointPoller,
};
pub use result::FFIErrorKind;

mod addr;
mod connection;
mod endpoint;
mod result;
