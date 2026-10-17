//! Host handling of R008, R009 and R010 (adapters.md 2.3, 4; config-cli.md 3).
//!
//! * R008 adapter-unavailable: an error under `sync` (exit 2), info under `check`,
//!   which then uses the lock base as observed.
//! * R009 file-busy: a dirty or read-only document is refused per file before any
//!   push; the refused file is never handed to the adapter.
//! * R010 push-rejected: reported per file and per locator; other locators proceed.
//!
//! Pure functions over protocol types; no I/O.

use std::collections::BTreeMap;

use crate::rpc::{FileState, LocatorStatus, PushStatus, R008, RpcError};
use crate::session::R009;
use crate::{AdapterDiagnostic, Severity};

/// Diagnostic code for a push the adapter rejected (adapters.md 2.3).
// frob:ticket 01M4SWMCARH2B2RDJV076RVDH2
pub const R010: &str = "R010";

/// Process exit code for an unavailable adapter under `sync` (config-cli.md 3).
// frob:ticket 01M4SWMCARH2B2RDJV076RVDH2
pub const EXIT_UNAVAILABLE: u8 = 2;

/// Which verb is asking whether the adapter is usable.
// frob:ticket 01M4SWMCARH2B2RDJV076RVDH2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// `sync`: an unavailable adapter is fatal.
    Sync,
    /// `check`: an unavailable adapter is informational.
    Check,
}

/// The host's reaction to an unavailable adapter.
// frob:ticket 01M4SWMCARH2B2RDJV076RVDH2
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unavailable {
    /// The R008 diagnostic.
    pub diagnostic: AdapterDiagnostic,
    /// Exit code the run must end with; `None` when the run continues.
    pub exit: Option<u8>,
    /// `check` only: treat the lock base as the observed value (drift unverified).
    pub use_lock_base: bool,
}

/// Maps a failure to reach the adapter to R008, or `None` when it is another failure.
///
/// Spawn failures, version or platform mismatches and nonconforming results are R008; `sync` makes it an error with exit 2,
/// `check` an info that falls back to the lock base.
// frob:ticket 01M4SWMCARH2B2RDJV076RVDH2
pub fn unavailable(adapter: &str, err: &RpcError, verb: Verb) -> Option<Unavailable> {
    let detail = match err {
        RpcError::Spawn(_)
        | RpcError::VersionMismatch { .. }
        | RpcError::PlatformMismatch { .. }
        | RpcError::Nonconforming(_) => err.to_string(),
        _ => return None,
    };
    let (severity, exit, use_lock_base) = match verb {
        Verb::Sync => (Severity::Error, Some(EXIT_UNAVAILABLE), false),
        Verb::Check => (Severity::Info, None, true),
    };
    match verb {
        Verb::Sync => tracing::error!(adapter, %detail, "adapter unavailable (R008)"),
        Verb::Check => {
            tracing::info!(adapter, %detail, "adapter unavailable (R008), using lock base")
        }
    }
    Some(Unavailable {
        diagnostic: AdapterDiagnostic {
            code: R008.to_string(),
            severity,
            message: format!("adapter `{adapter}` is unavailable: {detail}"),
        },
        exit,
        use_lock_base,
    })
}

/// A document refused before any push (R009).
// frob:ticket 01M4SWMCARH2B2RDJV076RVDH2
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Busy {
    /// The refused file.
    pub file: String,
    /// Why: `dirty` or `read-only`.
    pub reason: &'static str,
    /// The R009 diagnostic.
    pub diagnostic: AdapterDiagnostic,
}

/// Splits opened files into those safe to push into and those refused (R009).
///
/// A refused file is reported individually and must not be pushed to, saved or closed
/// by the host on its own account; the other files proceed.
// frob:ticket 01M4SWMCARH2B2RDJV076RVDH2
pub fn partition_busy(states: &[FileState]) -> (Vec<String>, Vec<Busy>) {
    let mut ready = Vec::new();
    let mut busy = Vec::new();
    for f in states {
        let reason = if f.dirty {
            "dirty"
        } else if f.readonly {
            "read-only"
        } else {
            ready.push(f.path.clone());
            continue;
        };
        tracing::warn!(file = %f.path, reason, "file busy, not touched (R009)");
        busy.push(Busy {
            file: f.path.clone(),
            reason,
            diagnostic: AdapterDiagnostic {
                code: R009.to_string(),
                severity: Severity::Error,
                message: format!("{}: file is {reason}; refused before any push", f.path),
            },
        });
    }
    (ready, busy)
}

/// R010 for one file: the locators the adapter rejected, the rest having proceeded.
// frob:ticket 01M4SWMCARH2B2RDJV076RVDH2
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    /// The file pushed into.
    pub file: String,
    /// Rejected locators of that file.
    pub locators: Vec<String>,
    /// The R010 diagnostic (one per file).
    pub diagnostic: AdapterDiagnostic,
}

/// Groups `Rejected` push statuses into one R010 per file, in file order.
///
/// `pushed` is `(file, statuses)` as returned per file by a drive sync; files with no
/// rejected locator yield nothing.
// frob:ticket 01M4SWMCARH2B2RDJV076RVDH2
pub fn rejections(pushed: &[(String, Vec<LocatorStatus>)]) -> Vec<Rejected> {
    let mut by_file: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (file, statuses) in pushed {
        for s in statuses.iter().filter(|s| s.status == PushStatus::Rejected) {
            by_file.entry(file).or_default().push(s.locator.clone());
        }
    }
    by_file
        .into_iter()
        .map(|(file, locators)| {
            tracing::warn!(file, rejected = locators.len(), "push rejected (R010)");
            let message = format!("{file}: push rejected for {}", locators.join(", "));
            Rejected {
                file: file.to_owned(),
                locators,
                diagnostic: AdapterDiagnostic {
                    code: R010.to_string(),
                    severity: Severity::Error,
                    message,
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(path: &str, dirty: bool, readonly: bool) -> FileState {
        FileState {
            path: path.to_owned(),
            dirty,
            readonly,
        }
    }

    fn status(locator: &str, status: PushStatus) -> LocatorStatus {
        LocatorStatus {
            locator: locator.to_owned(),
            status,
            old: None,
            new: None,
            tolerance: None,
        }
    }

    fn mismatch() -> RpcError {
        RpcError::VersionMismatch {
            expected: "1.0".into(),
            found: "2.0".into(),
        }
    }

    // frob:tests crates/cadetta-adapter/src/availability.rs::unavailable kind=unit
    #[test]
    fn r008_unavailable_adapter_is_error_exit_2_on_sync() {
        let u = unavailable("sw", &mismatch(), Verb::Sync).unwrap();
        assert_eq!(u.diagnostic.code, "R008");
        assert_eq!(u.diagnostic.severity, Severity::Error);
        assert_eq!(u.exit, Some(2));
        assert!(!u.use_lock_base);
    }

    // frob:tests crates/cadetta-adapter/src/availability.rs::unavailable kind=unit
    #[test]
    fn r008_unavailable_adapter_is_info_on_check_with_lock_base() {
        let err = RpcError::Spawn(std::io::Error::other("not found"));
        let u = unavailable("sw", &err, Verb::Check).unwrap();
        assert_eq!(u.diagnostic.severity, Severity::Info);
        assert_eq!(u.exit, None);
        assert!(u.use_lock_base);
        assert!(unavailable("sw", &RpcError::Unsupported("x"), Verb::Sync).is_none());
    }

    // frob:tests crates/cadetta-adapter/src/availability.rs::partition_busy kind=unit
    #[test]
    fn r009_busy_file_is_refused_and_not_touched() {
        let states = [
            state("a.sldprt", true, false),
            state("b.sldprt", false, true),
            state("c.sldprt", false, false),
        ];
        let (ready, busy) = partition_busy(&states);
        assert_eq!(ready, ["c.sldprt"]);
        assert_eq!(busy.len(), 2);
        assert_eq!(busy[0].reason, "dirty");
        assert_eq!(busy[1].reason, "read-only");
        assert!(busy.iter().all(|b| b.diagnostic.code == "R009"));
    }

    // frob:tests crates/cadetta-adapter/src/availability.rs::rejections kind=unit
    #[test]
    fn r010_rejection_is_reported_per_file() {
        let pushed = vec![
            (
                "a".to_owned(),
                vec![
                    status("D1", PushStatus::Rejected),
                    status("D2", PushStatus::Pushed),
                    status("D3", PushStatus::Rejected),
                ],
            ),
            ("b".to_owned(), vec![status("D1", PushStatus::Pushed)]),
            ("c".to_owned(), vec![status("D9", PushStatus::Rejected)]),
        ];
        let r = rejections(&pushed);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].file, "a");
        assert_eq!(r[0].locators, ["D1", "D3"]);
        assert_eq!(r[1].file, "c");
        assert!(r.iter().all(|x| x.diagnostic.code == "R010"));
    }
}
