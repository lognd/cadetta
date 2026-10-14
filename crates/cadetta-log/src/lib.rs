//! frob:ticket 01M4SY4XVHM745JWB285NFC7G7
//! Logging and redaction shared by every cadetta crate.
//!
//! This is the one crate that owns tracing subscriber setup: products
//! install one subscriber via [`init`] and never write to stdout or stderr
//! with print macros; anything on stderr goes through the layers built
//! here. `CADETTA_LOG` is the one documented diagnostic environment
//! variable; it takes `tracing` env-filter directives such as
//! `cadetta_units=debug,warn`. [`redact`] masks secret-shaped strings
//! before output is stored or logged.

mod env;
mod init;
mod redact;
mod sink;
#[cfg(any(test, feature = "test-util"))]
pub mod testing;

pub use init::{InitError, LOG_ENV, init};
pub use redact::{REDACTED, redact};
pub use sink::{Sink, init_sink};
