use crate::proto_impl::IpAddr;

macro_rules! never_null {
    ($($t:ty),*) => {
        $(
            impl IsNull for $t {
                fn is_null(&self) -> bool {
                    false
                }
            }
        )*
    }
}

/**
Whether or not a value passed across an FFI boundary is null.
*/
pub trait IsNull {
    fn is_null(&self) -> bool;
}

impl<T: ?Sized> IsNull for *const T {
    fn is_null(&self) -> bool {
        <*const T>::is_null(*self)
    }
}

impl<T: ?Sized> IsNull for *mut T {
    fn is_null(&self) -> bool {
        <*mut T>::is_null(*self)
    }
}

never_null!(usize, isize, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, bool, IpAddr);
