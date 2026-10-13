// frob:ticket 01M4ST8EEGK712KZT44DEDBSEG
//! Alternate viability, undecided results and per-alternate waivers (language.md 3.2).
//!
//! An alternate is viable when no check, fit, bound or require violates under
//! its variant. Undecided results leave it viable with an I003 fact. A
//! non-viable alternate stays listed (I002) naming each failure, reported as
//! `C001 for alt NAME`; C001/C002 are not raised for it. A waiver scoped to
//! one alternate applies to that variant only, but a `require` is never
//! suppressed, so a violated one always makes the alternate non-viable.

use std::fmt;

use tracing::{debug, info, trace, warn};

use crate::evaluate::{VariantOutcome, VariantResult};
use crate::slot::Slot;

/// Diagnostic code: an alternate that fails under its variant.
pub const I002: &str = "I002";
/// Diagnostic code: a result undecided under a variant (the alternate stays viable).
pub const I003: &str = "I003";
/// Diagnostic code: a violated bound or check.
pub const C001: &str = "C001";
/// Diagnostic code: an undecided bound or check.
pub const C002: &str = "C002";

/// What kind of obligation produced a verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObligationKind {
    /// A `check`.
    Check,
    /// A `fit`.
    Fit,
    /// An interface or ots bound.
    Bound,
    /// A `require`, which is never suppressed by a waiver.
    Require,
}

impl fmt::Display for ObligationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ObligationKind::Check => "check",
            ObligationKind::Fit => "fit",
            ObligationKind::Bound => "bound",
            ObligationKind::Require => "require",
        })
    }
}

/// How an obligation reads under a variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Holds over every tolerance.
    Pass,
    /// Violated.
    Violated,
    /// Cannot be decided (for example an unknown input).
    Undecided,
}

/// One obligation's verdict under a variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The obligation kind.
    pub kind: ObligationKind,
    /// The qualified target (`Rover.load_ok`).
    pub target: String,
    /// Its verdict.
    pub verdict: Verdict,
}

impl Finding {
    /// The lint code this verdict would raise on the primary: C001 violated, C002 undecided.
    pub fn code(&self) -> Option<&'static str> {
        match self.verdict {
            Verdict::Pass => None,
            Verdict::Violated => Some(C001),
            Verdict::Undecided => Some(C002),
        }
    }
}

/// A waiver, optionally scoped to one alternate (`for alt Rover/wheel=Colson2Series`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Waiver {
    /// The waived code (`C002`).
    pub code: String,
    /// The waived target.
    pub target: String,
    /// The variant key it is scoped to; `None` applies to every variant.
    pub alt: Option<String>,
}

impl Waiver {
    /// True when the waiver covers `finding` under the variant `key`.
    fn covers(&self, key: &str, finding: &Finding) -> bool {
        finding.kind != ObligationKind::Require
            && finding.code() == Some(self.code.as_str())
            && self.target == finding.target
            && self.alt.as_deref().is_none_or(|a| a == key)
    }
}

/// One reason an alternate is non-viable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// An obligation violated under the variant.
    Violation {
        /// The obligation kind.
        kind: ObligationKind,
        /// The target.
        target: String,
        /// The report label, `C001 for alt NAME`.
        label: String,
    },
    /// The variant failed to evaluate (E007, a failed `as`).
    EvaluationError {
        /// The error text.
        reason: String,
    },
}

/// The viability verdict for one alternate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Viability {
    /// The variant key `slot=alternate`.
    pub key: String,
    /// True when nothing violates (undecided results do not make it non-viable).
    pub viable: bool,
    /// Each failure; I002 names every one. Empty when viable.
    pub failures: Vec<Failure>,
    /// Targets undecided under the variant: the I003 facts (not waived).
    pub undecided: Vec<String>,
}

impl Viability {
    /// The I002 diagnostic code when non-viable.
    pub fn i002(&self) -> Option<&'static str> {
        (!self.viable).then_some(I002)
    }

    /// The I003 diagnostic code when some result is undecided.
    pub fn i003(&self) -> Option<&'static str> {
        (!self.undecided.is_empty()).then_some(I003)
    }
}

/// Judges one alternate from its findings; a non-viable evaluation outcome is a failure.
///
/// C001/C002 are the primary's verdicts and are not raised for the alternate.
pub fn judge(
    result: &VariantResult<impl Clone>,
    findings: &[Finding],
    waivers: &[Waiver],
) -> Viability {
    let key = result.key();
    let mut failures = Vec::new();
    let mut undecided = Vec::new();
    if let VariantOutcome::NonViable { reason } = &result.outcome {
        warn!(variant = %key, %reason, "non-viable: evaluation error");
        failures.push(Failure::EvaluationError {
            reason: reason.clone(),
        });
    }
    for f in findings {
        let Some(code) = f.code() else { continue };
        if waivers.iter().any(|w| w.covers(&key, f)) {
            debug!(variant = %key, target = %f.target, code, "finding waived for this variant");
            continue;
        }
        match f.verdict {
            Verdict::Violated => {
                warn!(variant = %key, target = %f.target, kind = %f.kind, "violation under variant");
                failures.push(Failure::Violation {
                    kind: f.kind,
                    target: f.target.clone(),
                    label: format!("{code} for alt {key}"),
                });
            }
            Verdict::Undecided => {
                trace!(variant = %key, target = %f.target, "undecided under variant");
                undecided.push(f.target.clone());
            }
            Verdict::Pass => {}
        }
    }
    let viable = failures.is_empty();
    info!(variant = %key, viable, undecided = undecided.len(), "alternate judged");
    Viability {
        key,
        viable,
        failures,
        undecided,
    }
}

/// Why an alternate cannot be promoted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromoteError {
    /// The name that is not an alternate of the slot.
    pub alt: String,
}

impl fmt::Display for PromoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not an approved alternate", self.alt)
    }
}

impl std::error::Error for PromoteError {}

/// Promotes `alt` to primary; it leaves the list, and the old primary joins it only if `keep_old`.
pub fn promote(slot: &Slot, alt: &str, keep_old: bool) -> Result<Slot, PromoteError> {
    if !slot.alts.iter().any(|a| a == alt) {
        warn!(slot = %slot.name, %alt, "promote of a non-alternate");
        return Err(PromoteError { alt: alt.into() });
    }
    let mut out = slot.clone();
    out.alts.retain(|a| a != alt);
    if keep_old && let Some(old) = &slot.primary {
        out.alts.push(old.clone());
    }
    out.primary = Some(alt.into());
    debug!(slot = %slot.name, %alt, keep_old, "alternate promoted");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluate::{Selection, VariantOutcome};
    use cadetta_syntax::ast::Span;
    use std::collections::BTreeMap;

    fn res(outcome: VariantOutcome<i64>) -> VariantResult<i64> {
        VariantResult {
            selection: Selection {
                slot: "Rover/wheel".into(),
                alt: "Colson2Series".into(),
            },
            outcome,
        }
    }
    fn ok() -> VariantResult<i64> {
        res(VariantOutcome::Evaluated {
            diff: BTreeMap::new(),
        })
    }
    fn f(kind: ObligationKind, target: &str, verdict: Verdict) -> Finding {
        Finding {
            kind,
            target: target.into(),
            verdict,
        }
    }
    fn waiver(code: &str, target: &str, alt: Option<&str>) -> Waiver {
        Waiver {
            code: code.into(),
            target: target.into(),
            alt: alt.map(Into::into),
        }
    }
    const KEY: &str = "Rover/wheel=Colson2Series";

    // frob:tests crates/cadetta-variants/src/viability.rs::judge kind=unit
    #[test]
    fn no_violation_is_viable() {
        let v = judge(
            &ok(),
            &[f(ObligationKind::Check, "Rover.a", Verdict::Pass)],
            &[],
        );
        assert!(v.viable && v.failures.is_empty() && v.i002().is_none() && v.i003().is_none());
    }

    // frob:tests crates/cadetta-variants/src/viability.rs::judge kind=unit
    #[test]
    fn undecided_stays_viable_with_i003() {
        let v = judge(
            &ok(),
            &[f(
                ObligationKind::Check,
                "Rover.load_ok",
                Verdict::Undecided,
            )],
            &[],
        );
        assert!(v.viable);
        assert_eq!(v.i003(), Some(I003));
        assert_eq!(v.undecided, ["Rover.load_ok"]);
    }

    // frob:tests crates/cadetta-variants/src/viability.rs::judge kind=unit
    #[test]
    fn violation_is_non_viable_and_labelled() {
        let v = judge(
            &ok(),
            &[f(ObligationKind::Bound, "Caster.bore", Verdict::Violated)],
            &[],
        );
        assert!(!v.viable);
        assert_eq!(v.i002(), Some(I002));
        assert_eq!(
            v.failures,
            [Failure::Violation {
                kind: ObligationKind::Bound,
                target: "Caster.bore".into(),
                label: format!("C001 for alt {KEY}"),
            }]
        );
    }

    // frob:tests crates/cadetta-variants/src/viability.rs::Waiver kind=unit
    #[test]
    fn waiver_applies_to_that_variant_only() {
        let fs = [f(ObligationKind::Check, "Rover.load_ok", Verdict::Violated)];
        let w = [waiver("C001", "Rover.load_ok", Some(KEY))];
        assert!(judge(&ok(), &fs, &w).viable);
        let other = [waiver("C001", "Rover.load_ok", Some("Rover/wheel=Other"))];
        assert!(!judge(&ok(), &fs, &other).viable);
        let undec = [f(
            ObligationKind::Check,
            "Rover.load_ok",
            Verdict::Undecided,
        )];
        let w2 = [waiver("C002", "Rover.load_ok", Some(KEY))];
        assert!(judge(&ok(), &undec, &w2).i003().is_none());
    }

    // frob:tests crates/cadetta-variants/src/viability.rs::judge kind=unit
    #[test]
    fn waived_require_still_non_viable() {
        let fs = [f(ObligationKind::Require, "Rover.r", Verdict::Violated)];
        let w = [
            waiver("C001", "Rover.r", Some(KEY)),
            waiver("C001", "Rover.r", None),
        ];
        assert!(!judge(&ok(), &fs, &w).viable);
    }

    // frob:tests crates/cadetta-variants/src/viability.rs::judge kind=unit
    #[test]
    fn i002_names_each_failure_and_eval_error() {
        let v = judge(
            &res(VariantOutcome::NonViable {
                reason: "E007".into(),
            }),
            &[
                f(ObligationKind::Check, "a", Verdict::Violated),
                f(ObligationKind::Fit, "b", Verdict::Violated),
                f(ObligationKind::Require, "c", Verdict::Violated),
            ],
            &[],
        );
        assert_eq!(v.failures.len(), 4);
        assert!(matches!(v.failures[0], Failure::EvaluationError { .. }));
    }

    // frob:tests crates/cadetta-variants/src/viability.rs::promote kind=unit
    #[test]
    fn promote_keeps_old_primary_only_if_listed() {
        let slot = Slot {
            name: "wheel".into(),
            interface: "Caster".into(),
            primary: Some("Vendor4471B".into()),
            alts: vec!["Colson2Series".into(), "Z".into()],
            span: Span::default(),
        };
        let dropped = promote(&slot, "Colson2Series", false).unwrap();
        assert_eq!(dropped.primary.as_deref(), Some("Colson2Series"));
        assert_eq!(dropped.alts, ["Z"]);
        let kept = promote(&slot, "Colson2Series", true).unwrap();
        assert_eq!(kept.alts, ["Z", "Vendor4471B"]);
        assert!(promote(&slot, "Nope", false).is_err());
    }
}
