use crate::ffi::IsNull;
use std::{
    marker::PhantomData,
    panic::{
        RefUnwindSafe,
        UnwindSafe,
    },
    slice,
};

/// An initialized parameter passed by shared reference.
#[repr(transparent)]
pub struct Ref<'a, T>(*const T, PhantomData<&'a T>)
where
    T: ?Sized + Send;

impl<'a, T> UnwindSafe for Ref<'a, T> where T: ?Sized + RefUnwindSafe + Send {}

/// The handle is semantically `&T`, dont use generics with internal mutability!
unsafe impl<'a, T> Sync for Ref<'a, T> where T: Send {}

impl<'a, T> Ref<'a, T>
where
    T: ?Sized + Send,
{
    // The pointer must be nonnull and will remain valid
    pub unsafe fn as_ref(&self) -> &T {
        &*self.0
    }
}

impl<'a> Ref<'a, u8> {
    // The pointer must be nonnull, the length is correct, and will remain valid
    pub unsafe fn as_bytes(&self, len: usize) -> &[u8] {
        slice::from_raw_parts(self.0, len)
    }
}

/// An initialized parameter passed by exclusive reference.
#[repr(transparent)]
pub struct RefMut<'a, T>(*mut T, PhantomData<&'a mut T>)
where
    T: ?Sized + Send + Sync;

impl<'a, T> UnwindSafe for RefMut<'a, T> where T: ?Sized + RefUnwindSafe + Send + Sync {}

impl<'a, T: ?Sized + Send + Sync> RefMut<'a, T> {
    // The pointer must be nonnull and will remain valid
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0 }
    }
}

impl<'a> RefMut<'a, u8> {
    // The pointer must be nonnull, the length is correct, and will remain valid
    pub fn as_bytes_mut(&mut self, len: usize) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.0, len) }
    }
}

impl<'a, T: ?Sized + Send + Sync> IsNull for Ref<'a, T> {
    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl<'a, T: ?Sized + Send + Sync> IsNull for RefMut<'a, T> {
    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}
