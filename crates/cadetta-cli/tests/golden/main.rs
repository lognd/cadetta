//! frob:ticket 01M4SXSGQQHR6RMBNJQJ3392AZ
//! Golden corpus runner (tests/golden/README.md).
//!
//! For each `tests/golden/NN-name/` case the runner copies the case to a scratch directory, runs
//! `cadetta check --format json` (diagnostics) and `cadetta ir` (values and verdicts: `check`
//! reports neither), then every `[[artifact]]` command, and compares with `expected.toml`.
//!
//! Runner rules (to be copied into tests/golden/README.md, see the follow-up ticket):
//! - Values compare as exact quantities in any unit; `min`/`max` are nominal plus the lowest and
//!   highest tolerance deviations. Verdicts map holds/violates/undecided to pass/fail/unknown.
//! - Diagnostics are closed for live (non-waived) errors and warnings; info only when listed.
//! - Artifacts compare byte for byte after each `mask` match becomes `<masked>` in both files.
//! - Expectations the implementation cannot yet produce are "unevaluable" (not mismatches).
//! - A mismatch names its suspect: the implementation when the run is not clean (the IR fails
//!   or `check` has live errors), the hand evaluation when the run is clean. A path written
//!   `A/b` that the IR holds as `A.b` is found by alias (sema bug ~137W4EE).
//! - `status = "confirmed"`: any mismatch or unevaluable expectation fails the test (regression).
//! - `status = "hand-evaluated"`: findings are reported only; they fail under
//!   `CADETTA_GOLDEN_STRICT=1`.
//! - A case whose `spec` differs from the current draft (`Status: draft X.Y` in
//!   docs/spec/README.md) always fails until ported.
//! - `CADETTA_GOLDEN_CONFIRM=1` writes real digests over `blake3:TBD` files and sets
//!   `status = "confirmed"` for hand-evaluated, current-spec cases with no mismatch and no
//!   unevaluable expectation. Never part of a normal run.
//! - The per-case summary goes to stderr and `$CARGO_TARGET_TMPDIR/golden-report.txt`.

mod actual;
mod compare;
mod confirm;
mod expect;
mod quantity;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use actual::{Scratch, copy_dir, observe};
use compare::{Outcome, compare};

fn golden_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

/// The current spec draft, read from docs/spec/README.md (`Status: draft X.Y`).
fn current_spec() -> Result<String, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec/README.md");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.lines()
        .find_map(|l| l.split("Status: draft ").nth(1))
        .and_then(|r| r.split(|c: char| !(c.is_ascii_digit() || c == '.')).next())
        .map(str::to_owned)
        .ok_or_else(|| "no `Status: draft X.Y` line in docs/spec/README.md".into())
}

/// Case directories (`NN-name`) sorted by name.
fn cases(root: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("expected.toml").is_file())
        .collect();
    v.sort();
    v
}

/// A case written against another spec draft than the current one is stale and fails until ported.
fn is_stale(case_spec: &str, current: &str) -> bool {
    case_spec != current
}

/// One case's verdict on the test.
struct CaseResult {
    summary: String,
    fails: bool,
    outcome: Option<Outcome>,
}

fn run_case(dir: &Path, current: &str, strict: bool) -> CaseResult {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let expected = match expect::load(dir) {
        Ok(e) => e,
        Err(e) => {
            return CaseResult {
                summary: format!("{name}: FAIL {e}\n"),
                fails: true,
                outcome: None,
            };
        }
    };
    let stale = is_stale(&expected.case.spec, current);
    let scratch = match Scratch::new().and_then(|s| copy_dir(dir, &s.0).map(|()| s)) {
        Ok(s) => s,
        Err(e) => {
            return CaseResult {
                summary: format!("{name}: FAIL {e}\n"),
                fails: true,
                outcome: None,
            };
        }
    };
    let actual = observe(&scratch.0);
    let outcome = compare(&expected, &actual, &scratch.0, dir);
    let confirmed = expected.case.status == "confirmed";
    let findings = outcome.mismatches() + outcome.unevaluable() > 0;
    let fails = stale || (findings && (confirmed || strict));
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{name}: status={} spec={} (current {current}){} clean={} aliased={} \
         values m/x/u={}/{}/{} verdicts={}/{}/{} diagnostics={}/{}/{} artifacts={}/{}/{} \
         suspect impl={} hand={}{}",
        expected.case.status,
        expected.case.spec,
        if stale { " STALE" } else { "" },
        actual.clean(),
        outcome.aliased,
        outcome.values.matched,
        outcome.values.mismatched,
        outcome.values.unevaluable,
        outcome.verdicts.matched,
        outcome.verdicts.mismatched,
        outcome.verdicts.unevaluable,
        outcome.diagnostics.matched,
        outcome.diagnostics.mismatched,
        outcome.diagnostics.unevaluable,
        outcome.artifacts.matched,
        outcome.artifacts.mismatched,
        outcome.artifacts.unevaluable,
        outcome.impl_suspect,
        outcome.hand_suspect,
        if fails { " FAIL" } else { "" },
    );
    if let Err(e) = &actual.ir {
        let _ = writeln!(s, "  ir: {e}");
    }
    for d in &outcome.details {
        let _ = writeln!(s, "{d}");
    }
    tracing::info!(case = %name, fails, "golden: case compared");
    CaseResult {
        summary: s,
        fails,
        outcome: Some(outcome),
    }
}

// frob:tests crates/cadetta-cli/tests/golden/main.rs::golden_corpus
#[test]
fn golden_corpus() {
    let strict = std::env::var("CADETTA_GOLDEN_STRICT").is_ok_and(|v| v == "1");
    let confirm_mode = std::env::var("CADETTA_GOLDEN_CONFIRM").is_ok_and(|v| v == "1");
    let current = current_spec().expect("current spec");
    let root = golden_root();
    let mut report = String::new();
    let mut failed = Vec::new();
    for dir in cases(&root) {
        let r = run_case(&dir, &current, strict);
        report.push_str(&r.summary);
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if r.fails {
            failed.push(name.clone());
        }
        let clean_run = r
            .outcome
            .as_ref()
            .is_some_and(|o| o.mismatches() + o.unevaluable() == 0);
        let hand = std::fs::read_to_string(dir.join("expected.toml"))
            .is_ok_and(|t| t.contains("\"hand-evaluated\""));
        if confirm_mode && clean_run && hand && !r.fails {
            match confirm::confirm_with(&dir, confirm::solve_in) {
                Ok(files) => report.push_str(&format!(
                    "{name}: confirmed ({} files written)\n",
                    files.len()
                )),
                Err(e) => {
                    report.push_str(&format!("{name}: confirm refused: {e}\n"));
                    failed.push(name);
                }
            }
        }
    }
    let _ = std::io::Write::write_all(&mut std::io::stderr(), report.as_bytes());
    if let Some(dir) = option_env!("CARGO_TARGET_TMPDIR") {
        let _ = std::fs::write(Path::new(dir).join("golden-report.txt"), &report);
    }
    assert!(failed.is_empty(), "golden cases failed: {failed:?}");
}

// frob:tests crates/cadetta-cli/tests/golden/main.rs::stale_spec_fails_until_ported
#[test]
fn stale_spec_fails_until_ported() {
    assert!(is_stale("1.4", "1.5"));
    assert!(!is_stale("1.5", "1.5"));
    let current = current_spec().expect("current spec");
    assert!(current.chars().all(|c| c.is_ascii_digit() || c == '.'));
}
