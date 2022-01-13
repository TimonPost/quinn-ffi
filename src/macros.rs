/**
Wrap an FFI function.

This macro ensures all arguments satisfy `NotNull::not_null`. It's also a simple way to work
around not having a stable catch expression yet so we can handle early returns from ffi functions.
The macro doesn't support generics or argument patterns that are more complex than simple identifiers.
*/
macro_rules! ffi {
    ($(fn $name:ident ( $( $arg_ident:ident : $arg_ty:ty),* ) -> QuinnResult $body:expr)*) => {
        $(
            #[allow(unsafe_code, unused_attributes)]
            #[no_mangle]
            pub unsafe extern "cdecl" fn $name( $($arg_ident : $arg_ty),* ) -> QuinnResult {
                #[allow(unused_mut)]
                fn call( $(mut $arg_ident: $arg_ty),* ) -> QuinnResult {
                    $(
                        if $crate::ffi::IsNull::is_null(&$arg_ident) {
                            return QuinnResult::argument_null().context(QuinnError::new(0, stringify!($arg_ident).to_string()));
                        }
                    )*

                    $body
                }

                QuinnResult::catch(move || call( $($arg_ident),* ))
            }
        )*
    };
}
