use std::{
    cell::RefCell,
    error::Error,
    ffi::CString,
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

impl<T, E: Error> From<Result<T, E>> for QuinnResult {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(_) => QuinnResult::ok(),
            Err(e) => QuinnResult::err().context(QuinnError::new(0, e.to_string())),
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
