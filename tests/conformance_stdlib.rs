//! frob:ticket 01M4STCFHFMG7999J9WDPK1BC3
//! Conformance run for the stdlib spec: drives every
//! `tests/conformance/stdlib/`
//! fixture pair through the shared corpus runner and requires each must-fire
//! fixture to produce its expected diagnostics and each must-not-fire fixture
//! to produce none.

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

/// Fixture docs under test (corpus-relative directory names).
const DOCS: [&str; 1] = ["stdlib"];

/// The repository's conformance corpus.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance")
}

/// Run the whole corpus once and keep only the verdicts of `doc`.
fn doc_report(doc: &str) -> (corpus::Report, Vec<String>) {
    let allow = corpus::load_known_gaps(&corpus_root().join("known-gaps.toml"))
        .unwrap_or_else(|e| panic!("known-gaps.toml: {e}"));
    let report = corpus::run_corpus(&corpus_root(), &allow);
    let prefix = format!("{doc}/");
    let failures = report
        .failures
        .iter()
        .filter(|f| f.starts_with(&prefix))
        .cloned()
        .collect();
    (report, failures)
}

/// Count fixtures of exactly `doc` (not nested docs) in the report.
fn count_in(report: &corpus::Report, doc: &str) -> usize {
    report
        .outcomes
        .keys()
        .filter(|k| {
            k.strip_prefix(&format!("{doc}/"))
                .is_some_and(|r| !r.contains('/'))
        })
        .count()
}

// frob:tests crates/cadetta-cli/tests/conformance/corpus.rs::run_corpus
#[test]
fn stdlib_fixtures_match_the_spec() {
    for doc in DOCS {
        let (report, failures) = doc_report(doc);
        for (name, outcome) in &report.outcomes {
            if name.starts_with(&format!("{doc}/")) && matches!(outcome, Outcome::KnownGap) {
                tracing::info!(fixture = %name, doc, "conformance known gap");
            }
        }
        assert!(
            failures.is_empty(),
            "{} {doc} conformance failure(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

// frob:tests crates/cadetta-cli/tests/conformance/corpus.rs::run_corpus
#[test]
fn stdlib_corpus_is_not_empty() {
    for doc in DOCS {
        let (report, _) = doc_report(doc);
        assert!(
            count_in(&report, doc) > 0,
            "no {doc} fixtures were discovered"
        );
    }
}
