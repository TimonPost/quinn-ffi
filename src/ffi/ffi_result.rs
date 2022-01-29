use crate::proto_impl::FFIErrorKind;

use std::{
    any::Any,
    cell::RefCell,
    ffi::CString,
    fmt,
    fmt::{
        Display,
        Formatter,
    },
    panic::{
        catch_unwind,
        UnwindSafe,
    },
};

thread_local!(
    static LAST_RESULT: RefCell<Option<LastResult>> = RefCell::new(None);
);

/// The last `QuinnError`.
#[derive(Debug)]
pub struct LastResult {
    err: Option<FFIErrorKind>,
}

/// FFI safe result type.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FFIResult {
    // FFi result only contains a enum kind.
    pub kind: FFIResultKind,
}

impl FFIResult {
    pub fn new(kind: FFIResultKind) -> FFIResult {
        FFIResult { kind }
    }

    /// Result is successful.
    pub fn ok() -> Self {
        FFIResult::new(FFIResultKind::Ok)
    }

    /// Result is erroneous.
    pub fn err() -> Self {
        FFIResult::new(FFIResultKind::Error)
    }

    pub fn buffer_too_small() -> Self {
        FFIResult::new(FFIResultKind::BufferToSmall)
    }

    pub fn buffer_blocked() -> Self {
        FFIResult::new(FFIResultKind::BufferBlocked)
    }

    pub fn argument_null() -> Self {
        FFIResult::new(FFIResultKind::ArgumentNull)
    }

    pub fn is_err(&self) -> bool {
        self.kind != FFIResultKind::Ok
    }

    /// Sets the `LAST_RESULT` context to the given `FFIErrorKind`.
    pub fn context(self, e: FFIErrorKind) -> Self {
        tracing::error!("with context {:?}", e);
        LAST_RESULT.with(|last_result| {
            let result = LastResult { err: Some(e) };
            last_result.replace(Some(result));
        });

        self
    }

    /// Creates result from `LAST_RESULT`.
    pub fn from_last_result<R>(f: impl FnOnce(Option<&FFIErrorKind>) -> R) -> R {
        LAST_RESULT.with(|last_result| {
            let last_result = last_result.borrow();

            let mut message: Option<&FFIErrorKind> = None;

            if let Some(last) = last_result.as_ref() {
                if let Some(error) = last.err.as_ref() {
                    message = Some(error);
                }
            }

            tracing::error!("NONE {:?}", last_result);

            return f(message);
        })
    }

    /// Calls a function catching any panic and on panic sets the `LAST_RESULT`.
    pub(super) fn catch(f: impl FnOnce() -> Self + UnwindSafe) -> Self {
        LAST_RESULT.with(|last_result| {
            {
                *last_result.borrow_mut() = None;
            }
            return match catch_unwind(f) {
                Ok(result) => result,
                Err(e) => {
                    let extract_panic =
                        || extract_panic(&e).map(|s| format!("internal panic with '{}'", s));

                    // Set the last error to the panic message if it's not already set
                    let mut ref_mut = last_result.borrow_mut();

                    ref_mut.as_mut().map(|a| {
                        *a = LastResult {
                            err: Some(FFIErrorKind::io_error(&extract_panic().unwrap())),
                        };
                    });

                    FFIResult::err()
                }
            };
        })
    }
}

fn extract_panic(err: &Box<dyn Any + Send + 'static>) -> Option<String> {
    if let Some(err) = err.downcast_ref::<String>() {
        Some(err.clone())
    } else if let Some(err) = err.downcast_ref::<&'static str>() {
        Some((*err).to_owned())
    } else {
        None
    }
}

impl Display for FFIResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self.kind {
            FFIResultKind::Ok => write!(f, "Successful")?,
            FFIResultKind::Error => write!(f, "Some error occurred")?,
            FFIResultKind::BufferToSmall => write!(f, "The supplied buffer was to small.")?,
            FFIResultKind::BufferBlocked => {
                write!(f, "There is no data in the buffer to be read.")?
            }
            FFIResultKind::ArgumentNull => write!(f, "An argument was null.")?,
        }
        Ok(())
    }
}

impl<T> From<Result<T, FFIErrorKind>> for FFIResult {
    fn from(result: Result<T, FFIErrorKind>) -> Self {
        match result {
            Ok(_kind) => FFIResult::ok(),
            Err(e) => match e {
                FFIErrorKind::FFIResultKind(kind) => match kind {
                    FFIResultKind::Ok => FFIResult::ok(),
                    FFIResultKind::Error => FFIResult::err(),
                    FFIResultKind::BufferToSmall => FFIResult::buffer_too_small(),
                    FFIResultKind::BufferBlocked => FFIResult::buffer_blocked(),
                    FFIResultKind::ArgumentNull => FFIResult::argument_null(),
                },
                e => FFIResult::err().context(e),
            },
        }
    }
}

impl From<&str> for FFIResult {
    fn from(result: &str) -> Self {
        FFIResult::err().context(FFIErrorKind::io_error(result))
    }
}

/// Indicating a certain result kind.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FFIResultKind {
    /// Result is successful
    Ok,
    /// Result is erroneous
    Error,
    /// Buffer is to small, resize and try again.
    BufferToSmall,
    /// Buffer is blocked meaning it doesnt contain data.
    BufferBlocked,
    /// A argument to the FFI function was not initialized.
    ArgumentNull,
}

/// Error with code and reason.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuinnError {
    pub code: u64,
    pub reason: CString,
}

impl QuinnError {
    pub fn new(code: u64, reason: String) -> QuinnError {
        QuinnError {
            code,
            reason: CString::new(reason).unwrap(),
        }
    }
}
