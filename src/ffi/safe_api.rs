//! This module contains FFI handels and the ffi function generator macro.
//
//! This API validates if pointers are null and catches panics.
//! It also protects handle access with a mutex.
//! It is more safe then the unsafe api however it introduces some extra logic to keep this safe which could come at a little performance cost.

use crate::proto_impl::{
    ConnectionImpl,
    EndpointImpl,
    FFIErrorKind,
};

use crate::ffi::{
    handle_mut::FFIHandleMut,
    HandleMut,
    HandleRef,
};
use std::sync::{
    Arc,
    Mutex,
};

// Mutex required for unwind safeness due to possible interior mutability.
pub type RustlsClientConfigHandle<'a> = FFIHandleMut<'a, Mutex<quinn_proto::ClientConfig>>;
// Mutex required for unwind safeness due to possible interior mutability.
pub type RustlsServerConfigHandle<'a> = FFIHandleMut<'a, Mutex<quinn_proto::ServerConfig>>;
// Mutex require d for unwind safeness due to possible interior mutability.
pub type EndpointHandle<'a> = FFIHandleMut<'a, Arc<Mutex<EndpointImpl>>>;
// Mutex required for unwind safeness due to possible interior mutability.
pub type ConnectionHandle<'a> = FFIHandleMut<'a, Arc<Mutex<ConnectionImpl>>>;

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

impl<'a> HandleMut for RustlsServerConfigHandle<'a> {
    type Inner = quinn_proto::ServerConfig;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        let lock = self.lock().unwrap();
        cb(&lock)
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
        cb(&lock)
    }

    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        //println!(" ++ endpoint lock");
        let mut lock = self.lock().unwrap();
        let a = cb(&mut lock);
        //println!(" ++ end endpoint lock");
        a
    }

    fn new(instance: Self::Inner) -> Self {
        Self::alloc(Arc::new(Mutex::new(instance)))
    }
}

impl<'a> HandleMut for ConnectionHandle<'a> {
    type Inner = ConnectionImpl;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        let lock = self.lock().unwrap();
        cb(&lock)
    }

    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), FFIErrorKind>,
    ) -> Result<(), FFIErrorKind> {
        //println!("\t++ connection lock");
        let mut lock = self.lock().unwrap();
        let a = cb(&mut lock);
        //println!("\t-- end connection lock");
        a
    }

    fn new(instance: Self::Inner) -> Self {
        Self::alloc(Arc::new(Mutex::new(instance)))
    }
}
use tracing::warn;

/**
Wrap an FFI function.

This macro ensures all arguments satisfy `NotNull::not_null`. It's also a simple way to work
around not having a stable catch expression yet so we can handle early returns from ffi functions.
The macro doesn't support generics or argument patterns that are more complex than simple identifiers.
*/
macro_rules! ffi {
    (
        $(
            $(#[$meta:meta])*
            fn $name:ident ( $( $arg_ident:ident : $arg_ty:ty),* ) -> FFIResult $body:expr)*
    ) => {
        $(
            $(#[$meta])*
            #[allow(unsafe_code, unused_attributes)]
            #[no_mangle]
            pub unsafe extern "cdecl" fn $name( $($arg_ident : $arg_ty),* ) -> FFIResult {
                tracing::trace!("FFI invoke: {:?}", stringify!($name));

                #[allow(unused_mut)]
                fn call( $(mut $arg_ident: $arg_ty),* ) -> FFIResult {
                    $(
                        if $crate::ffi::IsNull::is_null(&$arg_ident) {
                            return FFIResult::argument_null().context(FFIErrorKind::io_error(&stringify!($arg_ident)));
                        }
                    )*

                    $body
                }

                FFIResult::catch(move || call( $($arg_ident),* ))
            }
        )*
    };
}
