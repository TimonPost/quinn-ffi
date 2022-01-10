use std::{
    cell::RefCell,
    ffi::CString,
};
use crate::proto_impl::QuinnErrorKind;

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
}

impl<T> From<Result<T, QuinnErrorKind>> for QuinnResult {
    fn from(result: Result<T, QuinnErrorKind>) -> Self {
        match result {
            Ok(kind) => QuinnResult::ok(),
            Err(e) => {
                match e   {
                    QuinnErrorKind::QuinErrorKind(kind) => {
                        match kind {
                            Kind::Ok => QuinnResult::ok(),
                            Kind::Error => QuinnResult::err(),
                            Kind::BufferToSmall => QuinnResult::buffer_too_small(),
                            Kind::BufferBlocked => QuinnResult::buffer_blocked()
                        }
                    }
                    e =>  QuinnResult::err().context(QuinnError::new(0, e.to_string())),
                }
            }
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
    BufferBlocked
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
