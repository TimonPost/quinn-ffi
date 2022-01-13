use crate::proto_impl::QuinnErrorKind;

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

#[derive(Debug)]
pub struct LastResult {
    err: Option<QuinnError>,
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuinnResult {
    pub kind: Kind,
}

impl QuinnResult {
    pub fn new(kind: Kind) -> QuinnResult {
        QuinnResult { kind }
    }

    pub fn ok() -> Self {
        QuinnResult::new(Kind::Ok)
    }

    pub fn err() -> Self {
        QuinnResult::new(Kind::Error)
    }

    pub fn buffer_too_small() -> Self {
        QuinnResult::new(Kind::BufferToSmall)
    }

    pub fn buffer_blocked() -> Self {
        QuinnResult::new(Kind::BufferBlocked)
    }

    pub fn argument_null() -> Self {
        QuinnResult::new(Kind::ArgumentNull)
    }

    pub fn is_err(&self) -> bool {
        self.kind != Kind::Ok
    }
    pub fn context(self, e: QuinnError) -> Self {
        LAST_RESULT.with(|last_result| {
            let result = LastResult { err: Some(e) };
            *last_result.borrow_mut() = Some(result);
        });

        self
    }

    pub fn with_last_result<R>(f: impl FnOnce(Option<&QuinnError>) -> R) -> R {
        LAST_RESULT.with(|last_result| {
            let last_result = last_result.borrow();

            let mut message: Option<&QuinnError> = None;

            if let Some(last) = last_result.as_ref() {
                if let Some(error) = last.err.as_ref() {
                    message = Some(error);
                }
            }

            return f(message);
        })
    }

    pub(super) fn catch(f: impl FnOnce() -> Self + UnwindSafe) -> Self {
        //println!("before last result");
        let r = LAST_RESULT.with(|last_result| {
            //println!("before last result");
            {
                *last_result.borrow_mut() = None;
            }
            //println!("after borrow last result");
            return match catch_unwind(f) {
                Ok(result) => {
                    if result.is_err() {
                        let error = QuinnError::new(0, result.to_string());

                        // Always set the last result so it matches what's returned.
                        // This `Ok` branch doesn't necessarily mean the result is ok,
                        // only that there wasn't a panic.
                        let mut ref_mut = last_result.borrow_mut();

                        ref_mut.as_mut().map(|a| {
                            *a = LastResult { err: Some(error) };
                        });
                        println!("result");
                        return result;
                    }

                    QuinnResult::ok()
                }
                Err(e) => {
                    println!("err");
                    let extract_panic =
                        || extract_panic(&e).map(|s| format!("internal panic with '{}'", s));

                    // Set the last error to the panic message if it's not already set
                    let mut ref_mut = last_result.borrow_mut();

                    ref_mut.as_mut().map(|a| {
                        *a = LastResult {
                            err: Some(QuinnError::new(0, extract_panic().unwrap())),
                        };
                    });

                    QuinnResult::err()
                }
            };
        });
        //println!("after last result: {:?}", r);
        r
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

impl Display for QuinnResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self.kind {
            Kind::Ok => write!(f, "Successful")?,
            Kind::Error => write!(f, "Some error occurred")?,
            Kind::BufferToSmall => write!(f, "The supplied buffer was to small.")?,
            Kind::BufferBlocked => write!(f, "There is no data in the buffer to be read.")?,
            Kind::ArgumentNull => write!(f, "An argument was null.")?,
        }
        Ok(())
    }
}

impl<T> From<Result<T, QuinnErrorKind>> for QuinnResult {
    fn from(result: Result<T, QuinnErrorKind>) -> Self {
        match result {
            Ok(_kind) => QuinnResult::ok(),
            Err(e) => match e {
                QuinnErrorKind::QuinErrorKind(kind) => match kind {
                    Kind::Ok => QuinnResult::ok(),
                    Kind::Error => QuinnResult::err(),
                    Kind::BufferToSmall => QuinnResult::buffer_too_small(),
                    Kind::BufferBlocked => QuinnResult::buffer_blocked(),
                    Kind::ArgumentNull => QuinnResult::argument_null(),
                },
                e => QuinnResult::err().context(QuinnError::new(0, e.to_string())),
            },
        }
    }
}

impl From<&str> for QuinnResult {
    fn from(result: &str) -> Self {
        QuinnResult::err().context(QuinnError::new(0, result.to_string()))
    }
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Ok,
    Error,
    BufferToSmall,
    BufferBlocked,
    ArgumentNull,
}

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
