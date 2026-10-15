//! Adapter host: built-in emitters and the JSON-RPC client for external adapters.
//!
//! Specification: `docs/spec/adapters.md`.

pub mod atomic;
pub mod availability;
pub mod bindings;
pub mod builtin;
pub mod classify;
pub mod geometry_bind;
pub mod import;
pub mod locator;
pub mod policy;
pub mod read_facts;
pub mod read_units;
pub mod rpc;
pub mod scope;
pub mod session;
pub mod sync;
pub mod testing;

use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;

/// Diagnostic code for a rebuild failure after a push (adapters.md 1).
pub const R004: &str = "R004";

/// One of the three adapter capabilities (adapters.md 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    /// IR to file.
    Emit,
    /// File to observed IR.
    Read,
    /// IR to and from a live tool; implies rebuild-failure reporting.
    Drive,
}

/// The set of capabilities an adapter declares at `initialize`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Capabilities(BTreeSet<Capability>);

impl Capabilities {
    /// Builds a set from the given capabilities.
    pub fn new(caps: impl IntoIterator<Item = Capability>) -> Self {
        Self(caps.into_iter().collect())
    }

    /// Reports whether `cap` is declared.
    pub fn has(&self, cap: Capability) -> bool {
        self.0.contains(&cap)
    }
}

/// Diagnostic severity in an adapter result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Informational.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

/// A `cadetta.diag/1`-shaped diagnostic produced by an adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterDiagnostic {
    /// Rule code such as `R004`.
    pub code: String,
    /// Severity of the finding.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
}

impl AdapterDiagnostic {
    /// Builds an R004 rebuild-failure diagnostic for `target`.
    pub fn rebuild_failure(target: &str, detail: &str) -> Self {
        tracing::warn!(target, detail, "rebuild failed (R004)");
        Self {
            code: R004.to_string(),
            severity: Severity::Error,
            message: format!("rebuild failed for {target}: {detail}"),
        }
    }
}

/// Error type of every adapter operation; carries diagnostics, never panics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    /// The adapter does not declare the capability the operation needs.
    Unsupported(Capability),
    /// The operation failed; the diagnostics explain why.
    Failed(Vec<AdapterDiagnostic>),
}

impl AdapterError {
    /// Wraps a single diagnostic as a failure.
    pub fn failed(diag: AdapterDiagnostic) -> Self {
        Self::Failed(vec![diag])
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(c) => write!(f, "adapter does not support {c:?}"),
            Self::Failed(ds) => {
                let msgs: Vec<&str> = ds.iter().map(|d| d.message.as_str()).collect();
                write!(f, "adapter failed: {}", msgs.join("; "))
            }
        }
    }
}

impl std::error::Error for AdapterError {}

/// Result alias for adapter operations.
pub type AdapterResult<T> = Result<T, AdapterError>;

/// Files written by an operation, with diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Paths written.
    pub files: Vec<PathBuf>,
    /// Non-fatal diagnostics.
    pub diagnostics: Vec<AdapterDiagnostic>,
}

/// What `sync` did for an adapter (adapters.md 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncPlan {
    /// Full loop: drive the live tool.
    Drive,
    /// No drive: emit, then tell the user the manual step.
    EmitWithInstructions {
        /// The one manual step (relink or reload).
        instructions: String,
    },
}

/// An adapter to one CAD tool or file format; `emit`, `read` and `drive` are optional.
pub trait Adapter {
    /// Adapter name.
    fn name(&self) -> &str;

    /// Declared capabilities.
    fn capabilities(&self) -> Capabilities;

    /// Writes the parameter file(s) for the design; requires `emit`.
    fn emit(&mut self) -> AdapterResult<Outcome>;

    /// Pushes values into the live tool and regenerates; requires `drive`.
    ///
    /// Rebuild failures return `AdapterError::Failed` with R004 diagnostics.
    fn drive(&mut self) -> AdapterResult<Outcome> {
        Err(AdapterError::Unsupported(Capability::Drive))
    }

    /// Instructions for the manual step when `drive` is absent.
    fn manual_step(&self) -> String {
        format!("reload the files emitted by {}", self.name())
    }
}

/// Chooses the sync strategy: drive if declared, else emit plus instructions.
pub fn plan_sync(adapter: &dyn Adapter) -> AdapterResult<SyncPlan> {
    let caps = adapter.capabilities();
    if caps.has(Capability::Drive) {
        tracing::debug!(adapter = adapter.name(), "sync via drive");
        Ok(SyncPlan::Drive)
    } else if caps.has(Capability::Emit) {
        tracing::info!(adapter = adapter.name(), "no drive: degrading sync to emit");
        Ok(SyncPlan::EmitWithInstructions {
            instructions: adapter.manual_step(),
        })
    } else {
        tracing::error!(adapter = adapter.name(), "no emit or drive for sync");
        Err(AdapterError::Unsupported(Capability::Emit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        caps: Capabilities,
        fail_rebuild: bool,
    }

    impl Adapter for Fake {
        fn name(&self) -> &str {
            "fake"
        }
        fn capabilities(&self) -> Capabilities {
            self.caps.clone()
        }
        fn emit(&mut self) -> AdapterResult<Outcome> {
            Ok(Outcome::default())
        }
        fn drive(&mut self) -> AdapterResult<Outcome> {
            if self.fail_rebuild {
                Err(AdapterError::failed(AdapterDiagnostic::rebuild_failure(
                    "Part1",
                    "bad feature",
                )))
            } else {
                Ok(Outcome::default())
            }
        }
    }

    #[test]
    fn sync_without_drive_degrades_to_emit() {
        let a = Fake {
            caps: Capabilities::new([Capability::Emit]),
            fail_rebuild: false,
        };
        assert!(matches!(
            plan_sync(&a),
            Ok(SyncPlan::EmitWithInstructions { .. })
        ));
    }

    #[test]
    fn failing_result_is_err_with_diagnostic() {
        let a = Fake {
            caps: Capabilities::new([Capability::Read]),
            fail_rebuild: false,
        };
        let r = plan_sync(&a);
        assert_eq!(r, Err(AdapterError::Unsupported(Capability::Emit)));
        struct NoDrive;
        impl Adapter for NoDrive {
            fn name(&self) -> &str {
                "nodrive"
            }
            fn capabilities(&self) -> Capabilities {
                Capabilities::default()
            }
            fn emit(&mut self) -> AdapterResult<Outcome> {
                Ok(Outcome::default())
            }
        }
        assert_eq!(
            NoDrive.drive(),
            Err(AdapterError::Unsupported(Capability::Drive))
        );
    }

    #[test]
    fn drive_rebuild_error_is_r004() {
        let mut a = Fake {
            caps: Capabilities::new([Capability::Drive]),
            fail_rebuild: true,
        };
        assert_eq!(plan_sync(&a), Ok(SyncPlan::Drive));
        match a.drive() {
            Err(AdapterError::Failed(ds)) => assert_eq!(ds[0].code, "R004"),
            other => panic!("unexpected {other:?}"),
        }
    }
}
