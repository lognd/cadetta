//! Solver lints S003, S005 and S006 (lints.md S; solvers.md 4 and 4.1).
//!
//! Decisions made here (written into solvers.md 4.1): S003 fires only for a
//! stored result whose digests differ from the current ones and names what
//! changed in a note; S005 fires for a failed run with a solver-level code
//! (protocol failures that carry another code, such as Q001, are reported
//! under that code instead) and says whether a previous result stays in use;
//! S006 fires only for a variant (never the primary) whose own result is
//! missing, and a missing primary result is an unsolved output, not S006.

// frob:ticket 01M4STANFNKA8G30C6KEEBM38C

use cadetta_lint::diag::{Diagnostic, Note, Severity};
use tracing::{debug, info};

use crate::outputs::{Digests, S003, stale_changes};
use crate::results::{CurrentDigests, Lookup};
use crate::transport::TransportError;

/// The lint code for a failed solve (lints.md S).
pub const S005: &str = "S005";

/// The lint code for a variant with no solve result (lints.md S).
pub const S006: &str = "S006";

fn variant_label(variant: Option<&str>) -> String {
    variant.map_or_else(String::new, |v| format!(" under variant {v}"))
}

/// S003: the diagnostic for a stale result, naming what changed (`inputs`, `script` or a dep path).
pub fn stale(solver: &str, variant: Option<&str>, changed: &[String]) -> Diagnostic {
    info!(solver, ?variant, ?changed, "S003 solver-stale");
    let mut d = Diagnostic::new(
        S003,
        Severity::Warning,
        format!(
            "solver result for `{solver}`{} is stale",
            variant_label(variant)
        ),
    );
    d.path = Some(solver.to_string());
    d.alt = variant.map(str::to_string);
    d.notes = changed
        .iter()
        .map(|c| Note {
            message: format!("changed: {c}"),
            location: None,
        })
        .collect();
    d
}

/// S005: the diagnostic for a failed solve, or `None` when the failure carries another code.
pub fn failed(
    solver: &str,
    err: &TransportError,
    previous_result_in_use: bool,
) -> Option<Diagnostic> {
    if err.code() != S005 {
        debug!(
            solver,
            code = err.code(),
            "failure is not S005; not linted here"
        );
        return None;
    }
    info!(solver, previous_result_in_use, "S005 solver-failed");
    let tail = if previous_result_in_use {
        "; the previous result is in use"
    } else {
        "; no previous result exists"
    };
    let mut d = Diagnostic::new(
        S005,
        Severity::Error,
        format!("solver `{solver}` failed: {err}{tail}"),
    );
    d.path = Some(solver.to_string());
    Some(d)
}

/// S006: the diagnostic for a variant whose solve result is missing.
pub fn unsolved_variant(solver: &str, variant: &str) -> Diagnostic {
    info!(solver, variant, "S006 solver-unsolved-variant");
    let mut d = Diagnostic::new(
        S006,
        Severity::Warning,
        format!(
            "no solver result for `{solver}` under variant {variant}; dependent verdicts are undecided"
        ),
    );
    d.path = Some(solver.to_string());
    d.alt = Some(variant.to_string());
    d
}

/// Lint a stored-result lookup: S003 for stale, S006 for a missing variant result, else nothing.
pub fn lint_lookup(
    solver: &str,
    variant: Option<&str>,
    lookup: &Lookup<'_>,
    current: &CurrentDigests,
) -> Option<Diagnostic> {
    match lookup {
        Lookup::Fresh(_) => None,
        Lookup::Stale(r) => {
            let recorded = Digests {
                input_digest: r.input_digest.clone(),
                script_digest: r.script_digest.clone(),
                dep_digests: r.dep_digests.clone(),
            };
            let now = Digests {
                input_digest: current.input_digest.clone(),
                script_digest: current.script_digest.clone(),
                dep_digests: current.dep_digests.clone(),
            };
            Some(stale(solver, variant, &stale_changes(&recorded, &now)))
        }
        Lookup::Missing => variant.map(|v| unsolved_variant(solver, v)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::{SolveResult, digest_bytes};
    use std::collections::BTreeMap;

    fn result(variant: Option<&str>) -> SolveResult {
        SolveResult {
            solver: "p::s".into(),
            input_digest: digest_bytes(b"in"),
            script_digest: digest_bytes(b"sc"),
            dep_digests: BTreeMap::new(),
            version: None,
            variant: variant.map(str::to_string),
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            diagnostics: vec![],
        }
    }

    fn current(input: &[u8]) -> CurrentDigests {
        CurrentDigests {
            input_digest: digest_bytes(input),
            script_digest: digest_bytes(b"sc"),
            dep_digests: BTreeMap::new(),
        }
    }

    // frob:tests crates/cadetta-solve/src/lints.rs::lint_lookup kind=unit
    #[test]
    fn s003_fires_when_stale_and_not_when_fresh() {
        let r = result(None);
        let d = lint_lookup("p::s", None, &Lookup::Stale(&r), &current(b"changed")).unwrap();
        assert_eq!(d.code, "S003");
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.notes[0].message, "changed: inputs");
        assert!(lint_lookup("p::s", None, &Lookup::Fresh(&r), &current(b"in")).is_none());
    }

    // frob:tests crates/cadetta-solve/src/lints.rs::failed kind=unit
    #[test]
    fn s005_fires_for_failed_run_and_not_for_other_codes() {
        let e = TransportError::Timeout {
            solver: "p::s".into(),
            secs: 5,
        };
        let d = failed("p::s", &e, true).unwrap();
        assert_eq!(d.code, "S005");
        assert_eq!(d.severity, Severity::Error);
        assert!(d.message.contains("previous result is in use"));
        let d = failed("p::s", &e, false).unwrap();
        assert!(d.message.contains("no previous result"));
        let q = TransportError::Protocol {
            solver: "p::s".into(),
            source: crate::protocol::ProtocolError::MalformedJson("x".into()),
        };
        if q.code() != "S005" {
            assert!(failed("p::s", &q, true).is_none());
        }
    }

    // frob:tests crates/cadetta-solve/src/lints.rs::lint_lookup kind=unit
    #[test]
    fn s006_fires_for_missing_variant_but_not_missing_primary() {
        let d = lint_lookup("p::s", Some("A=B"), &Lookup::Missing, &current(b"in")).unwrap();
        assert_eq!(d.code, "S006");
        assert_eq!(d.alt.as_deref(), Some("A=B"));
        assert!(lint_lookup("p::s", None, &Lookup::Missing, &current(b"in")).is_none());
        let r = result(Some("A=B"));
        assert!(lint_lookup("p::s", Some("A=B"), &Lookup::Fresh(&r), &current(b"in")).is_none());
    }
}
