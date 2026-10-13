// frob:ticket 01M4ST8EC6HVWM52PEHS43F430
//! Variant evaluation, one slot at a time (language.md 3.2, ir.md 3, solvers.md 4.1).
//!
//! The design is evaluated once with every primary, then once per approved
//! alternate of every slot with that alternate replacing the primary in all
//! instances of its slot and every other slot on its primary. The cost is
//! linear in the number of alternates, never a cross product. Only values
//! that differ from the primary evaluation are kept, keyed `slot=alternate`.
//! An evaluation error under a variant makes that alternate non-viable with
//! the error as reason; it never fails the run.
//!
//! The engine is generic over [`Evaluate`] so the language evaluator (which
//! re-derives values, part arguments, fits, checks, requires and roll-ups for
//! a selection) plugs in without this crate depending on it.

use std::collections::BTreeMap;
use std::fmt;

use tracing::{debug, info, trace, warn};

use crate::slot::Slot;

/// What a value reads as: known, or unknown (for example a solver result missing under a variant).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reading<V> {
    /// A known value.
    Value(V),
    /// Unknown: never borrowed from the primary (solvers.md 4.1).
    Unknown,
}

/// One slot swap: the slot name and the alternate that replaces its primary everywhere.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    /// The slot name (`Rover/wheel`); applies to every instance of the slot.
    pub slot: String,
    /// The alternate implementer replacing the primary.
    pub alt: String,
}

impl Selection {
    /// The variant key `slot=alternate` (ir.md 3).
    pub fn key(&self) -> String {
        format!("{}={}", self.slot, self.alt)
    }
}

/// A design evaluator: re-derives every value under an optional selection.
pub trait Evaluate {
    /// A member value (or a check/require verdict) as the evaluator represents it.
    type Value: Clone + PartialEq;
    /// Why an evaluation failed as a whole (an E007 lookup miss, a failed `as`).
    type Error: fmt::Display;

    /// Evaluates the whole design; `None` selects every primary.
    ///
    /// Values the evaluator cannot determine under the selection (such as a
    /// solver output with no result for the changed inputs) are returned as
    /// [`Reading::Unknown`], never borrowed from the primary.
    fn evaluate(
        &mut self,
        selection: Option<&Selection>,
    ) -> Result<BTreeMap<String, Reading<Self::Value>>, Self::Error>;
}

/// The outcome of one alternate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariantOutcome<V> {
    /// Evaluated; holds only the readings that differ from the primary.
    Evaluated {
        /// Member path to its reading under this variant.
        diff: BTreeMap<String, Reading<V>>,
    },
    /// Evaluation failed under the variant; the alternate is non-viable.
    NonViable {
        /// The evaluation error, as text.
        reason: String,
    },
}

/// One alternate's evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantResult<V> {
    /// The swap evaluated.
    pub selection: Selection,
    /// Its outcome.
    pub outcome: VariantOutcome<V>,
}

impl<V> VariantResult<V> {
    /// The variant key `slot=alternate`.
    pub fn key(&self) -> String {
        self.selection.key()
    }
}

/// The primary evaluation and every per-alternate variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantRun<V> {
    /// The primary evaluation.
    pub primary: BTreeMap<String, Reading<V>>,
    /// One result per alternate, slots in order, alternates in written order.
    pub variants: Vec<VariantResult<V>>,
    /// How many design evaluations ran: 1 plus the alternate count.
    pub evaluations: usize,
}

impl<V: Clone> VariantRun<V> {
    /// Per member path, the readings that differ, keyed `slot=alternate` (the IR `variants` field).
    pub fn by_member(&self) -> BTreeMap<String, BTreeMap<String, Reading<V>>> {
        let mut out: BTreeMap<String, BTreeMap<String, Reading<V>>> = BTreeMap::new();
        for v in &self.variants {
            if let VariantOutcome::Evaluated { diff } = &v.outcome {
                for (path, reading) in diff {
                    out.entry(path.clone())
                        .or_default()
                        .insert(v.key(), reading.clone());
                }
            }
        }
        out
    }
}

/// Readings in `variant` that differ from `primary`; a path the variant lacks reads unknown.
fn diff_readings<V: Clone + PartialEq>(
    primary: &BTreeMap<String, Reading<V>>,
    variant: BTreeMap<String, Reading<V>>,
) -> BTreeMap<String, Reading<V>> {
    let mut diff = BTreeMap::new();
    for (path, reading) in &variant {
        if primary.get(path) != Some(reading) {
            trace!(%path, "value differs under variant");
            diff.insert(path.clone(), reading.clone());
        }
    }
    for path in primary.keys().filter(|p| !variant.contains_key(*p)) {
        trace!(%path, "value absent under variant, reads unknown");
        diff.insert(path.clone(), Reading::Unknown);
    }
    diff
}

/// Evaluates the primary design and then each alternate of each slot, one slot swap at a time.
///
/// Returns the primary's error when the primary itself fails to evaluate,
/// since there is nothing to diff against; a failing alternate never errors.
pub fn evaluate_variants<E: Evaluate>(
    slots: &[Slot],
    evaluator: &mut E,
) -> Result<VariantRun<E::Value>, E::Error> {
    let primary = evaluator.evaluate(None)?;
    let mut evaluations = 1;
    let mut variants = Vec::new();
    for slot in slots {
        for alt in &slot.alts {
            let selection = Selection {
                slot: slot.name.clone(),
                alt: alt.clone(),
            };
            evaluations += 1;
            let outcome = match evaluator.evaluate(Some(&selection)) {
                Ok(values) => {
                    let diff = diff_readings(&primary, values);
                    debug!(variant = %selection.key(), changed = diff.len(), "variant evaluated");
                    VariantOutcome::Evaluated { diff }
                }
                Err(e) => {
                    warn!(variant = %selection.key(), error = %e, "variant non-viable: evaluation error");
                    VariantOutcome::NonViable {
                        reason: e.to_string(),
                    }
                }
            };
            variants.push(VariantResult { selection, outcome });
        }
    }
    info!(slots = slots.len(), evaluations, "variant evaluation done");
    Ok(VariantRun {
        primary,
        variants,
        evaluations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::Span;

    fn slot(name: &str, alts: &[&str]) -> Slot {
        Slot {
            name: name.into(),
            interface: "I".into(),
            primary: Some("P".into()),
            alts: alts.iter().map(|s| (*s).to_owned()).collect(),
            span: Span::default(),
        }
    }

    /// Models `total = sum of per-slot mass`, `bad` fails a lookup, solver output goes unknown.
    struct Model {
        calls: Vec<Option<Selection>>,
    }

    impl Evaluate for Model {
        type Value = i64;
        type Error = String;
        fn evaluate(
            &mut self,
            sel: Option<&Selection>,
        ) -> Result<BTreeMap<String, Reading<i64>>, String> {
            self.calls.push(sel.cloned());
            let mut m = BTreeMap::new();
            let (wheel, motor) = (
                if sel.is_some_and(|s| s.alt == "Heavy") {
                    9
                } else {
                    5
                },
                if sel.is_some_and(|s| s.alt == "Strong") {
                    7
                } else {
                    3
                },
            );
            if sel.is_some_and(|s| s.alt == "Bad") {
                return Err("E007: lookup miss".into());
            }
            m.insert("wheel.mass".into(), Reading::Value(wheel));
            m.insert("motor.mass".into(), Reading::Value(motor));
            m.insert("total".into(), Reading::Value(wheel * 4 + motor));
            m.insert("fixed".into(), Reading::Value(1));
            m.insert(
                "solved".into(),
                if sel.is_some_and(|s| s.alt == "Strong") {
                    Reading::Unknown
                } else {
                    Reading::Value(2)
                },
            );
            Ok(m)
        }
    }

    // frob:tests crates/cadetta-variants/src/evaluate.rs::evaluate_variants kind=unit
    #[test]
    fn alternate_replaces_primary_and_others_keep_primaries() {
        let mut m = Model { calls: vec![] };
        let run = evaluate_variants(
            &[
                slot("Rover/wheel", &["Heavy"]),
                slot("Rover/motor", &["Strong"]),
            ],
            &mut m,
        )
        .unwrap();
        let by = run.by_member();
        assert_eq!(by["wheel.mass"]["Rover/wheel=Heavy"], Reading::Value(9));
        assert_eq!(by["total"]["Rover/wheel=Heavy"], Reading::Value(39));
        assert!(!by["motor.mass"].contains_key("Rover/wheel=Heavy"));
        assert_eq!(by["total"]["Rover/motor=Strong"], Reading::Value(27));
    }

    // frob:tests crates/cadetta-variants/src/evaluate.rs::evaluate_variants kind=unit
    #[test]
    fn work_is_linear_in_alternates() {
        let mut m = Model { calls: vec![] };
        let slots = [slot("a", &["Heavy", "X"]), slot("b", &["Strong", "Y", "Z"])];
        let run = evaluate_variants(&slots, &mut m).unwrap();
        assert_eq!(run.evaluations, 1 + 5);
        assert_eq!(m.calls.len(), 6);
        assert_eq!(run.variants.len(), 5);
    }

    // frob:tests crates/cadetta-variants/src/evaluate.rs::VariantRun.by_member kind=unit
    #[test]
    fn only_differing_values_are_kept() {
        let mut m = Model { calls: vec![] };
        let run = evaluate_variants(&[slot("Rover/wheel", &["Heavy"])], &mut m).unwrap();
        let by = run.by_member();
        assert!(!by.contains_key("fixed"));
        assert!(!by.contains_key("motor.mass"));
        assert_eq!(by.len(), 2);
    }

    // frob:tests crates/cadetta-variants/src/evaluate.rs::evaluate_variants kind=unit
    #[test]
    fn evaluation_error_makes_alternate_non_viable() {
        let mut m = Model { calls: vec![] };
        let run = evaluate_variants(&[slot("s", &["Bad", "Heavy"])], &mut m).unwrap();
        assert_eq!(
            run.variants[0].outcome,
            VariantOutcome::NonViable {
                reason: "E007: lookup miss".into()
            }
        );
        assert!(matches!(
            run.variants[1].outcome,
            VariantOutcome::Evaluated { .. }
        ));
    }

    // frob:tests crates/cadetta-variants/src/evaluate.rs::Evaluate kind=unit
    #[test]
    fn solver_output_unknown_under_variant_not_borrowed() {
        let mut m = Model { calls: vec![] };
        let run = evaluate_variants(&[slot("m", &["Strong"])], &mut m).unwrap();
        assert_eq!(run.by_member()["solved"]["m=Strong"], Reading::Unknown);
    }
}
