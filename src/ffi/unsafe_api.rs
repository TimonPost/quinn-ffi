//! This module doesnt validate if pointers are null, and does not catch panics.
//! Its unsafe but might be faster since there is less overhead for FFI calls.
use crate::{
    ffi::HandleMut,
    proto_impl::{
        ConnectionImpl,
        EndpointImpl,
        FFIErrorKind,
    },
};

use crate::ffi::HandleRef;
use std::sync::{
    Arc,
    Mutex,
};

// Mutex required for unwind safeness due to possible interior mutability.
pub type RustlsClientConfigHandle<'a> = HandleMut<'a, Mutex<quinn_proto::ClientConfig>>;
// Mutex required for unwind safeness due to possible interior mutability.
pub type RustlsServerConfigHandle<'a> = HandleMut<'a, Mutex<quinn_proto::ServerConfig>>;
// Mutex required for unwind safeness due to possible interior mutability.
pub type EndpointHandle<'a> = HandleMut<'a, Arc<Mutex<EndpointImpl>>>;
// Mutex required for unwind safeness due to possible interior mutability.
pub type ConnectionHandle<'a> = HandleMut<'a, Arc<Mutex<ConnectionImpl>>>;

impl<'a> HandleMut for RustlsClientConfigHandle<'a> {
    type Inner = quinn_proto::ClientConfig;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        let lock = &self.lock().unwrap();
        cb(lock)
    }

    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        let mut lock = self.lock().unwrap();

        cb(&mut lock)
    }

    fn new(instance: Self::Inner) -> Self {
        Self::alloc(Mutex::new(instance))
    }
}

impl<'a> HandleMut for EndpointHandle<'a> {
    type Inner = EndpointImpl;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        let mut lock = self.lock().unwrap();

        cb(&mut lock)
    }

    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        let mut lock = self.lock().unwrap();

        cb(&mut lock)
    }

    fn new(instance: Self::Inner) -> Self {
        Self::alloc(Arc::new(Mutex::new(instance)))
    }
}

impl<'a> HandleMut for RustlsServerConfigHandle<'a> {
    type Inner = quinn_proto::ServerConfig;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        let lock = &self.lock().unwrap();

        cb(lock)
    }

    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        let mut lock = self.lock().unwrap();

        cb(&mut lock)
    }

    fn new(instance: Self::Inner) -> Self {
        Self::alloc(Mutex::new(instance))
    }
}

impl<'a> HandleMut for ConnectionHandle<'a> {
    type Inner = ConnectionImpl;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        let lock = &self.lock().unwrap();

        cb(lock)
    }

    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        let mut lock = self.lock().unwrap();

        let a = cb(&mut lock);
        drop(lock);
        a
    }

    fn new(instance: Self::Inner) -> Self {
        Self::alloc(Arc::new(Mutex::new(instance)))
    }
}

/**
Wrap an FFI function.

This macro doesnt implement `NotNull::not_null` checks and doesnt catches any panics.
*/
macro_rules! ffi {
    ($(fn $name:ident ( $( $arg_ident:ident : $arg_ty:ty),* ) -> FFIResult $body:expr)*) => {
        $(
            #[allow(unsafe_code, unused_attributes)]
            #[no_mangle]
            pub unsafe extern "cdecl" fn $name( $(mut $arg_ident : $arg_ty),* ) -> FFIResult {
                $body
            }
        )*
    };
}
