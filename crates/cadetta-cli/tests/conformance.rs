//! frob:ticket 01M4V0NSCWCW8W0P7DG628M8AP
//! Conformance corpus runner (tests/conformance/README.md): runs every
//! `NAME.expect.json` fixture pair through the `cadetta` library and compares
//! the diagnostics with the closed-list rules of the README.

#[path = "conformance/binding.rs"]
mod binding;
#[path = "conformance/compare.rs"]
mod compare;
#[path = "conformance/corpus.rs"]
mod corpus;
#[path = "conformance/exec.rs"]
mod exec;
#[path = "conformance/expect.rs"]
mod expect;
#[path = "conformance/run.rs"]
mod run;
#[path = "conformance/verify.rs"]
mod verify;

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
fn nested_doc_directories_are_discovered() {
    let report = corpus::run_corpus(&selftest_root(), &corpus::KnownGaps::default());
    assert!(
        matches!(report.outcomes["emitters/nested/ok"], Outcome::Pass),
        "{report:?}"
    );
    assert!(
        !report.outcomes.keys().any(|k| k == "emitters/ok"),
        "{report:?}"
    );
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

/// Copy the directory `from` into `to`, recursively.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

// frob:ticket 01M4YG7D22XEEK9M3X678B88BT
// frob:tests crates/cadetta-cli/tests/conformance/corpus.rs::run_corpus
#[test]
fn bare_adapter_command_is_not_run_from_path_and_finishes() {
    // The fixture's adapter is a bare `sh`; PATH-searching it ran the system shell on the
    // initialize request and hung. Only that one fixture is run, in a scratch corpus.
    let scratch = std::env::temp_dir().join(format!("cadetta-r008-corpus-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let writing = run::writing();
    let src = corpus_root().join("adapters");
    for entry in std::fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with("r008-no-path-search")
        {
            let dest = scratch.join("adapters").join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &dest);
            } else {
                std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
                std::fs::copy(entry.path(), &dest).unwrap();
            }
        }
    }
    drop(writing);
    let start = std::time::Instant::now();
    let report = corpus::run_corpus(&scratch, &corpus::KnownGaps::default());
    let _ = std::fs::remove_dir_all(&scratch);
    // Unrelated lint noise (e.g. U002) is not this test's concern; a missing R008 is.
    assert!(
        report.failures.iter().all(|f| !f.contains("missing")),
        "R008 was not raised: {report:?}"
    );
    assert!(
        report.outcomes.contains_key("adapters/r008-no-path-search"),
        "{report:?}"
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(30));
}

// frob:ticket 01M4YR9ZE0N4CF36GDZT2WRCP1
// frob:tests crates/cadetta-cli/tests/conformance/exec.rs::eval
#[test]
fn eval_entry_reports_q006_info_for_exact_trig_fixtures() {
    // Only the five quantities fixtures of the ticket are run, in a scratch corpus.
    let names = [
        "c002-inexact-boundary",
        "exact-zero-normalised",
        "trig-exact-atan2",
        "trig-exact-sin",
        "trig-exact-tan",
    ];
    let scratch = std::env::temp_dir().join(format!("cadetta-q006-corpus-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let writing = run::writing();
    let src = corpus_root().join("quantities");
    let dest = scratch.join("quantities");
    std::fs::create_dir_all(&dest).unwrap();
    for entry in std::fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        let file = entry.file_name().to_string_lossy().into_owned();
        if names.iter().any(|n| file.starts_with(&format!("{n}."))) {
            std::fs::copy(entry.path(), dest.join(&file)).unwrap();
        }
    }
    drop(writing);
    let report = corpus::run_corpus(&scratch, &corpus::KnownGaps::default());
    let _ = std::fs::remove_dir_all(&scratch);
    assert_eq!(report.outcomes.len(), names.len(), "{report:?}");
    assert!(report.failures.is_empty(), "{report:?}");
}

// frob:ticket 01M4X4044493Y32A7Y146BS0GA
// frob:tests crates/cadetta-cli/tests/conformance/run.rs::writing
#[test]
fn script_spawns_never_overlap_concurrent_script_writes() {
    use std::os::unix::fs::PermissionsExt;
    let base = std::env::temp_dir().join(format!("cadetta-etxtbsy-{}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    let workers: Vec<_> = (0..8)
        .map(|w| {
            let base = base.clone();
            std::thread::spawn(move || {
                for i in 0..25 {
                    let script = base.join(format!("s{w}-{i}.sh"));
                    {
                        let _writing = run::writing();
                        std::fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
                        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
                            .unwrap();
                    }
                    let _spawning = run::spawning();
                    let status = std::process::Command::new(&script).status();
                    assert!(status.is_ok_and(|s| s.success()), "{script:?}");
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let _ = std::fs::remove_dir_all(&base);
}

// frob:ticket 01M4VQXMX3SCM7CEBW4HX96KHW
// frob:tests crates/cadetta-cli/tests/conformance/binding.rs::display
#[test]
fn display_binding_fixtures_execute_and_match() {
    // Only the `display-*` quantities fixtures are run, in a scratch corpus; the one with a
    // fraction value (`1/3m`) is a known gap (~E3MA4MY) and is not run here.
    let scratch =
        std::env::temp_dir().join(format!("cadetta-display-corpus-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let writing = run::writing();
    let dest = scratch.join("quantities");
    std::fs::create_dir_all(&dest).unwrap();
    for entry in std::fs::read_dir(corpus_root().join("quantities")).unwrap() {
        let entry = entry.unwrap();
        let file = entry.file_name().to_string_lossy().into_owned();
        if file.starts_with("display-") && !file.starts_with("display-plain-exact-decimal.") {
            std::fs::copy(entry.path(), dest.join(&file)).unwrap();
        }
    }
    drop(writing);
    let report = corpus::run_corpus(&scratch, &corpus::KnownGaps::default());
    let _ = std::fs::remove_dir_all(&scratch);
    assert_eq!(report.outcomes.len(), 9, "{report:?}");
    assert!(
        report.outcomes.values().all(|o| matches!(o, Outcome::Pass)),
        "{report:?}"
    );
}

// frob:ticket 01M4YR9ZAJFNG41PMJECE627BF
// frob:tests crates/cadetta-cli/tests/conformance/exec.rs::names_path_or_key
#[test]
fn bind_invalid_fixtures_observe_the_named_path_or_key() {
    // Only the seven `bind-invalid-*` adapters fixtures are run, in a scratch corpus.
    let scratch =
        std::env::temp_dir().join(format!("cadetta-bindinv-corpus-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let writing = run::writing();
    let dest = scratch.join("adapters");
    for entry in std::fs::read_dir(corpus_root().join("adapters")).unwrap() {
        let entry = entry.unwrap();
        let file = entry.file_name().to_string_lossy().into_owned();
        if file.starts_with("bind-invalid-") {
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &dest.join(&file));
            } else {
                std::fs::create_dir_all(&dest).unwrap();
                std::fs::copy(entry.path(), dest.join(&file)).unwrap();
            }
        }
    }
    drop(writing);
    let report = corpus::run_corpus(&scratch, &corpus::KnownGaps::default());
    let _ = std::fs::remove_dir_all(&scratch);
    assert_eq!(report.outcomes.len(), 7, "{report:?}");
    assert!(
        report.outcomes.values().all(|o| matches!(o, Outcome::Pass)),
        "{report:?}"
    );
}

// frob:ticket 01M4V1HEFZVEV85KWZEZV526KN
// frob:tests crates/cadetta-cli/tests/conformance/verify.rs::check_output
#[test]
fn emit_point_fixtures_observe_the_emitted_value() {
    // Only the `emit-point-*` quantities fixtures are run, in a scratch corpus; the one
    // for reference inheritance (`emit-point-inherited`) is a known gap (~AQ8KSJA).
    let scratch =
        std::env::temp_dir().join(format!("cadetta-emitpoint-corpus-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let writing = run::writing();
    let dest = scratch.join("quantities");
    std::fs::create_dir_all(&dest).unwrap();
    for entry in std::fs::read_dir(corpus_root().join("quantities")).unwrap() {
        let entry = entry.unwrap();
        let file = entry.file_name().to_string_lossy().into_owned();
        if file.starts_with("emit-point-") && !file.starts_with("emit-point-inherited.") {
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &dest.join(&file));
            } else {
                std::fs::copy(entry.path(), dest.join(&file)).unwrap();
            }
        }
    }
    drop(writing);
    let report = corpus::run_corpus(&scratch, &corpus::KnownGaps::default());
    let _ = std::fs::remove_dir_all(&scratch);
    assert_eq!(report.outcomes.len(), 5, "{report:?}");
    assert!(
        report.outcomes.values().all(|o| matches!(o, Outcome::Pass)),
        "{report:?}"
    );
}

// frob:ticket 01M4ZCEYEDA4W3R18F6420HBMQ
// frob:tests crates/cadetta-cli/tests/conformance/exec.rs::solver_runs
#[test]
fn solve_variants_fixtures_observe_solver_runs_via_marker() {
    // Only the two `solve --variants` run-count fixtures are run, in a scratch corpus: equal
    // alternate inputs share one run (2 total) and unchanged inputs read the primary (1 total).
    let scratch =
        std::env::temp_dir().join(format!("cadetta-variants-corpus-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let writing = run::writing();
    let dest = scratch.join("solvers");
    std::fs::create_dir_all(&dest).unwrap();
    for entry in std::fs::read_dir(corpus_root().join("solvers")).unwrap() {
        let entry = entry.unwrap();
        let file = entry.file_name().to_string_lossy().into_owned();
        if file.starts_with("variants-equal-inputs-share-one-run.")
            || file.starts_with("variants-unchanged-inputs-read-primary.")
        {
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &dest.join(&file));
            } else {
                std::fs::copy(entry.path(), dest.join(&file)).unwrap();
            }
        }
    }
    drop(writing);
    let report = corpus::run_corpus(&scratch, &corpus::KnownGaps::default());
    let _ = std::fs::remove_dir_all(&scratch);
    assert_eq!(report.outcomes.len(), 2, "{report:?}");
    assert!(
        report.outcomes.values().all(|o| matches!(o, Outcome::Pass)),
        "{report:?}"
    );
}
