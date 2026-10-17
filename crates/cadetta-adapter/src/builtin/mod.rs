//! Built-in adapters shipped inside the host (adapters.md 1).

use crate::{AdapterError, Capabilities, Capability};

pub mod csv;
pub mod json;
pub mod sw_equations;

/// The capabilities a built-in adapter declares, or `None` when `name` is not built in.
///
/// Every built-in emits and reads; none drives a live tool (emitters/*.md intro).
// frob:ticket 01M4Z6JG9VWF4R7W6B96N3JQAV
pub fn capabilities(name: &str) -> Option<Capabilities> {
    match name {
        csv::ADAPTER | json::ADAPTER | sw_equations::ADAPTER => {
            Some(Capabilities::new([Capability::Emit, Capability::Read]))
        }
        _ => None,
    }
}

/// Refuses a request that needs `cap` from built-in `name` when it does not declare it.
// frob:ticket 01M4Z6JG9VWF4R7W6B96N3JQAV
pub fn require(name: &str, cap: Capability) -> Result<(), AdapterError> {
    match capabilities(name) {
        Some(caps) if caps.has(cap) => Ok(()),
        _ => {
            tracing::warn!(
                adapter = name,
                ?cap,
                "built-in adapter lacks the capability"
            );
            Err(AdapterError::Unsupported(cap))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:ticket 01M4Z6JG9VWF4R7W6B96N3JQAV
    // frob:tests crates/cadetta-adapter/src/builtin/mod.rs::require kind=unit
    #[test]
    fn builtins_emit_and_read_but_never_drive() {
        for name in [csv::ADAPTER, json::ADAPTER, sw_equations::ADAPTER] {
            assert!(require(name, Capability::Emit).is_ok(), "{name} emit");
            assert!(require(name, Capability::Read).is_ok(), "{name} read");
            assert_eq!(
                require(name, Capability::Drive),
                Err(AdapterError::Unsupported(Capability::Drive)),
                "{name} drive"
            );
        }
        assert!(capabilities("freecad").is_none());
    }
}
