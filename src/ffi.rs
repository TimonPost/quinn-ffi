pub mod bindings;
mod client_config;
mod deferred_cleanup;
mod handle_exclusive;
mod handle_shared;
mod handle_sync;
mod null;
mod out;
mod quinn_result;
mod reference;
mod thread_bound;
mod transport_config;

use crate::proto_impl::QuinnErrorKind;
pub use deferred_cleanup::DeferredCleanup;
pub use handle_exclusive::HandleExclusive;
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

pub trait Handle {
    type Inner;

    fn ref_access(
        &self,
        cb: &mut dyn FnMut(&Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind>;
    fn mut_access(
        &mut self,
        cb: &mut dyn FnMut(&mut Self::Inner) -> Result<(), QuinnErrorKind>,
    ) -> Result<(), QuinnErrorKind>;
    fn new(instance: Self::Inner) -> Self;
}
