#![feature(option_result_unwrap_unchecked)]
#![feature(box_into_inner)]

pub use quinn_proto as proto;

#[macro_use]
mod macros;
pub mod ffi;
pub mod proto_impl;

pub use handles::*;

#[cfg(feature = "unwind-safe")]
mod handles {
    use crate::{
        ffi::{
            Handle,
            HandleSync,
        },
        proto_impl::{
            ConnectionInner,
            EndpointInner,
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
    // Mutex required for unwind safeness due to possible interior mutability.
    pub type EndpointHandle<'a> = HandleSync<'a, Arc<Mutex<EndpointInner>>>;
    // Mutex required for unwind safeness due to possible interior mutability.
    pub type ConnectionHandle<'a> = HandleSync<'a, Mutex<ConnectionInner>>;

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
        type Inner = EndpointInner;

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
            let mut lock = self.lock().unwrap();

            cb(&mut lock)
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
        type Inner = ConnectionInner;

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
            if let Err(e) = self.try_lock() {
                println!("lock: {:?}", e);
            }

            let mut lock = self.lock().unwrap();

            let a = cb(&mut lock);
            drop(lock);
            a
        }

        fn new(instance: Self::Inner) -> Self {
            Self::alloc(Mutex::new(instance))
        }
    }
}

#[cfg(feature = "safe")]
mod handles {
    // Mutex required for unwind safeness due to possible interior mutability.
    pub type RustlsClientConfigHandle<'a> = HandleExclusive<'a, Mutex<quinn_proto::ClientConfig>>;
    // Mutex required for unwind safeness due to possible interior mutability.
    pub type RustlsServerConfigHandle<'a> = HandleSync<'a, Mutex<quinn_proto::ServerConfig>>;
    pub type EndpointHandle<'a> = HandleSync<'a, Arc<Mutex<EndpointInner>>>;
    pub type ConnectionHandle<'a> = HandleExclusive<'a, ConnectionInner>;
}
