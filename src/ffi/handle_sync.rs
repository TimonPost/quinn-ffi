use crate::ffi::IsNull;
use std::{
    marker::PhantomData,
    ops::{
        Deref,
        DerefMut,
    },
    panic::{
        RefUnwindSafe,
        UnwindSafe,
    },
};

/// A shared handle that can be read/write-accessed concurrently by multiple threads.
///
/// Can only contain types that are `Sync` + `Send` semantically.
#[repr(transparent)]
pub struct HandleSync<'a, T>(*mut T, PhantomData<&'a T>)
where
    T: ?Sized + Send + Sync;

impl<'a, T> UnwindSafe for HandleSync<'a, T> where T: ?Sized + Send + Sync + RefUnwindSafe {}

// The handle is semantically `&T`
unsafe impl<'a, T> Send for HandleSync<'a, T> where T: ?Sized + Send + Sync {}

// The handle is semantically `&T`
unsafe impl<'a, T> Sync for HandleSync<'a, T> where T: ?Sized + Send + Sync {}

impl<'a, T> HandleSync<'a, T>
where
    T: Send + Sync,
{
    /// Allocates and initializes memory for the passed type.
    pub fn alloc(value: T) -> Self {
        HandleSync(Box::into_raw(Box::new(value)), PhantomData)
    }

    /// Deallocates and initializes memory for the passed type.
    ///
    /// There are no other live references and the handle won't be used again
    pub unsafe fn dealloc<R>(handle: Self, f: impl FnOnce(T) -> R) -> R {
        let v = Box::into_inner(Box::from_raw(handle.0));
        f(v)
    }
}

impl<'a, T> Deref for HandleSync<'a, T>
where
    T: ?Sized + Send + Sync,
{
    type Target = T;

    fn deref(&self) -> &T {
        // We own the interior value
        unsafe { &*self.0 }
    }
}

impl<'a, T> DerefMut for HandleSync<'a, T>
where
    T: ?Sized + Send + Sync,
{
    fn deref_mut(&mut self) -> &mut T {
        // We own the interior value
        unsafe { &mut *self.0 }
    }
}

impl<'a, T> IsNull for HandleSync<'a, T>
where
    T: ?Sized + Send + Sync,
{
    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}
