//! The only place cadetta-log reads the process environment.

use crate::init::{InitError, LOG_ENV};

/// Reads `CADETTA_LOG`: `Ok(None)` when unset, an error when not valid unicode.
pub(crate) fn log_spec() -> Result<Option<String>, InitError> {
    match std::env::var_os(LOG_ENV).map(std::ffi::OsString::into_string) {
        None => Ok(None),
        Some(Ok(s)) => Ok(Some(s)),
        Some(Err(_)) => Err(InitError::NotUnicode),
    }
}
