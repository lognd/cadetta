// frob:ticket 01M4SPWNHPTRETQBYCM6EA2GEV
//! Kinds, the kind algebra table and the Count/Ratio/angle rules
//! (quantities.md 4.1-4.3, language.md 3.10). Diagnostics are returned as
//! values: errors as [`KindError`], warnings as [`KindWarning`].

use std::collections::BTreeMap;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed};
use thiserror::Error;
use tracing::{debug, trace, warn};

use crate::dimension::{Dim, Dimension};
use crate::rational::{Magnitude, Rational};
use crate::units::{ResolvedUnit, UnitError, UnitKind, UnitTable};

/// Built-in named kinds with the unit expression giving their canonical dimension and
/// angle exponent (quantities.md 4.3: Torque -1, Lead -1, Angle and AngularVelocity +1).
/// Count and Ratio are dimensionless and handled separately; Money is per currency.
const BUILTIN_KINDS: &[(&str, &str)] = &[
    ("Length", "m"),
    ("Angle", "rad"),
    ("Area", "m^2"),
    ("Volume", "m^3"),
    ("Mass", "kg"),
    ("Force", "N"),
    ("Torque", "N*m/rad"),
    ("Energy", "J"),
    ("Power", "W"),
    ("Pressure", "Pa"),
    ("Time", "s"),
    ("Frequency", "Hz"),
    ("Velocity", "m/s"),
    ("VolumeFlow", "m^3/s"),
    ("Acceleration", "m/s^2"),
    ("AngularVelocity", "rad/s"),
    ("Lead", "m/rad"),
    ("Current", "A"),
    ("Voltage", "V"),
    ("Resistance", "ohm"),
    ("Inertia", "kg*m^2"),
    ("Density", "kg/m^3"),
    ("Temperature", "K"),
    ("TemperatureDelta", "K"),
];

/// Kind algebra table rows for `*` (commutative): (left, right, result).
const MUL_TABLE: &[(&str, &str, &str)] = &[
    ("Length", "Length", "Area"),
    ("Area", "Length", "Volume"),
    ("Mass", "Acceleration", "Force"),
];

/// Kind algebra table rows for `/`: (numerator, denominator, result).
const DIV_TABLE: &[(&str, &str, &str)] = &[
    ("Length", "Time", "Velocity"),
    ("Velocity", "Time", "Acceleration"),
    ("Force", "Area", "Pressure"),
    ("Mass", "Volume", "Density"),
    ("Volume", "Time", "VolumeFlow"),
    ("Energy", "Time", "Power"),
    ("Angle", "Time", "AngularVelocity"),
];

/// Diagnostics (errors) owned by the kind layer, each tagged with its code.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KindError {
    /// Q002: operands or target differ in dimension.
    #[error("Q002 dimension-mismatch: {left} vs {right}")]
    DimensionMismatch {
        /// Rendering of the left or found dimension.
        left: String,
        /// Rendering of the right or expected dimension.
        right: String,
    },
    /// Q003: two distinct named kinds never convert implicitly.
    #[error("Q003 kind-mismatch: {found} is not {expected} (use `as {expected}`)")]
    KindMismatch {
        /// Kind of the value.
        found: String,
        /// Kind required.
        expected: String,
    },
    /// Q009: a Count must be an integer.
    #[error("Q009 non-integer-count: {0} is not an integer")]
    NonIntegerCount(String),
    /// Q011: the dimension does not divide by the root index.
    #[error("Q011 root-of-dimension: cannot take root {index} of {dimension}")]
    RootOfDimension {
        /// Root index (2 for `sqrt`, 3 for `cbrt`).
        index: i32,
        /// Rendering of the offending dimension.
        dimension: String,
    },
    /// A kind name that is not registered.
    #[error("unknown kind `{0}`")]
    UnknownKind(String),
    /// A kind name registered twice.
    #[error("kind `{0}` is already declared")]
    DuplicateKind(String),
    /// A malformed kind expression (`Power / Area`).
    #[error("bad kind expression `{0}`")]
    BadExpression(String),
    /// Rounding to a Count needs an exact rational (no `pi` factor).
    #[error("Q006 inexact-value: cannot round a value carrying pi^{0} to a Count")]
    InexactRounding(i32),
    /// A unit expression failed to resolve.
    #[error(transparent)]
    Unit(#[from] UnitError),
}

impl KindError {
    /// The spec diagnostic code, when the error has one.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::DimensionMismatch { .. } => Some("Q002"),
            Self::KindMismatch { .. } => Some("Q003"),
            Self::InexactRounding(_) => Some("Q006"),
            Self::NonIntegerCount(_) => Some("Q009"),
            Self::RootOfDimension { .. } => Some("Q011"),
            _ => None,
        }
    }
}

/// Warnings raised by assignment and comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KindWarning {
    /// Q008 angle-dropped: the angle exponent differs from the target kind's.
    AngleDropped {
        /// Angle exponent of the value.
        found: i32,
        /// Canonical angle exponent of the target.
        expected: i32,
    },
}

impl KindWarning {
    /// The spec diagnostic code.
    pub fn code(&self) -> &'static str {
        "Q008"
    }
}

/// A kind with its dimension and angle exponent: the type of a quantity minus magnitude.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KindedDim {
    /// Named kind or anonymous.
    pub kind: UnitKind,
    /// Dimension and angle exponent.
    pub dim: Dim,
}

impl KindedDim {
    /// An anonymous kind of the given dimension and angle.
    pub fn anonymous(dim: Dim) -> Self {
        Self {
            kind: UnitKind::Anonymous,
            dim,
        }
    }

    /// The kind name, when named.
    pub fn name(&self) -> Option<&str> {
        match &self.kind {
            UnitKind::Named(n) => Some(n),
            UnitKind::Anonymous => None,
        }
    }

    fn is_named(&self, n: &str) -> bool {
        self.name() == Some(n)
    }

    /// Dimensionless with angle exponent 0: a pure scale factor.
    fn is_scalar(&self) -> bool {
        self.dim.dimension.is_dimensionless() && self.dim.angle == 0
    }

    fn label(&self) -> String {
        self.name().unwrap_or("anonymous").to_string()
    }
}

/// Result of an assignment or combination: the stored type plus any warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assigned {
    /// The type after assignment (target kind, canonical angle).
    pub value: KindedDim,
    /// Warnings raised (Q008).
    pub warnings: Vec<KindWarning>,
}

/// Rounding functions yielding a Count (quantities.md 4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    /// Toward negative infinity.
    Floor,
    /// Toward positive infinity.
    Ceil,
    /// Nearest, ties away from zero.
    Round,
}

/// Registry of kinds: built-ins plus package `kind` declarations (language.md 3.10).
#[derive(Debug, Clone)]
pub struct KindTable {
    kinds: BTreeMap<String, Dim>,
}

impl Default for KindTable {
    fn default() -> Self {
        Self::builtin()
    }
}

impl KindTable {
    /// The built-in kinds with canonical dimension and angle exponent.
    pub fn builtin() -> Self {
        let units = UnitTable::builtin();
        let mut kinds = BTreeMap::new();
        for (name, expr) in BUILTIN_KINDS {
            let r = units
                .parse(expr)
                .expect("built-in kind expression resolves");
            kinds.insert((*name).to_string(), r.dim);
        }
        kinds.insert("Ratio".to_string(), Dim::default());
        kinds.insert("Count".to_string(), Dim::default());
        debug!(count = kinds.len(), "built-in kind table ready");
        Self { kinds }
    }

    /// Canonical dimension and angle of a registered kind.
    pub fn dim_of(&self, name: &str) -> Option<&Dim> {
        self.kinds.get(name)
    }

    /// The named type of a registered kind.
    pub fn named(&self, name: &str) -> Result<KindedDim, KindError> {
        let dim = self
            .kinds
            .get(name)
            .ok_or_else(|| KindError::UnknownKind(name.to_string()))?;
        Ok(KindedDim {
            kind: UnitKind::Named(name.to_string()),
            dim: dim.clone(),
        })
    }

    /// Type of a literal in a resolved unit: its default kind at the kind's canonical angle
    /// (quantities.md 4.1 rules 1 and 5), so a Torque literal already carries angle -1.
    pub fn from_unit(&self, unit: &ResolvedUnit) -> KindedDim {
        match &unit.kind {
            UnitKind::Named(n) if self.kinds.contains_key(n) => {
                let mut dim = unit.dim.clone();
                dim.angle = self.kinds[n].angle;
                KindedDim {
                    kind: unit.kind.clone(),
                    dim,
                }
            }
            _ => KindedDim::anonymous(unit.dim.clone()),
        }
    }

    /// Register `kind name = Existing` (a distinct kind of the same dimension) or
    /// `kind name = A / B` for a kind expression (language.md 3.10).
    pub fn register(&mut self, name: &str, expr: &str) -> Result<(), KindError> {
        if self.kinds.contains_key(name) {
            return Err(KindError::DuplicateKind(name.to_string()));
        }
        let dim = self.eval_expr(expr)?.dim;
        debug!(name, expr, "registered kind");
        self.kinds.insert(name.to_string(), dim);
        Ok(())
    }

    /// Evaluate a kind expression (`Power / Area`, `Length ^ 2`, `Volume / Time`) left to right.
    pub fn eval_expr(&self, expr: &str) -> Result<KindedDim, KindError> {
        let bad = || KindError::BadExpression(expr.to_string());
        let mut acc: Option<KindedDim> = None;
        let mut op = '*';
        let mut rest = expr.trim();
        while !rest.is_empty() {
            let end = rest.find(['*', '/']).unwrap_or(rest.len());
            let (term, tail) = rest.split_at(end);
            let (base, pow) = match term.split_once('^') {
                Some((b, p)) => (b.trim(), p.trim().parse::<i32>().map_err(|_| bad())?),
                None => (term.trim(), 1),
            };
            if base.is_empty() || !base.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(bad());
            }
            let t = self.powi(&self.named(base)?, pow);
            acc = Some(match acc {
                None if op == '*' => t,
                None => return Err(bad()),
                Some(a) if op == '*' => self.mul(&a, &t),
                Some(a) => self.div(&a, &t),
            });
            let mut chars = tail.chars();
            match chars.next() {
                Some(c) => {
                    op = c;
                    rest = chars.as_str().trim();
                    if rest.is_empty() {
                        return Err(bad());
                    }
                }
                None => rest = "",
            }
        }
        acc.ok_or_else(bad)
    }

    /// Name the result of a product or quotient when the table has a row and the result
    /// dimension matches that kind's canonical dimension; anonymous otherwise.
    fn finish(&self, dim: Dim, row: Option<&str>) -> KindedDim {
        match row {
            Some(n) if self.kinds.get(n) == Some(&dim) => {
                trace!(kind = n, "kind algebra table hit");
                KindedDim {
                    kind: UnitKind::Named(n.to_string()),
                    dim,
                }
            }
            _ => KindedDim::anonymous(dim),
        }
    }

    /// The kind kept when `a` is scaled by dimensionless `s` (rule 3). A Count survives only
    /// scaling by a Count; scaled by any other scalar it becomes a Ratio.
    fn scaled_kind(&self, a: &KindedDim, s: &KindedDim, dim: Dim) -> KindedDim {
        let kind = match a.name() {
            Some("Count") if !s.is_named("Count") => UnitKind::Named("Ratio".to_string()),
            Some(_) => a.kind.clone(),
            None => UnitKind::Anonymous,
        };
        KindedDim { kind, dim }
    }

    /// Product kind (quantities.md 4.1 rule 3).
    pub fn mul(&self, a: &KindedDim, b: &KindedDim) -> KindedDim {
        let dim = a.dim.mul(&b.dim);
        let out = if b.is_scalar() && a.name().is_some() {
            self.scaled_kind(a, b, dim)
        } else if a.is_scalar() && b.name().is_some() {
            self.scaled_kind(b, a, dim)
        } else {
            let row = match (a.name(), b.name()) {
                (Some(x), Some(y)) => MUL_TABLE
                    .iter()
                    .find(|(l, r, _)| (*l == x && *r == y) || (*l == y && *r == x))
                    .map(|r| r.2),
                _ => None,
            };
            self.finish(dim, row)
        };
        trace!(left = %a.label(), right = %b.label(), result = %out.label(), "mul kind");
        out
    }

    /// Quotient kind (quantities.md 4.1 rule 3).
    pub fn div(&self, a: &KindedDim, b: &KindedDim) -> KindedDim {
        let dim = a.dim.div(&b.dim);
        let out = if b.is_scalar() && a.name().is_some() {
            self.scaled_kind(a, b, dim)
        } else {
            let row = match (a.name(), b.name()) {
                (Some(x), Some(y)) => DIV_TABLE
                    .iter()
                    .find(|(l, r, _)| *l == x && *r == y)
                    .map(|r| r.2),
                _ => None,
            };
            self.finish(dim, row)
        };
        trace!(left = %a.label(), right = %b.label(), result = %out.label(), "div kind");
        out
    }

    /// Integer power `x ^ n` (quantities.md 4.1 rule 3): `Length^2` is Area, `Length^3` Volume.
    pub fn powi(&self, a: &KindedDim, n: i32) -> KindedDim {
        let dim = a.dim.powi(n);
        match (a.name(), n) {
            (_, 1) => a.clone(),
            (Some("Length"), 2) => self.finish(dim, Some("Area")),
            (Some("Length"), 3) => self.finish(dim, Some("Volume")),
            _ => KindedDim::anonymous(dim),
        }
    }

    /// `sqrt`/`cbrt` kind (index 2 or 3): Area to Length and Volume to Length, else anonymous;
    /// Q011 when the dimension (or angle exponent) does not divide.
    pub fn root(&self, a: &KindedDim, index: i32) -> Result<KindedDim, KindError> {
        let divisible = a.dim.angle % index == 0;
        let dimension = a
            .dim
            .dimension
            .root(index)
            .filter(|_| divisible)
            .ok_or_else(|| {
                debug!(index, dim = %a.dim.dimension.render(), "root of dimension");
                KindError::RootOfDimension {
                    index,
                    dimension: a.dim.dimension.render(),
                }
            })?;
        let dim = Dim::new(dimension, a.dim.angle / index);
        let row = match (a.name(), index) {
            (Some("Area"), 2) | (Some("Volume"), 3) => Some("Length"),
            _ => None,
        };
        Ok(self.finish(dim, row))
    }

    /// `+ - min max` and comparisons (rule 2): equal kinds, or a named and an anonymous kind of
    /// equal dimension and angle (result named). An angle disagreement is Q008, not an error.
    pub fn combine(&self, a: &KindedDim, b: &KindedDim) -> Result<Assigned, KindError> {
        a.dim
            .check_same_dimension(&b.dim)
            .map_err(|_| KindError::DimensionMismatch {
                left: a.dim.dimension.render(),
                right: b.dim.dimension.render(),
            })?;
        let value = match (a.name(), b.name()) {
            (Some(x), Some(y)) if x != y => {
                return Err(KindError::KindMismatch {
                    found: y.into(),
                    expected: x.into(),
                });
            }
            (Some(_), _) => a.clone(),
            (None, Some(_)) => b.clone(),
            (None, None) => a.clone(),
        };
        let mut warnings = Vec::new();
        if a.dim.angle != b.dim.angle {
            warn!(
                left = a.dim.angle,
                right = b.dim.angle,
                "Q008 angle-dropped in combine"
            );
            warnings.push(KindWarning::AngleDropped {
                found: b.dim.angle,
                expected: a.dim.angle,
            });
        }
        let mut value = value;
        if let Some(n) = value.name().map(str::to_string) {
            value.dim.angle = self.kinds.get(&n).map_or(value.dim.angle, |d| d.angle);
        }
        Ok(Assigned { value, warnings })
    }

    /// Assign `value` to a parameter of kind `target` (rule 4 and 5, 4.3).
    ///
    /// Anonymous kinds and equal kinds are accepted, and a Count to a Ratio. Any other named
    /// kind is Q003. A differing angle exponent warns Q008, except an exponent-0 value assigned
    /// to Torque (lever arm). The result carries the target's canonical angle.
    pub fn assign(&self, value: &KindedDim, target: &str) -> Result<Assigned, KindError> {
        let t = self.named(target)?;
        value
            .dim
            .check_same_dimension(&t.dim)
            .map_err(|_| KindError::DimensionMismatch {
                left: value.dim.dimension.render(),
                right: t.dim.dimension.render(),
            })?;
        match value.name() {
            None => {}
            Some(n) if n == target => {}
            Some("Count") if target == "Ratio" => {}
            Some(n) => {
                debug!(found = n, expected = target, "Q003 kind mismatch");
                return Err(KindError::KindMismatch {
                    found: n.into(),
                    expected: target.into(),
                });
            }
        }
        let mut warnings = Vec::new();
        let lever_arm = target == "Torque" && value.dim.angle == 0;
        if value.dim.angle != t.dim.angle && !lever_arm {
            warn!(
                found = value.dim.angle,
                expected = t.dim.angle,
                target,
                "Q008 angle-dropped"
            );
            warnings.push(KindWarning::AngleDropped {
                found: value.dim.angle,
                expected: t.dim.angle,
            });
        }
        Ok(Assigned { value: t, warnings })
    }

    /// Explicit `x as Kind` (rule 5): dimension must match; the angle exponent may change
    /// silently and the result takes the target's canonical angle.
    pub fn cast(&self, value: &KindedDim, target: &str) -> Result<KindedDim, KindError> {
        let t = self.named(target)?;
        value
            .dim
            .check_same_dimension(&t.dim)
            .map_err(|_| KindError::DimensionMismatch {
                left: value.dim.dimension.render(),
                right: t.dim.dimension.render(),
            })?;
        Ok(t)
    }

    /// Q009: require an integer value for a Count and return the Count type.
    pub fn count(&self, value: &Rational) -> Result<KindedDim, KindError> {
        if !value.denom().is_one() {
            debug!(%value, "Q009 non-integer count");
            return Err(KindError::NonIntegerCount(value.to_string()));
        }
        self.named("Count")
    }

    /// `floor`/`ceil`/`round` of a dimensionless value: an integer Count. Ties round away from
    /// zero. The value must be dimensionless with angle 0 (Q002) and carry no `pi` factor (Q006).
    pub fn round_to_count(
        &self,
        ty: &KindedDim,
        mag: &Magnitude,
        mode: Rounding,
    ) -> Result<(Rational, KindedDim), KindError> {
        if !ty.is_scalar() {
            return Err(KindError::DimensionMismatch {
                left: ty.dim.dimension.render(),
                right: Dimension::dimensionless().render(),
            });
        }
        if mag.pi_exp() != 0 && !mag.is_zero() {
            return Err(KindError::InexactRounding(mag.pi_exp()));
        }
        let r = mag.rational();
        let (n, d) = (r.numer(), r.denom());
        let q = match mode {
            Rounding::Floor => n.div_floor(d),
            Rounding::Ceil => -((-n).div_floor(d)),
            Rounding::Round => {
                // Half away from zero: floor(|n|/d + 1/2) with the original sign.
                let two_d: BigInt = d * 2;
                let twice: BigInt = n.abs() * 2 + d;
                let mag_q = twice.div_floor(&two_d);
                if n.is_negative() { -mag_q } else { mag_q }
            }
        };
        let out = Rational::from_int(q);
        Ok((out, self.named("Count")?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(unit: &str) -> KindedDim {
        KindTable::builtin().from_unit(&UnitTable::builtin().parse(unit).unwrap())
    }

    fn k(name: &str) -> KindedDim {
        KindTable::builtin().named(name).unwrap()
    }

    fn mag(n: i64, d: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, d).unwrap(), 0)
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.mul kind=unit
    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.div kind=unit
    #[test]
    fn algebra_table_rows() {
        let t = KindTable::builtin();
        let mul = [
            ("Length", "Length", "Area"),
            ("Area", "Length", "Volume"),
            ("Length", "Area", "Volume"),
            ("Mass", "Acceleration", "Force"),
        ];
        for (a, b, r) in mul {
            assert_eq!(t.mul(&k(a), &k(b)), k(r), "{a}*{b}");
        }
        let div = [
            ("Length", "Time", "Velocity"),
            ("Velocity", "Time", "Acceleration"),
            ("Force", "Area", "Pressure"),
            ("Mass", "Volume", "Density"),
            ("Volume", "Time", "VolumeFlow"),
            ("Energy", "Time", "Power"),
            ("Angle", "Time", "AngularVelocity"),
        ];
        for (a, b, r) in div {
            assert_eq!(t.div(&k(a), &k(b)), k(r), "{a}/{b}");
        }
        let p = t.powi(&k("Length"), 2);
        assert_eq!(p, k("Area"));
        assert_eq!(t.powi(&k("Length"), 3), k("Volume"));
        assert_eq!(t.powi(&k("Length"), 4).kind, UnitKind::Anonymous);
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.div kind=unit
    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.mul kind=unit
    #[test]
    fn named_scaled_by_dimensionless_keeps_kind() {
        let t = KindTable::builtin();
        let two = KindedDim::anonymous(Dim::default());
        assert_eq!(t.div(&k("Length"), &two), k("Length"));
        assert_eq!(t.mul(&two, &k("Length")), k("Length"));
        assert_eq!(t.mul(&k("Length"), &k("Ratio")), k("Length"));
        assert_eq!(t.mul(&k("Count"), &k("Count")), k("Count"));
        // A Count scaled by a non-Count is not an integer: Ratio.
        assert_eq!(t.mul(&k("Count"), &two), k("Ratio"));
        // Scaling an anonymous value stays anonymous.
        assert_eq!(t.mul(&two, &two).kind, UnitKind::Anonymous);
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.mul kind=unit
    #[test]
    fn force_times_length_is_anonymous() {
        let t = KindTable::builtin();
        let fl = t.mul(&k("Force"), &k("Length"));
        assert_eq!(fl.kind, UnitKind::Anonymous);
        assert_eq!(fl.dim, k("Energy").dim);
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.root kind=unit
    #[test]
    fn roots() {
        let t = KindTable::builtin();
        assert_eq!(t.root(&k("Area"), 2).unwrap(), k("Length"));
        assert_eq!(t.root(&k("Volume"), 3).unwrap(), k("Length"));
        let e = t.root(&k("Length"), 2).unwrap_err();
        assert_eq!(e.code(), Some("Q011"));
        assert!(matches!(
            t.root(&k("Area"), 3),
            Err(KindError::RootOfDimension { .. })
        ));
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.assign kind=unit
    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.cast kind=unit
    #[test]
    fn torque_to_energy_needs_as() {
        let t = KindTable::builtin();
        let torque = lit("N*m");
        let e = t.assign(&torque, "Energy").unwrap_err();
        assert_eq!(e.code(), Some("Q003"));
        assert_eq!(t.cast(&torque, "Energy").unwrap(), k("Energy"));
        // `as` never changes the dimension.
        assert_eq!(t.cast(&torque, "Length").unwrap_err().code(), Some("Q002"));
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.assign kind=unit
    #[test]
    fn anonymous_assigns_to_named() {
        let t = KindTable::builtin();
        let v = t.div(&k("Force"), &k("Area"));
        let anon = KindedDim::anonymous(v.dim.clone());
        let ok = t.assign(&anon, "Pressure").unwrap();
        assert_eq!(ok.value, k("Pressure"));
        assert!(ok.warnings.is_empty());
        assert_eq!(t.assign(&anon, "Length").unwrap_err().code(), Some("Q002"));
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.combine kind=unit
    #[test]
    fn additive_rules() {
        let t = KindTable::builtin();
        let anon = KindedDim::anonymous(k("Length").dim);
        assert_eq!(t.combine(&k("Length"), &anon).unwrap().value, k("Length"));
        assert_eq!(t.combine(&anon, &k("Length")).unwrap().value, k("Length"));
        assert_eq!(
            t.combine(&k("Torque"), &k("Energy")).unwrap_err().code(),
            Some("Q003")
        );
        assert_eq!(
            t.combine(&k("Torque"), &k("Length")).unwrap_err().code(),
            Some("Q002")
        );
        assert_eq!(
            t.combine(&k("Pressure"), &k("Pressure")).unwrap().value,
            k("Pressure")
        );
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.count kind=unit
    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.round_to_count kind=unit
    #[test]
    fn count_and_rounding() {
        let t = KindTable::builtin();
        let half = Rational::new(1, 2).unwrap();
        assert_eq!(t.count(&half).unwrap_err().code(), Some("Q009"));
        assert_eq!(t.count(&Rational::from_int(3)).unwrap(), k("Count"));
        let ratio = k("Ratio");
        let cases = [
            (Rounding::Floor, 5, 2, 2),
            (Rounding::Floor, -5, 2, -3),
            (Rounding::Ceil, 5, 2, 3),
            (Rounding::Ceil, -5, 2, -2),
            (Rounding::Round, 5, 2, 3),
            (Rounding::Round, -5, 2, -3),
            (Rounding::Round, 7, 3, 2),
            (Rounding::Round, -7, 3, -2),
            (Rounding::Round, 1, 3, 0),
            (Rounding::Round, 4, 1, 4),
        ];
        for (mode, n, d, want) in cases {
            let (r, ty) = t.round_to_count(&ratio, &mag(n, d), mode).unwrap();
            assert_eq!(r, Rational::from_int(want), "{mode:?} {n}/{d}");
            assert_eq!(ty, k("Count"));
        }
        let e = t
            .round_to_count(&k("Length"), &mag(1, 2), Rounding::Round)
            .unwrap_err();
        assert_eq!(e.code(), Some("Q002"));
        let pi = Magnitude::new(Rational::one(), 1);
        assert_eq!(
            t.round_to_count(&ratio, &pi, Rounding::Floor)
                .unwrap_err()
                .code(),
            Some("Q006")
        );
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.assign kind=unit
    #[test]
    fn count_to_ratio_one_way() {
        let t = KindTable::builtin();
        assert_eq!(t.assign(&k("Count"), "Ratio").unwrap().value, k("Ratio"));
        assert_eq!(
            t.assign(&k("Ratio"), "Count").unwrap_err().code(),
            Some("Q003")
        );
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.register kind=unit
    #[test]
    fn registered_kinds_are_distinct() {
        let mut t = KindTable::builtin();
        t.register("Stress", "Pressure").unwrap();
        t.register("HeatFlux", "Power / Area").unwrap();
        let stress = t.named("Stress").unwrap();
        let heat = t.named("HeatFlux").unwrap();
        assert_eq!(stress.dim, k("Pressure").dim);
        assert_eq!(heat.dim, t.div(&k("Power"), &k("Area")).dim);
        assert_ne!(stress, k("Pressure"));
        assert_eq!(
            t.assign(&stress, "Pressure").unwrap_err().code(),
            Some("Q003")
        );
        assert_eq!(
            t.assign(&k("Pressure"), "Stress").unwrap_err().code(),
            Some("Q003")
        );
        assert_eq!(
            t.register("Stress", "Pressure"),
            Err(KindError::DuplicateKind("Stress".into()))
        );
        assert!(matches!(
            t.register("X", "Nope"),
            Err(KindError::UnknownKind(_))
        ));
        assert!(matches!(
            t.register("X", "Power /"),
            Err(KindError::BadExpression(_))
        ));
        // The Power / Area result (anonymous) is assignable to HeatFlux.
        let anon = KindedDim::anonymous(heat.dim.clone());
        assert!(t.assign(&anon, "HeatFlux").unwrap().warnings.is_empty());
    }

    fn angle_case(t: &KindTable, v: &KindedDim, target: &str) -> Vec<KindWarning> {
        t.assign(v, target).unwrap().warnings
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.assign kind=unit
    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.from_unit kind=unit
    #[test]
    fn angle_worked_cases() {
        let t = KindTable::builtin();
        let rpm = lit("rpm");
        // 1500rpm * 8mm/rev to Velocity: exponent 0, clean.
        let v = t.mul(&rpm, &lit("mm/rev"));
        assert_eq!(v.dim.angle, 0);
        assert!(angle_case(&t, &v, "Velocity").is_empty());
        // 1500rpm * 8mm to Velocity: exponent +1, Q008.
        let v = t.mul(&rpm, &lit("mm"));
        assert_eq!(v.dim.angle, 1);
        let w = angle_case(&t, &v, "Velocity");
        assert_eq!(
            w,
            vec![KindWarning::AngleDropped {
                found: 1,
                expected: 0
            }]
        );
        assert_eq!(w[0].code(), "Q008");
        // 0.45N*m * eta / 8mm/rev to Force: exponent 0, clean.
        let eta = k("Ratio");
        let torque = t.mul(&lit("N*m"), &eta);
        assert_eq!(torque, k("Torque"));
        let f = t.div(&torque, &lit("mm/rev"));
        assert_eq!(f.dim.angle, 0);
        assert!(angle_case(&t, &f, "Force").is_empty());
        // Same over a Length lead: exponent -1, Q008.
        let f = t.div(&torque, &lit("mm"));
        assert_eq!(f.dim.angle, -1);
        assert_eq!(
            angle_case(&t, &f, "Force"),
            vec![KindWarning::AngleDropped {
                found: -1,
                expected: 0
            }]
        );
        // 50N * 9mm to Torque: lever-arm exception, clean, stored with exponent -1.
        let fl = t.mul(&lit("N"), &lit("mm"));
        let a = t.assign(&fl, "Torque").unwrap();
        assert!(a.warnings.is_empty());
        assert_eq!(a.value.dim.angle, -1);
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.assign kind=unit
    #[test]
    fn lever_arm_exception_is_exponent_zero_only() {
        let t = KindTable::builtin();
        let mut v = t.mul(&k("Force"), &k("Length"));
        v.dim.angle = 1;
        assert_eq!(angle_case(&t, &v, "Torque").len(), 1);
    }

    // frob:tests crates/cadetta-units/src/kind.rs::KindTable.builtin kind=unit
    #[test]
    fn builtin_canonical_angles() {
        let t = KindTable::builtin();
        for (n, a) in [
            ("Angle", 1),
            ("AngularVelocity", 1),
            ("Lead", -1),
            ("Torque", -1),
            ("Length", 0),
        ] {
            assert_eq!(t.dim_of(n).unwrap().angle, a, "{n}");
        }
    }
}
