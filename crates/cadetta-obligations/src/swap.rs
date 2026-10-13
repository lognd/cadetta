//! frob:ticket 01M4SWN7JR7PMBQ4QEHBZ6BN9J
//! The `swap` lock record written by `cadetta swap` (obligations.md 3).

use crate::lock::{Date, Lock, Record, Swap};

/// Why a swap record cannot be built.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SwapRecordError {
    /// The reason is empty or blank; a swap must say why (obligations.md 3).
    #[error("a swap needs a non-empty `--reason`")]
    EmptyReason,
    /// The reason spans several lines, which a one-line record cannot hold.
    #[error("the swap reason must be a single line")]
    MultilineReason,
    /// The date is not `YYYY-MM-DD`.
    #[error("{0}")]
    BadDate(String),
}

/// Build the `swap` record for `slot` going `from` to `to`, by `by` on `at`, for `reason`.
pub fn record(
    slot: &str,
    from: &str,
    to: &str,
    by: &str,
    at: &str,
    reason: &str,
) -> Result<Record, SwapRecordError> {
    let reason = reason.trim();
    if reason.is_empty() {
        tracing::warn!(slot, "swap record refused: empty reason");
        return Err(SwapRecordError::EmptyReason);
    }
    if reason.contains(['\n', '\r']) {
        tracing::warn!(slot, "swap record refused: multi-line reason");
        return Err(SwapRecordError::MultilineReason);
    }
    let at = Date::new(at).map_err(SwapRecordError::BadDate)?;
    tracing::debug!(slot, from, to, by, "swap record built");
    Ok(Record::Swap(Swap {
        slot: slot.to_owned(),
        from: from.to_owned(),
        to: to.to_owned(),
        by: by.to_owned(),
        at,
        reason: reason.to_owned(),
    }))
}

/// Append a swap record to `lock`; returns whether it was stored (it is, unless identical).
pub fn append(lock: &mut Lock, rec: Record) -> bool {
    let stored = lock.insert(rec);
    tracing::info!(stored, "swap record appended to the lock");
    stored
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests crates/cadetta-obligations/src/swap.rs::record
    #[test]
    fn builds_and_renders_a_swap_line() {
        let rec = record("a::R/wheel", "A", "B", "x@y.z", "2026-10-13", " cheaper ").unwrap();
        let mut lock = Lock::new();
        assert!(append(&mut lock, rec));
        let text = lock.render();
        assert!(text.contains("swap a::R/wheel from=A to=B by=x@y.z at=2026-10-13"));
        assert!(text.contains("reason=\"cheaper\""));
        assert_eq!(Lock::parse(&text).unwrap(), lock);
    }

    // frob:tests crates/cadetta-obligations/src/swap.rs::record
    #[test]
    fn refuses_bad_reason_and_date() {
        assert_eq!(
            record("s", "A", "B", "u", "2026-10-13", "  "),
            Err(SwapRecordError::EmptyReason)
        );
        assert_eq!(
            record("s", "A", "B", "u", "2026-10-13", "a\nb"),
            Err(SwapRecordError::MultilineReason)
        );
        assert!(matches!(
            record("s", "A", "B", "u", "13/10/26", "why"),
            Err(SwapRecordError::BadDate(_))
        ));
    }
}
