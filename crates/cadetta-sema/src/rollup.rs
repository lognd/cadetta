// frob:ticket 01M4ST7AKENKKS96TTXY13YMZQ
//! Roll-ups over every part instance: `sum`, `max` and `min` (language.md 3.5).
//!
//! `sum(self, mass)` and `max(self, lead_time)` walk every instance of every
//! part, recursively, so an array of four contributes four times. The `?`
//! forms skip parts that lack the parameter and list them in the result note;
//! the plain forms fail on the first lack. The module is pure: the evaluator
//! flattens the part tree into [`Contribution`]s and calls [`roll_up`].

use std::cmp::Ordering;
use std::fmt;

use cadetta_units::Sum;
use tracing::{debug, trace, warn};

use cadetta_syntax::ast::{Arg, Expr, ExprKind};

use crate::eval::Quant;

/// The roll-up parameter names, exactly the members of `std::commercial::Sourced`.
pub const ROLLUP_NAMES: &[&str] = &["unit_cost", "lead_time", "moq", "supplier", "mass"];

/// The roll-up a call expression spells, if any: `sum|max|min(self, m)` or `m(self)`.
// frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
pub fn shape(callee: &Expr, args: &[Arg]) -> Option<(RollOp, String)> {
    let ExprKind::Path(cp) = &callee.kind else {
        return None;
    };
    let name = cp.segments.last()?.name.as_str();
    let is_self = |a: &Arg| matches!(&a.value.kind, ExprKind::Path(p) if p.text() == "self");
    let op = match name {
        "sum" => Some(RollOp::Sum),
        "max" => Some(RollOp::Max),
        "min" => Some(RollOp::Min),
        _ => None,
    };
    match (args, op) {
        ([a], None) if ROLLUP_NAMES.contains(&name) && is_self(a) => {
            Some((RollOp::Sum, name.to_owned()))
        }
        ([a, b], Some(op)) if is_self(a) => match &b.value.kind {
            ExprKind::Path(p) if p.segments.len() == 1 => Some((op, p.text())),
            _ => None,
        },
        _ => None,
    }
}

/// The roll-up over one part array a call spells, if any: `sum|max|min(slot.member)`.
// frob:ticket 01M4W5HZZQ36F4WN08B1EBM7ZH
pub fn slot_shape<'a>(callee: &Expr, args: &'a [Arg]) -> Option<(RollOp, &'a Expr)> {
    let ExprKind::Path(cp) = &callee.kind else {
        return None;
    };
    let op = match cp.segments.last()?.name.as_str() {
        "sum" => RollOp::Sum,
        "max" => RollOp::Max,
        "min" => RollOp::Min,
        _ => return None,
    };
    match args {
        [a] if a.name.is_none()
            && matches!(&a.value.kind, ExprKind::Path(_) | ExprKind::Field(..)) =>
        {
            Some((op, &a.value))
        }
        _ => None,
    }
}

/// Which reduction a roll-up applies.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RollOp {
    /// `sum(...)`, `mass(...)`.
    Sum,
    /// `max(...)`.
    Max,
    /// `min(...)`.
    Min,
}

impl RollOp {
    /// The source-level function name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sum => "sum",
            Self::Max => "max",
            Self::Min => "min",
        }
    }
}

/// One part instance's contribution: its path and its value, if it has the parameter.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Contribution {
    /// The instance path, e.g. `Rover/wheel[2]`.
    pub path: String,
    /// The parameter value; `None` when the part lacks the parameter.
    pub value: Option<Quant>,
}

/// Why a roll-up failed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RollupError {
    /// A part lacks the parameter and the roll-up was not the `?` form.
    Missing {
        /// The parameter.
        param: String,
        /// The first part lacking it.
        path: String,
    },
    /// No part supplied a value, so there is nothing to reduce.
    Empty(String),
    /// Two values do not share a dimension, or exact addition failed.
    Incompatible(String),
}

impl fmt::Display for RollupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { param, path } => {
                write!(f, "`{path}` has no `{param}` (use the `?` form to skip it)")
            }
            Self::Empty(p) => write!(f, "no part supplies `{p}`"),
            Self::Incompatible(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for RollupError {}

/// The outcome of a roll-up.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rollup {
    /// The reduced value.
    pub value: Quant,
    /// Paths of parts that lacked the parameter (only for `?` forms).
    pub missing: Vec<String>,
    /// Number of instances that contributed.
    pub counted: usize,
}

impl Rollup {
    /// The result note listing missing parts, if any.
    pub fn note(&self, param: &str) -> Option<String> {
        if self.missing.is_empty() {
            return None;
        }
        Some(format!(
            "{} part(s) lack `{param}`: {}",
            self.missing.len(),
            self.missing.join(", ")
        ))
    }
}

fn cmp_nominal(a: &Quant, b: &Quant) -> Ordering {
    let d = match a.nominal.sub(&b.nominal) {
        Sum::Exact(m) => m.to_f64(),
        Sum::Inexact(f) => f,
    };
    d.partial_cmp(&0.0).unwrap_or(Ordering::Equal)
}

/// Reduces `items` with `op` over `param`; `optional` is the `?` form.
pub fn roll_up(
    op: RollOp,
    param: &str,
    optional: bool,
    items: &[Contribution],
) -> Result<Rollup, RollupError> {
    let mut acc: Option<Quant> = None;
    let mut missing = Vec::new();
    let mut counted = 0usize;
    for it in items {
        let Some(v) = &it.value else {
            if !optional {
                warn!(param, path = %it.path, "roll-up part lacks parameter");
                return Err(RollupError::Missing {
                    param: param.to_owned(),
                    path: it.path.clone(),
                });
            }
            trace!(param, path = %it.path, "roll-up skips part");
            missing.push(it.path.clone());
            continue;
        };
        counted += 1;
        acc = Some(match acc {
            None => v.clone(),
            Some(a) => {
                if a.dim() != v.dim() {
                    return Err(RollupError::Incompatible(format!(
                        "`{}` of `{param}` has a different dimension from earlier parts",
                        it.path
                    )));
                }
                match op {
                    RollOp::Sum => {
                        let form = a
                            .form
                            .add(&v.form)
                            .map_err(|e| RollupError::Incompatible(e.to_string()))?;
                        let nominal = match a.nominal.add(&v.nominal) {
                            Sum::Exact(m) => m,
                            Sum::Inexact(_) => {
                                return Err(RollupError::Incompatible(
                                    "sum of different powers of pi is inexact".into(),
                                ));
                            }
                        };
                        Quant { form, nominal }
                    }
                    RollOp::Max => match cmp_nominal(v, &a) {
                        Ordering::Greater => v.clone(),
                        _ => a,
                    },
                    RollOp::Min => match cmp_nominal(v, &a) {
                        Ordering::Less => v.clone(),
                        _ => a,
                    },
                }
            }
        });
    }
    let value = acc.ok_or_else(|| RollupError::Empty(param.to_owned()))?;
    debug!(
        op = op.name(),
        param,
        counted,
        missing = missing.len(),
        "roll-up done"
    );
    Ok(Rollup {
        value,
        missing,
        counted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_units::{Dim, Magnitude, Rational};

    fn q(n: i64) -> Quant {
        Quant::constant(Dim::default(), Magnitude::new(Rational::from_int(n), 0))
    }

    fn item(path: &str, v: Option<i64>) -> Contribution {
        Contribution {
            path: path.to_owned(),
            value: v.map(q),
        }
    }

    fn nominal(r: &Rollup) -> Magnitude {
        r.value.nominal.clone()
    }

    // frob:tests crates/cadetta-sema/src/rollup.rs::roll_up kind=unit
    #[test]
    fn sum_counts_every_instance_and_max_min_pick_extremes() {
        let items: Vec<_> = (0..4)
            .map(|i| item(&format!("w[{i}]"), Some(3)))
            .chain([item("frame", Some(10))])
            .collect();
        let s = roll_up(RollOp::Sum, "mass", false, &items).unwrap();
        assert_eq!(nominal(&s), Magnitude::new(Rational::from_int(22), 0));
        assert_eq!(s.counted, 5);
        let mx = roll_up(RollOp::Max, "lead_time", false, &items).unwrap();
        assert_eq!(nominal(&mx), Magnitude::new(Rational::from_int(10), 0));
        let mn = roll_up(RollOp::Min, "lead_time", false, &items).unwrap();
        assert_eq!(nominal(&mn), Magnitude::new(Rational::from_int(3), 0));
    }

    // frob:tests crates/cadetta-sema/src/rollup.rs::roll_up kind=unit
    #[test]
    fn missing_parameter_errors_unless_optional_which_lists_parts() {
        let items = vec![item("a", Some(1)), item("b", None), item("c", Some(2))];
        assert!(matches!(
            roll_up(RollOp::Sum, "mass", false, &items),
            Err(RollupError::Missing { ref path, .. }) if path == "b"
        ));
        let r = roll_up(RollOp::Sum, "mass", true, &items).unwrap();
        assert_eq!(nominal(&r), Magnitude::new(Rational::from_int(3), 0));
        assert_eq!(r.missing, vec!["b".to_owned()]);
        assert!(r.note("mass").unwrap().contains('b'));
        assert_eq!(
            roll_up(RollOp::Sum, "mass", true, &[item("x", None)]),
            Err(RollupError::Empty("mass".into()))
        );
    }
}
