//! Using solver outputs as checked values or as drivers (solvers.md 2 and 4.1).
//!
//! Decisions made here (written into solvers.md 2): a bound use yields a
//! [`UsedOutput`] that is only ever compared (`drives` is false) and leaves the
//! output's role untouched; a driving use marks the output driving, and the
//! derived value carries a [`SolverCitation`] naming the solver and output so
//! downstream values cite it. A result whose recorded digests differ from the
//! current ones is stale: the output still evaluates, carrying an
//! [`S003`] fact, so the rest of the design keeps checking.

// frob:ticket 01M4STANDBYZAR6T9BXPKHYYYV

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::decl::{OutputRole, SolverDecl};

/// The lint code for a stale solver result (lints.md S).
pub const S003: &str = "S003";

/// How an expression uses a solver output (solvers.md 2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutputUse {
    /// A check compares a design value against the output.
    Check,
    /// The output feeds a design value (`wall = round_to(s.wall, 0.5mm)`).
    Drive,
}

/// The digests a committed result was computed against, or the current ones.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Digests {
    /// Digest of every input value.
    pub input_digest: String,
    /// Digest of the script.
    pub script_digest: String,
    /// Digest of each dependency file by path.
    pub dep_digests: BTreeMap<String, String>,
}

/// One committed result for a solver (solvers.md 4): digests plus output literals.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct RecordedResult {
    /// What the result was computed against.
    pub digests: Digests,
    /// Output literals by name.
    pub outputs: BTreeMap<String, String>,
}

/// Names the solver output a derived value came from.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SolverCitation {
    /// The solver's canonical path.
    pub solver: String,
    /// The output name.
    pub output: String,
}

/// A stale-result fact attached to an output that still evaluates (S003).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StaleFact {
    /// Always [`S003`].
    pub code: &'static str,
    /// Which recorded digests differ: `inputs`, `script` or a dep path.
    pub changed: Vec<String>,
}

/// An evaluated solver output with its role, provenance and staleness.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct UsedOutput {
    /// The output literal from the committed result.
    pub value: String,
    /// True when the use drives a design value; false for a bound comparison.
    pub drives: bool,
    /// The solver citation downstream values must carry; set only when driving.
    pub cite: Option<SolverCitation>,
    /// The S003 fact when the result is stale; the value still stands.
    pub stale: Option<StaleFact>,
}

/// Why an output could not be used.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum OutputError {
    /// The solver declares no such output.
    #[error("solver `{solver}` has no output `{output}`")]
    UnknownOutput {
        /// The solver path.
        solver: String,
        /// The requested output.
        output: String,
    },
    /// The committed result has no value for the output.
    #[error("no recorded value for `{solver}.{output}`; run `cadetta solve`")]
    Unsolved {
        /// The solver path.
        solver: String,
        /// The requested output.
        output: String,
    },
}

/// List what differs between a recorded result and the current digests.
pub fn stale_changes(recorded: &Digests, current: &Digests) -> Vec<String> {
    let mut changed = Vec::new();
    if recorded.input_digest != current.input_digest {
        changed.push("inputs".to_string());
    }
    if recorded.script_digest != current.script_digest {
        changed.push("script".to_string());
    }
    let paths: std::collections::BTreeSet<_> = recorded
        .dep_digests
        .keys()
        .chain(current.dep_digests.keys())
        .collect();
    for p in paths {
        if recorded.dep_digests.get(p) != current.dep_digests.get(p) {
            changed.push(p.clone());
        }
    }
    changed
}

/// Evaluate `output` of `decl` for a `usage`, marking driving uses and attaching S003 when stale.
pub fn use_output(
    decl: &mut SolverDecl,
    output: &str,
    usage: OutputUse,
    recorded: &RecordedResult,
    current: &Digests,
) -> Result<UsedOutput, OutputError> {
    if !decl.outputs.contains_key(output) {
        warn!(solver = %decl.path, output, "unknown solver output");
        return Err(OutputError::UnknownOutput {
            solver: decl.path.clone(),
            output: output.to_string(),
        });
    }
    let Some(value) = recorded.outputs.get(output) else {
        warn!(solver = %decl.path, output, "output has no recorded value");
        return Err(OutputError::Unsolved {
            solver: decl.path.clone(),
            output: output.to_string(),
        });
    };
    let changed = stale_changes(&recorded.digests, current);
    let stale = if changed.is_empty() {
        None
    } else {
        info!(solver = %decl.path, output, ?changed, "S003 solver result is stale; output still evaluates");
        Some(StaleFact {
            code: S003,
            changed,
        })
    };
    let (drives, cite) = match usage {
        OutputUse::Check => {
            debug!(solver = %decl.path, output, "bound use: checked, never drives");
            (false, None)
        }
        OutputUse::Drive => {
            decl.mark_driving(output);
            debug_assert_eq!(decl.outputs[output].role, OutputRole::Driving);
            debug!(solver = %decl.path, output, "driving use: derived values cite the solver");
            (
                true,
                Some(SolverCitation {
                    solver: decl.path.clone(),
                    output: output.to_string(),
                }),
            )
        }
    };
    Ok(UsedOutput {
        value: value.clone(),
        drives,
        cite,
        stale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decl::{SolverClosure, SolverOutput};

    fn decl() -> SolverDecl {
        SolverDecl {
            path: "p::wall_sizing".into(),
            script: "w.py".into(),
            citation: None,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::from([(
                "min_wall".to_string(),
                SolverOutput {
                    kind: "Length".into(),
                    role: OutputRole::Bound,
                },
            )]),
            deps: vec![],
            closure: SolverClosure::default(),
        }
    }

    fn digests(i: &str) -> Digests {
        Digests {
            input_digest: i.into(),
            script_digest: "s".into(),
            dep_digests: BTreeMap::new(),
        }
    }

    fn rec(i: &str) -> RecordedResult {
        RecordedResult {
            digests: digests(i),
            outputs: BTreeMap::from([("min_wall".to_string(), "2.5mm".to_string())]),
        }
    }

    // frob:tests crates/cadetta-solve/src/outputs.rs::use_output kind=unit
    #[test]
    fn bound_output_is_checked_and_never_drives() {
        let mut d = decl();
        let u = use_output(
            &mut d,
            "min_wall",
            OutputUse::Check,
            &rec("a"),
            &digests("a"),
        )
        .expect("ok");
        assert_eq!(u.value, "2.5mm");
        assert!(!u.drives);
        assert!(u.cite.is_none());
        assert!(u.stale.is_none());
        assert_eq!(d.outputs["min_wall"].role, OutputRole::Bound);
    }

    // frob:tests crates/cadetta-solve/src/outputs.rs::use_output kind=unit
    #[test]
    fn driving_output_cites_the_solver() {
        let mut d = decl();
        let u = use_output(
            &mut d,
            "min_wall",
            OutputUse::Drive,
            &rec("a"),
            &digests("a"),
        )
        .expect("ok");
        assert!(u.drives);
        assert_eq!(
            u.cite,
            Some(SolverCitation {
                solver: "p::wall_sizing".into(),
                output: "min_wall".into()
            })
        );
        assert_eq!(d.outputs["min_wall"].role, OutputRole::Driving);
    }

    // frob:tests crates/cadetta-solve/src/outputs.rs::use_output kind=unit
    #[test]
    fn stale_output_still_evaluates_with_s003() {
        for usage in [OutputUse::Check, OutputUse::Drive] {
            let mut d = decl();
            let u = use_output(&mut d, "min_wall", usage, &rec("old"), &digests("new"))
                .expect("stale still evaluates");
            assert_eq!(u.value, "2.5mm");
            let f = u.stale.expect("fact");
            assert_eq!(f.code, "S003");
            assert_eq!(f.changed, ["inputs"]);
        }
        let mut cur = digests("a");
        cur.dep_digests.insert("m.csv".into(), "x".into());
        assert_eq!(stale_changes(&digests("a"), &cur), ["m.csv"]);
    }

    // frob:tests crates/cadetta-solve/src/outputs.rs::use_output kind=unit
    #[test]
    fn unknown_and_unsolved_outputs_error() {
        let mut d = decl();
        let e = use_output(&mut d, "nope", OutputUse::Check, &rec("a"), &digests("a"));
        assert!(matches!(e, Err(OutputError::UnknownOutput { .. })));
        let e = use_output(
            &mut d,
            "min_wall",
            OutputUse::Check,
            &RecordedResult::default(),
            &digests("a"),
        );
        assert!(matches!(e, Err(OutputError::Unsolved { .. })));
    }
}
