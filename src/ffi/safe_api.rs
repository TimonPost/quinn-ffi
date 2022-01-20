//! This module contains FFI handels and the ffi function generator macro.
//
//! This API validates if pointers are null and catches panics.
//! It also protects handle access with a mutex.
//! It is more safe then the unsafe api however it introduces some extra logic to keep this safe which could come at a little performance cost.

use crate::{
    ffi::{
        Handle,
        HandleSync,
    },
    proto_impl::{
        ConnectionImpl,
        EndpointImpl,
        QuinnErrorKind,
    },
};

use std::sync::{
    Arc,
    Mutex,
};

// Mutex required for unwind safeness due to possible interior mutability.
pub type RustlsClientConfigHandle<'a> = HandleSync<'a, Mutex<quinn_proto::ClientConfig>>;
// Mutex required for unwind safeness due to possible interior mutability.
pub type RustlsServerConfigHandle<'a> = HandleSync<'a, Mutex<quinn_proto::ServerConfig>>;
// Mutex require d for unwind safeness due to possible interior mutability.
pub type EndpointHandle<'a> = HandleSync<'a, Arc<Mutex<EndpointImpl>>>;
// Mutex required for unwind safeness due to possible interior mutability.
pub type ConnectionHandle<'a> = HandleSync<'a, Arc<Mutex<ConnectionImpl>>>;

impl<'a> Handle for RustlsClientConfigHandle<'a> {
    type Inner = quinn_proto::ClientConfig;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind> {
        let lock = &self.lock().unwrap();
        cb(lock)
    }

    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind> {
        let mut lock = self.lock().unwrap();

        cb(&mut lock)
    }

    fn new(instance: Self::Inner) -> Self {
        Self::alloc(Mutex::new(instance))
    }
}

impl<'a> Handle for EndpointHandle<'a> {
    type Inner = EndpointImpl;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind> {
        let mut lock = self.lock().unwrap();

        cb(&mut lock)
    }

    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind> {
        //println!(" ++ endpoint lock");
        let mut lock = self.lock().unwrap();
        let a = cb(&mut lock);
        //println!(" ++ end endpoint lock");
        drop(lock);
        a
    }

    fn new(instance: Self::Inner) -> Self {
        Self::alloc(Arc::new(Mutex::new(instance)))
    }
}

impl<'a> Handle for RustlsServerConfigHandle<'a> {
    type Inner = quinn_proto::ServerConfig;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind> {
        let lock = &self.lock().unwrap();

        cb(lock)
    }

    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind> {
        let mut lock = self.lock().unwrap();

        cb(&mut lock)
    }

    fn new(instance: Self::Inner) -> Self {
        Self::alloc(Mutex::new(instance))
    }
}

impl<'a> Handle for ConnectionHandle<'a> {
    type Inner = ConnectionImpl;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind> {
        let lock = &self.lock().unwrap();

        cb(lock)
    }

    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind> {
        //println!("\t++ connection lock");
        let mut lock = self.lock().unwrap();
        let a = cb(&mut lock);
        drop(lock);
        //println!("\t-- end connection lock");
        a
    }

    fn new(instance: Self::Inner) -> Self {
        Self::alloc(Arc::new(Mutex::new(instance)))
    }
}

/**
Wrap an FFI function.

This macro ensures all arguments satisfy `NotNull::not_null`. It's also a simple way to work
around not having a stable catch expression yet so we can handle early returns from ffi functions.
The macro doesn't support generics or argument patterns that are more complex than simple identifiers.
*/

macro_rules! ffi {
    (
        $(  $(#[$meta:meta])* fn $name:ident ( $( $arg_ident:ident : $arg_ty:ty),* ) -> FFIResult $body:expr)*
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
                            return FFIResult::argument_null().context(QuinnError::new(0, stringify!($arg_ident).to_string()));
                        }
                    )*

                    $body
                }

                FFIResult::catch(move || call( $($arg_ident),* ))
            }
        )*
    };
}
