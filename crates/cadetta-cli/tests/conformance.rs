//! frob:ticket 01M4V0NSCWCW8W0P7DG628M8AP
//! Conformance corpus runner (tests/conformance/README.md): runs every
//! `NAME.expect.json` fixture pair through the `cadetta` library and compares
//! the diagnostics with the closed-list rules of the README.

#[path = "conformance/compare.rs"]
mod compare;
#[path = "conformance/corpus.rs"]
mod corpus;
#[path = "conformance/expect.rs"]
mod expect;
#[path = "conformance/run.rs"]
mod run;

use std::path::{Path, PathBuf};

use corpus::Outcome;

/// The repository's conformance corpus.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance")
}

/// The self-test corpus: one passing pair and one pair that must fail.
fn selftest_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/selftest")
}

// frob:tests crates/cadetta-cli/tests/conformance/corpus.rs::run_corpus
#[test]
fn conformance_corpus_matches_the_spec() {
    let allow = corpus::load_known_gaps(&corpus_root().join("known-gaps.toml"))
        .unwrap_or_else(|e| panic!("known-gaps.toml: {e}"));
    let report = corpus::run_corpus(&corpus_root(), &allow);
    for line in &report.notes {
        tracing::info!(note = %line, "conformance");
    }
    assert!(
        report.failures.is_empty(),
        "{} conformance failure(s):\n{}",
        report.failures.len(),
        report.failures.join("\n")
    );
}

// frob:tests crates/cadetta-cli/tests/conformance/corpus.rs::run_corpus
#[test]
fn unmet_expectation_fails_naming_fixture_doc_and_code() {
    let report = corpus::run_corpus(&selftest_root(), &corpus::KnownGaps::default());
    let results: std::collections::BTreeMap<_, _> = report
        .outcomes
        .iter()
        .map(|(name, o)| (name.as_str(), o))
        .collect();
    assert!(
        matches!(results["language/ok"], Outcome::Pass),
        "{report:?}"
    );
    assert!(
        matches!(results["language/bad"], Outcome::Fail(_)),
        "{report:?}"
    );
    let joined = report.failures.join("\n");
    assert!(joined.contains("language/bad"), "{joined}");
    assert!(joined.contains("[language]"), "{joined}");
    assert!(joined.contains("missing E999"), "{joined}");
    assert!(joined.contains("exit status 0"), "{joined}");
    assert!(!joined.contains("language/ok"), "{joined}");
}

// frob:tests crates/cadetta-cli/tests/conformance/corpus.rs::run_corpus
#[test]
fn known_gap_tolerates_failure_and_flags_stale_entry() {
    let mut allow = corpus::KnownGaps::default();
    allow.insert("language/bad", "~TEST", "implementation lags");
    let report = corpus::run_corpus(&selftest_root(), &allow);
    assert!(report.failures.is_empty(), "{report:?}");
    assert!(matches!(report.outcomes["language/bad"], Outcome::KnownGap));

    let mut stale = corpus::KnownGaps::default();
    stale.insert("language/ok", "~TEST", "passes now");
    let report = corpus::run_corpus(&selftest_root(), &stale);
    assert!(
        report.failures.iter().any(|f| f.contains("stale")),
        "{report:?}"
    );
}

// frob:tests crates/cadetta-cli/tests/conformance/corpus.rs::run_corpus
#[test]
fn empty_or_missing_corpus_is_tolerated() {
    let missing = std::env::temp_dir().join("cadetta-no-such-corpus");
    let report = corpus::run_corpus(&missing, &corpus::KnownGaps::default());
    assert!(report.failures.is_empty());
    assert!(report.outcomes.is_empty());
}
