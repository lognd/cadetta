//! Contract test harness: proves an adapter meets adapters.md 4 rules 2 to 5.
//!
//! Adapter authors call the `check_*` functions from their own tests. Each
//! returns `Err(ContractViolation)` naming the rule that failed.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::read_units::ObservedValue;
use crate::{Adapter, AdapterError, AdapterResult, Capability};

/// Diagnostic code for a value not representable at the declared precision.
pub const Q004: &str = "Q004";

/// Observed values keyed by parameter path.
pub type Observed = BTreeMap<String, ObservedValue>;

/// An adapter that can also read its own output back as observed IR.
pub trait ReadAdapter: Adapter {
    /// Reads `files` into observed values keyed by parameter path.
    fn read(&mut self, files: &[PathBuf]) -> AdapterResult<Observed>;
}

/// A contract rule an adapter broke, with a human-readable detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractViolation {
    /// Rule number in adapters.md 4.
    pub rule: u8,
    /// What went wrong.
    pub detail: String,
}

impl fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "adapters.md 4 rule {}: {}", self.rule, self.detail)
    }
}

impl std::error::Error for ContractViolation {}

/// Result alias for contract checks.
pub type ContractResult = Result<(), ContractViolation>;

fn violation(rule: u8, detail: impl Into<String>) -> ContractViolation {
    let detail = detail.into();
    tracing::error!(rule, detail = detail.as_str(), "contract violation");
    ContractViolation { rule, detail }
}

type Snapshot = Vec<(PathBuf, Vec<u8>)>;

fn snapshot(files: &[PathBuf]) -> Result<Snapshot, ContractViolation> {
    files
        .iter()
        .map(|p| {
            fs::read(p)
                .map(|b| (p.clone(), b))
                .map_err(|e| violation(2, format!("cannot read emitted {}: {e}", p.display())))
        })
        .collect()
}

fn emit_snapshot(adapter: &mut dyn Adapter) -> Result<Snapshot, ContractViolation> {
    let out = adapter
        .emit()
        .map_err(|e| violation(2, format!("emit failed: {e}")))?;
    snapshot(&out.files)
}

/// Rule 2: emitting twice yields the same files with identical bytes.
pub fn check_deterministic(adapter: &mut dyn Adapter) -> ContractResult {
    let a = emit_snapshot(adapter)?;
    let b = emit_snapshot(adapter)?;
    if a == b {
        tracing::debug!(adapter = adapter.name(), "emit is deterministic");
        Ok(())
    } else {
        Err(violation(2, "two emits produced different bytes or files"))
    }
}

/// Rule 2: emit then read gives values equal to `expected` (literal, unit, exactness).
pub fn check_round_trip(adapter: &mut dyn ReadAdapter, expected: &Observed) -> ContractResult {
    if !adapter.capabilities().has(Capability::Read) {
        return Err(violation(2, "round trip requires the read capability"));
    }
    let out = adapter
        .emit()
        .map_err(|e| violation(2, format!("emit failed: {e}")))?;
    let got = adapter
        .read(&out.files)
        .map_err(|e| violation(2, format!("read failed: {e}")))?;
    if &got == expected {
        Ok(())
    } else {
        Err(violation(
            2,
            format!("round trip mismatch: expected {expected:?}, got {got:?}"),
        ))
    }
}

/// Rule 3: an unrepresentable value makes emit fail with Q004, never silent rounding.
pub fn check_q004_on_unrepresentable(adapter: &mut dyn Adapter) -> ContractResult {
    match adapter.emit() {
        Err(AdapterError::Failed(ds)) if ds.iter().any(|d| d.code == Q004) => Ok(()),
        Err(e) => Err(violation(3, format!("expected Q004, got error: {e}"))),
        Ok(_) => Err(violation(3, "emit succeeded; value was silently rounded")),
    }
}

/// Rule 4: `path` reads back inexact with a reason; no inexact value lacks a reason.
pub fn check_inexact_has_reason(
    adapter: &mut dyn ReadAdapter,
    files: &[PathBuf],
    path: &str,
) -> ContractResult {
    let got = adapter
        .read(files)
        .map_err(|e| violation(4, format!("read failed: {e}")))?;
    if let Some((p, _)) = got
        .iter()
        .find(|(_, v)| !v.exact && v.reason.as_deref().is_none_or(str::is_empty))
    {
        return Err(violation(4, format!("{p} is inexact without a reason")));
    }
    match got.get(path) {
        Some(v) if !v.exact => Ok(()),
        Some(_) => Err(violation(
            4,
            format!("{path} reported exact; expected exact:false"),
        )),
        None => Err(violation(4, format!("{path} missing from observed IR"))),
    }
}

fn cdt_snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "cdt")
                && let Ok(b) = fs::read(&p)
            {
                out.insert(p, b);
            }
        }
    }
    out
}

/// Rule 5: running emit leaves every `.cdt` file under `source_root` untouched and writes none.
pub fn check_no_source_writes(adapter: &mut dyn Adapter, source_root: &Path) -> ContractResult {
    let before = cdt_snapshot(source_root);
    let out = adapter
        .emit()
        .map_err(|e| violation(5, format!("emit failed: {e}")))?;
    if let Some(p) = out
        .files
        .iter()
        .find(|p| p.extension().is_some_and(|e| e == "cdt"))
    {
        return Err(violation(5, format!("emitted into source {}", p.display())));
    }
    if cdt_snapshot(source_root) == before {
        Ok(())
    } else {
        Err(violation(5, ".cdt source changed during emit"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AdapterDiagnostic, Capabilities, Outcome, Severity};

    #[derive(Clone, Copy, PartialEq)]
    enum Mode {
        Good,
        Nondeterministic,
        Rounds,
        WritesSource,
        Guesses,
    }

    struct Fake {
        dir: PathBuf,
        mode: Mode,
        n: u32,
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cadetta-testing-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    impl Adapter for Fake {
        fn name(&self) -> &str {
            "fake"
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::new([Capability::Emit, Capability::Read])
        }
        fn emit(&mut self) -> AdapterResult<Outcome> {
            self.n += 1;
            if self.mode == Mode::Rounds {
                return Err(AdapterError::failed(AdapterDiagnostic {
                    code: Q004.into(),
                    severity: Severity::Error,
                    message: "not representable".into(),
                }));
            }
            let name = if self.mode == Mode::WritesSource {
                "part.cdt"
            } else {
                "part.txt"
            };
            let body = if self.mode == Mode::Nondeterministic {
                format!("d=12.5 {}", self.n)
            } else {
                "d=12.5".to_string()
            };
            let p = self.dir.join(name);
            fs::write(&p, body).unwrap();
            Ok(Outcome {
                files: vec![p],
                diagnostics: vec![],
            })
        }
    }

    impl ReadAdapter for Fake {
        fn read(&mut self, _files: &[PathBuf]) -> AdapterResult<Observed> {
            let exact = self.mode != Mode::Guesses;
            let mut m = Observed::new();
            m.insert("d".into(), obs(exact, None));
            Ok(m)
        }
    }

    fn obs(exact: bool, reason: Option<&str>) -> ObservedValue {
        ObservedValue {
            literal: "12.5".into(),
            unit: Some("mm".into()),
            unit_assumed: false,
            exact,
            reason: reason.map(String::from),
        }
    }

    fn fake(name: &str, mode: Mode) -> Fake {
        Fake {
            dir: tmp(name),
            mode,
            n: 0,
        }
    }

    #[test]
    fn deterministic_emit_passes_and_drift_fails() {
        assert!(check_deterministic(&mut fake("det", Mode::Good)).is_ok());
        let v = check_deterministic(&mut fake("nondet", Mode::Nondeterministic)).unwrap_err();
        assert_eq!(v.rule, 2);
    }

    #[test]
    fn round_trip_compares_values() {
        let mut want = Observed::new();
        want.insert("d".into(), obs(true, None));
        assert!(check_round_trip(&mut fake("rt", Mode::Good), &want).is_ok());
        want.insert("extra".into(), obs(true, None));
        assert!(check_round_trip(&mut fake("rt2", Mode::Good), &want).is_err());
    }

    #[test]
    fn q004_required_not_silent_rounding() {
        assert!(check_q004_on_unrepresentable(&mut fake("q", Mode::Rounds)).is_ok());
        let v = check_q004_on_unrepresentable(&mut fake("q2", Mode::Good)).unwrap_err();
        assert_eq!(v.rule, 3);
    }

    #[test]
    fn inexact_read_needs_reason() {
        // Inexact without a reason is a violation.
        let v = check_inexact_has_reason(&mut fake("ix", Mode::Guesses), &[], "d").unwrap_err();
        assert_eq!(v.rule, 4);
        // Exact value where inexact was expected is a violation.
        assert!(check_inexact_has_reason(&mut fake("ix2", Mode::Good), &[], "d").is_err());
    }

    #[test]
    fn inexact_with_reason_passes() {
        struct R;
        impl Adapter for R {
            fn name(&self) -> &str {
                "r"
            }
            fn capabilities(&self) -> Capabilities {
                Capabilities::new([Capability::Read])
            }
            fn emit(&mut self) -> AdapterResult<Outcome> {
                Err(AdapterError::Unsupported(Capability::Emit))
            }
        }
        impl ReadAdapter for R {
            fn read(&mut self, _: &[PathBuf]) -> AdapterResult<Observed> {
                let mut m = Observed::new();
                m.insert("d".into(), obs(false, Some("driven dimension")));
                Ok(m)
            }
        }
        assert!(check_inexact_has_reason(&mut R, &[], "d").is_ok());
    }

    #[test]
    fn source_is_never_written() {
        let mut a = fake("src", Mode::Good);
        let root = a.dir.clone();
        fs::write(root.join("design.cdt"), "x").unwrap();
        assert!(check_no_source_writes(&mut a, &root).is_ok());
        let mut b = fake("src2", Mode::WritesSource);
        let root = b.dir.clone();
        let v = check_no_source_writes(&mut b, &root).unwrap_err();
        assert_eq!(v.rule, 5);
    }
}
