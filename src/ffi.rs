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

pub mod bindings;
mod handle_shared;
mod handle_sync;
mod null;
mod out;
mod quinn_result;
mod reference;
mod transport_config;

pub use handle_shared::HandleShared;
pub use handle_sync::HandleSync;
pub use null::IsNull;
pub use out::Out;
pub use quinn_result::{
    Kind,
    QuinnError,
    QuinnResult,
};
pub use reference::{
    Ref,
    RefMut,
};

use crate::proto_impl::QuinnErrorKind;

/// A handle defines a type that is shared across the FFi boundary.
pub trait Handle {
    type Inner;

    /// Returns a new allocated instance of the handle.
    fn new(instance: Self::Inner) -> Self;

    /// Access the immutable inner handle value.
    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind>;

    /// Access the mutable inner handle value.
    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind>;
}
