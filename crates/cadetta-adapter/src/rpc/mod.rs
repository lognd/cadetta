//! JSON-RPC 2.0 stdio host for external adapters (adapters.md 5).
//!
//! Framing: one JSON message per line (newline-delimited) on the adapter's
//! stdin and stdout. The adapter's stdout is the protocol and nothing else;
//! its logs go to stderr, which the host passes through.

pub mod client;
pub mod types;

pub use client::{Client, ProcessClient, RpcError};
pub use types::*;
