//! FFI layer that exposes functions and types for interacting with Quinn.

#[cfg(feature = "unsafe-api")]
#[macro_use]
pub mod unsafe_api;
#[cfg(feature = "safe-api")]
#[macro_use]
pub mod safe_api;

#[cfg(feature = "safe-api")]
pub use safe_api::*;
#[cfg(feature = "unsafe-api")]
pub use unsafe_api::*;

mod bindings;
mod ffi_result;
mod handle_mut;
mod handle_shared;
mod null;
mod out;
mod reference;

pub use null::IsNull;
pub use out::Out;

pub use ffi_result::{
    FFIResult,
    FFIResultKind,
    QuinnError,
};

pub use reference::{
    Ref,
    RefMut,
};

pub use bindings::{
    accept_stream,
    connect_client,
    create_client_config,
    create_client_endpoint,
    create_server_config,
    create_server_endpoint,
    handle_datagram,
    last_error,
    open_stream,
    poll_connection,
    read_stream,
    write_stream,
};

pub use bindings::callbacks;

use crate::proto_impl::FFIErrorKind;

/// A handle defines a type that is shared across the FFi boundary.
pub trait HandleMut {
    type Inner;

    /// Returns a new allocated instance of the handle.
    fn new(instance: Self::Inner) -> Self;

    /// Access the immutable inner handle value.
    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind>;

    /// Access the mutable inner handle value.
    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind>;
}

/// A handle defines a type that is shared across the FFi boundary.
pub trait HandleRef {
    type Inner;

    /// Returns a new allocated instance of the handle.
    fn new(instance: Self::Inner) -> Self;

    /// Access the immutable inner handle value.
    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind>;
}
