use crate::ffi::IsNull;
use std::{
    marker::PhantomData,
    ops::Deref,
    panic::{
        RefUnwindSafe,
        UnwindSafe,
    },
};

/// A handle that can be read-accessed concurrently by multiple threads.
///
/// The interior value can be treated like `&T`.
#[repr(transparent)]
pub struct FFIHandleRef<'a, T>(*const T, PhantomData<&'a T>)
where
    T: ?Sized + Send + Sync;

impl<'a, T> UnwindSafe for FFIHandleRef<'a, T> where T: ?Sized + RefUnwindSafe + Send + Sync {}

impl<'a, T> FFIHandleRef<'a, T>
where
    T: Send + Sync,
{
    /// Allocates and initializes memory for the passed type.
    pub fn alloc(value: T) -> Self
    where
        T: 'static,
    {
        let v = Box::new(value);

        FFIHandleRef(Box::into_raw(v), PhantomData)
    }
}

impl<'a, T> FFIHandleRef<'a, T>
where
    T: Send + Sync,
{
    /// Deallocates and initializes memory for the passed type.
    ///
    /// There are no other live references and the handle won't be used again
    pub unsafe fn dealloc<R>(handle: Self, f: impl FnOnce(T) -> R) -> R {
        let v = Box::from_raw(handle.0 as *mut T);
        f(*v)
    }
}

impl<'a, T> Deref for FFIHandleRef<'a, T>
where
    T: ?Sized + Send + Sync,
{
    type Target = T;

    // We own the interior value
    fn deref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

impl<'a, T> IsNull for FFIHandleRef<'a, T>
where
    T: ?Sized + Send + Sync,
{
    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}
