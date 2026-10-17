//! frob:ticket 01M4STCFQMGTXQAYPANNCHWEN4
//! Conformance run for the config-cli spec (the config-cli spec): drives
//! every `tests/conformance/config-cli/` fixture pair through the shared corpus
//! runner and requires each must-fire fixture to produce its expected
//! diagnostics and each must-not-fire fixture to produce none.

#[path = "../crates/cadetta-cli/tests/conformance/binding.rs"]
mod binding;
#[path = "../crates/cadetta-cli/tests/conformance/compare.rs"]
mod compare;
#[allow(dead_code)] // KnownGaps::insert is only used by the runner self-tests
#[path = "../crates/cadetta-cli/tests/conformance/corpus.rs"]
mod corpus;
#[path = "../crates/cadetta-cli/tests/conformance/exec.rs"]
mod exec;
#[path = "../crates/cadetta-cli/tests/conformance/expect.rs"]
mod expect;
#[path = "../crates/cadetta-cli/tests/conformance/run.rs"]
mod run;
#[path = "../crates/cadetta-cli/tests/conformance/verify.rs"]
mod verify;

use std::path::{Path, PathBuf};

use corpus::Outcome;

/// Document stem of the spec under test; also the fixture directory name.
const DOC: &str = "config-cli";

/// The repository's conformance corpus.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance")
}

/// Run only the requested doc(s) of the corpus and keep only the config-cli verdicts.
fn doc_report() -> (corpus::Report, Vec<String>) {
    let allow = corpus::load_known_gaps(&corpus_root().join("known-gaps.toml"))
        .unwrap_or_else(|e| panic!("known-gaps.toml: {e}"));
    let report = corpus::run_corpus_for(&corpus_root(), &allow, &[DOC]);
    let prefix = format!("{DOC}/");
    let failures = report
        .failures
        .iter()
        .filter(|f| f.starts_with(&prefix))
        .cloned()
        .collect();
    (report, failures)
}

// frob:tests crates/cadetta-cli/tests/conformance/corpus.rs::run_corpus
#[test]
fn config_cli_fixtures_match_the_spec() {
    let (report, failures) = doc_report();
    for (name, outcome) in &report.outcomes {
        if name.starts_with("config-cli/") && matches!(outcome, Outcome::KnownGap) {
            tracing::info!(fixture = %name, "config-cli known gap");
        }
    }
    assert!(
        failures.is_empty(),
        "{} config-cli conformance failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// frob:tests crates/cadetta-cli/tests/conformance/corpus.rs::run_corpus
#[test]
fn config_cli_corpus_is_not_empty() {
    let (report, _) = doc_report();
    let n = report
        .outcomes
        .keys()
        .filter(|k| k.starts_with("config-cli/"))
        .count();
    assert!(n > 0, "no config-cli fixtures were discovered");
}
