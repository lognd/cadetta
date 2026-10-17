//! Datum transforms: exact 90deg multiples, inexact otherwise, toleranced angles (geometry.md 2).
//!
//! A datum's composed transform is a 3x4 matrix (a 3x3 rotation and a translation) built by
//! applying offsets and rotations left to right, each in the frame built so far. Every entry is
//! a [`Coord`]: either an exact affine form over the source variables of the tolerances that fed
//! it (quantities.md 5.4), or an [`Approx`] binary64 interval marked inexact.
//!
//! Decisions made here (written into geometry.md 2): only an untoleranced rotation by a whole
//! multiple of 90deg keeps the matrix exact (so 60deg or 30deg are inexact even though their
//! sine or cosine is rational); any other untoleranced angle gives point entries naming the
//! sin or cos root for Q006; a toleranced angle gives conservative interval entries, whose root
//! is only named when the nominal itself is not a multiple of 90deg; offsets stay exact affine
//! forms, and carrying them through an exact rotation keeps their source variables.

// frob:ticket 01M4SWFX4ZH1Q8BJPEEWYEKQJC

use cadetta_units::{Affine, Dim, Magnitude, Rational, RootOp};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};
use thiserror::Error;
use tracing::{debug, trace};

use crate::datum::Axis;

/// Why a transform could not be composed.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum TransformError {
    /// A tolerance operation failed (dimension or pi-exponent mismatch).
    #[error("tolerance: {0}")]
    Tolerance(#[from] cadetta_units::ToleranceError),
    /// A rotation angle was not an angle.
    #[error("rotation angle must be dimensionless-with-angle, found {0}")]
    NotAnAngle(String),
    /// Operands of a coordinate sum differ in dimension.
    #[error("dimension mismatch: {0} vs {1}")]
    DimensionMismatch(String, String),
}

/// A binary64 interval marked inexact; the Q006 root is kept when the nominal caused it.
#[derive(Debug, Clone, PartialEq)]
pub struct Approx {
    /// Lower bound.
    pub lo: f64,
    /// Upper bound.
    pub hi: f64,
    /// The operation that made the nominal inexact, if the nominal is inexact.
    pub root: Option<RootOp>,
    /// True when the interval is an outer bound rather than a single value.
    pub conservative: bool,
    /// The dimension of the quantity.
    pub dim: Dim,
}

/// One matrix entry or coordinate: an exact affine form, or an inexact interval.
#[derive(Debug, Clone, PartialEq)]
pub enum Coord {
    /// Exact `center + sum(c_i * e_i)`; tolerances stay source variables.
    Exact(Affine),
    /// Inexact binary64 interval.
    Approx(Approx),
}

fn pad(lo: f64, hi: f64) -> (f64, f64) {
    let p = |x: f64| 4.0 * f64::EPSILON * x.abs().max(f64::MIN_POSITIVE);
    (lo - p(lo), hi + p(hi))
}

fn mag(n: i64, d: i64) -> Magnitude {
    Magnitude::new(Rational::new(n, d).expect("nonzero denominator"), 0)
}

impl Coord {
    /// An exact constant (zero, one, ...) of the given dimension.
    pub fn constant(dim: Dim, n: i64) -> Self {
        Coord::Exact(Affine::constant(dim, mag(n, 1)))
    }

    /// The dimension of the quantity.
    pub fn dim(&self) -> &Dim {
        match self {
            Coord::Exact(a) => a.dim(),
            Coord::Approx(p) => &p.dim,
        }
    }

    /// True when no binary64 is involved (tolerances may remain as source variables).
    pub fn is_exact(&self) -> bool {
        matches!(self, Coord::Exact(_))
    }

    /// True when the value is an outer bound (a conservative interval or nonlinear fallback).
    pub fn is_conservative(&self) -> bool {
        match self {
            Coord::Exact(a) => a.is_conservative(),
            Coord::Approx(p) => p.conservative,
        }
    }

    /// The Q006 root, if the nominal is inexact.
    pub fn root(&self) -> Option<RootOp> {
        match self {
            Coord::Exact(_) => None,
            Coord::Approx(p) => p.root,
        }
    }

    fn is_exact_zero(&self) -> bool {
        matches!(self, Coord::Exact(a) if a.is_exact() && a.center().is_zero())
    }

    fn to_approx(&self) -> Result<Approx, TransformError> {
        match self {
            Coord::Approx(p) => Ok(p.clone()),
            Coord::Exact(a) => {
                let (lo, hi) = a.bounds()?;
                Ok(Approx {
                    lo: lo.to_f64(),
                    hi: hi.to_f64(),
                    root: None,
                    conservative: !a.is_exact() || a.is_conservative(),
                    dim: a.dim().clone(),
                })
            }
        }
    }

    /// Exact negation (an interval is mirrored).
    pub fn neg(&self) -> Coord {
        match self {
            Coord::Exact(a) => Coord::Exact(a.neg()),
            Coord::Approx(p) => Coord::Approx(Approx {
                lo: -p.hi,
                hi: -p.lo,
                ..p.clone()
            }),
        }
    }

    /// Sum; exact only when both operands are.
    pub fn add(&self, o: &Coord) -> Result<Coord, TransformError> {
        if let (Coord::Exact(a), Coord::Exact(b)) = (self, o) {
            return Ok(Coord::Exact(a.add(b)?));
        }
        if self.dim().dimension != o.dim().dimension {
            return Err(TransformError::DimensionMismatch(
                format!("{:?}", self.dim()),
                format!("{:?}", o.dim()),
            ));
        }
        let (a, b) = (self.to_approx()?, o.to_approx()?);
        Ok(Coord::Approx(approx_op(&a, &b, a.dim.clone(), |x, y| {
            [x.0 + y.0, x.1 + y.1]
        })))
    }

    /// Product; exact only when both operands are, and an exact zero annihilates.
    pub fn mul(&self, o: &Coord) -> Result<Coord, TransformError> {
        if let (Coord::Exact(a), Coord::Exact(b)) = (self, o) {
            return Ok(Coord::Exact(a.mul(b)?));
        }
        let dim = self.dim().mul(o.dim());
        if self.is_exact_zero() || o.is_exact_zero() {
            return Ok(Coord::constant(dim, 0));
        }
        let (a, b) = (self.to_approx()?, o.to_approx()?);
        Ok(Coord::Approx(approx_op(&a, &b, dim, |x, y| {
            [x.0 * y.0, x.0 * y.1, x.1 * y.0, x.1 * y.1]
        })))
    }
}

/// Combine two intervals by the candidate endpoints `f` yields; pads outward when not a point.
fn approx_op<const N: usize>(
    a: &Approx,
    b: &Approx,
    dim: Dim,
    f: impl Fn((f64, f64), (f64, f64)) -> [f64; N],
) -> Approx {
    let c = f((a.lo, a.hi), (b.lo, b.hi));
    let lo = c.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = c.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let point = a.lo == a.hi && b.lo == b.hi;
    let (lo, hi) = if point { (lo, hi) } else { pad(lo, hi) };
    let root = a.root.or(b.root);
    trace!(?root, point, "transform: inexact coordinate arithmetic");
    Approx {
        lo,
        hi,
        root,
        conservative: a.conservative || b.conservative || !point,
        dim,
    }
}

/// Quarter turns (0..4) when the angle is a whole multiple of 90deg, else `None`.
fn quarter_turns(angle: &Magnitude) -> Option<u8> {
    if angle.is_zero() {
        return Some(0);
    }
    if angle.pi_exp() != 1 {
        return None;
    }
    // angle = r * pi, so the number of quarter turns is 2r.
    let n = angle.rational().mul(&Rational::from_int(2));
    if !n.denom().is_one() {
        return None;
    }
    let four = BigInt::from(4);
    let m = ((n.numer() % &four) + &four) % &four;
    m.to_u8()
}

type Mat = [[Coord; 3]; 3];

fn identity_rot() -> Mat {
    let dl = Dim::default();
    std::array::from_fn(|i| std::array::from_fn(|j| Coord::constant(dl.clone(), i64::from(i == j))))
}

/// `(cos, sin)` of the angle as dimensionless coordinates (see the module decisions).
pub(crate) fn cos_sin(angle: &Affine) -> Result<(Coord, Coord), TransformError> {
    if !angle.dim().dimension.is_dimensionless() {
        return Err(TransformError::NotAnAngle(format!("{:?}", angle.dim())));
    }
    let dl = Dim::default();
    let nominal = angle.center();
    let quarter = quarter_turns(nominal);
    if angle.is_exact()
        && let Some(n) = quarter
    {
        trace!(n, "transform: exact quarter-turn rotation");
        let (c, s) = [(1, 0), (0, 1), (-1, 0), (0, -1)][usize::from(n)];
        return Ok((Coord::constant(dl.clone(), c), Coord::constant(dl, s)));
    }
    let root = |op| if quarter.is_some() { None } else { Some(op) };
    if angle.is_exact() {
        let x = nominal.to_f64();
        debug!(root = "sin/cos", "transform: inexact rotation angle (Q006)");
        let pt = |v: f64, op| {
            Coord::Approx(Approx {
                lo: v,
                hi: v,
                root: Some(op),
                conservative: false,
                dim: dl.clone(),
            })
        };
        return Ok((pt(x.cos(), RootOp::Cos), pt(x.sin(), RootOp::Sin)));
    }
    debug!("transform: toleranced rotation angle, entries are conservative");
    let (c, s) = (angle.cos()?, angle.sin()?);
    let iv = |i: cadetta_units::InexactInterval, op| {
        Coord::Approx(Approx {
            lo: i.lo,
            hi: i.hi,
            root: root(op),
            conservative: true,
            dim: dl.clone(),
        })
    };
    Ok((iv(c, RootOp::Cos), iv(s, RootOp::Sin)))
}

fn rot_matrix(axis: Axis, c: Coord, s: Coord) -> Mat {
    let dl = Dim::default();
    let (z, o) = (Coord::constant(dl.clone(), 0), Coord::constant(dl, 1));
    let ns = s.neg();
    match axis {
        Axis::X => [
            [o.clone(), z.clone(), z.clone()],
            [z.clone(), c.clone(), ns],
            [z, s, c],
        ],
        Axis::Y => [
            [c.clone(), z.clone(), s.clone()],
            [z.clone(), o, z.clone()],
            [ns, z, c],
        ],
        Axis::Z => [
            [c.clone(), ns, z.clone()],
            [s, c, z.clone()],
            [z.clone(), z, o],
        ],
    }
}

fn dot(row: &[Coord; 3], v: &[Coord; 3]) -> Result<Coord, TransformError> {
    let mut acc = row[0].mul(&v[0])?;
    for k in 1..3 {
        acc = acc.add(&row[k].mul(&v[k])?)?;
    }
    Ok(acc)
}

/// The largest absolute value a coordinate can take.
fn abs_max(c: &Coord) -> Result<f64, TransformError> {
    let a = c.to_approx()?;
    Ok(a.lo.abs().max(a.hi.abs()))
}

/// One row of `R * p` where `p` holds disc terms and the row has binary64 entries.
///
/// The disc-free part goes through ordinary interval arithmetic. Each disc contributes
/// `+- sqrt(U^2 + V^2)`, where `U`, `V` are the row's combined reach over the disc's two
/// coefficients: the Euclidean norm, not the sum, so `cos t * (d/2) u + sin t * (d/2) v` stays
/// within `d/2` (geometry.md 3.1). The result is an outer bound and the disc identity is gone.
// frob:ticket 01M50C0NCKDC43EDJP1BS8CTVB
fn dot_discs(row: &[Coord; 3], v: &[Coord; 3]) -> Result<Option<Coord>, TransformError> {
    let forms: Vec<&Affine> = v
        .iter()
        .filter_map(|c| match c {
            Coord::Exact(a) => Some(a),
            Coord::Approx(_) => None,
        })
        .collect();
    let vars: std::collections::BTreeSet<_> = forms.iter().flat_map(|a| a.disc_vars()).collect();
    if forms.len() != 3 || vars.is_empty() || row.iter().all(Coord::is_exact) {
        return Ok(None);
    }
    let dim = v[0].dim().clone();
    let mut rest: Option<Coord> = None;
    for (r, f) in row.iter().zip(&forms) {
        let (c, b) = (f.center().to_f64(), f.box_radius()?.to_f64());
        let (lo, hi) = pad(c - b, c + b);
        let part = Coord::Approx(Approx {
            lo,
            hi,
            root: None,
            conservative: b != 0.0,
            dim: f.dim().clone(),
        });
        let term = r.mul(&part)?;
        rest = Some(match rest {
            None => term,
            Some(acc) => acc.add(&term)?,
        });
    }
    let mut reach = 0.0_f64;
    for var in &vars {
        let (mut u, mut w) = (0.0_f64, 0.0_f64);
        for (r, f) in row.iter().zip(&forms) {
            if let Some((a, b)) = f.disc_coefficients(*var) {
                let m = abs_max(r)?;
                u += m * a.to_f64().abs();
                w += m * b.to_f64().abs();
            }
        }
        reach += (u * u + w * w).sqrt();
    }
    let (lo, hi) = pad(-reach, reach);
    let discs = Coord::Approx(Approx {
        lo,
        hi,
        root: None,
        conservative: true,
        dim,
    });
    trace!(
        reach,
        "transform: inexact rotation of disc terms (Euclidean reach)"
    );
    Ok(Some(rest.expect("three terms").add(&discs)?))
}

/// A datum's composed 3x4 transform: `p_parent = R * p_local + t`.
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    rot: Mat,
    trans: [Coord; 3],
}

impl Transform {
    /// The identity transform (`origin`), translation entries having dimension `length`.
    pub fn identity(length: Dim) -> Self {
        Self {
            rot: identity_rot(),
            trans: std::array::from_fn(|_| Coord::constant(length.clone(), 0)),
        }
    }

    /// The rotation entry at row `i`, column `j`.
    pub fn rot(&self, i: usize, j: usize) -> &Coord {
        &self.rot[i][j]
    }

    /// The translation entry for row `i`.
    pub fn trans(&self, i: usize) -> &Coord {
        &self.trans[i]
    }

    fn entries(&self) -> impl Iterator<Item = &Coord> {
        self.rot.iter().flatten().chain(self.trans.iter())
    }

    /// True when no entry involves binary64 (geometry.md 2).
    pub fn is_exact(&self) -> bool {
        self.entries().all(Coord::is_exact)
    }

    /// The first Q006 root among the entries, if the transform is inexact.
    pub fn root(&self) -> Option<RootOp> {
        self.entries().find_map(Coord::root)
    }

    /// True when any entry is an outer bound (quantities.md 5.4).
    pub fn is_conservative(&self) -> bool {
        self.entries().any(Coord::is_conservative)
    }

    /// Applies `.offset(..)` in the current frame; `None` axes are zero.
    pub fn offset(&self, d: &[Option<Affine>; 3]) -> Result<Self, TransformError> {
        let dim = self.trans[0].dim().clone();
        let v: [Coord; 3] = std::array::from_fn(|i| match &d[i] {
            Some(a) => Coord::Exact(a.clone()),
            None => Coord::constant(dim.clone(), 0),
        });
        let mut trans = self.trans.clone();
        for (i, t) in trans.iter_mut().enumerate() {
            *t = t.add(&dot(&self.rot[i], &v)?)?;
        }
        debug!("transform: offset composed");
        Ok(Self {
            rot: self.rot.clone(),
            trans,
        })
    }

    /// Applies `.rotate(axis = angle)` in the current frame.
    pub fn rotate(&self, axis: Axis, angle: &Affine) -> Result<Self, TransformError> {
        let (c, s) = cos_sin(angle)?;
        let r = rot_matrix(axis, c, s);
        let rot: Mat =
            std::array::from_fn(|_| std::array::from_fn(|_| Coord::constant(Dim::default(), 0)));
        let mut rot = rot;
        for (i, row) in rot.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                let col: [Coord; 3] = std::array::from_fn(|k| r[k][j].clone());
                *cell = dot(&self.rot[i], &col)?;
            }
        }
        debug!(axis = axis.name(), "transform: rotation composed");
        Ok(Self {
            rot,
            trans: self.trans.clone(),
        })
    }

    /// Applies `.flip(axis)`, a half turn.
    pub fn flip(&self, axis: Axis) -> Result<Self, TransformError> {
        let pi = Magnitude::new(Rational::from_int(1), 1);
        self.rotate(axis, &Affine::constant(Dim::new(Default::default(), 1), pi))
    }

    /// Carries a local point through the transform: `R * p + t` (disc terms bounded by norm).
    pub fn apply(&self, p: &[Coord; 3]) -> Result<[Coord; 3], TransformError> {
        let mut out = self.trans.clone();
        for (i, o) in out.iter_mut().enumerate() {
            let term = match dot_discs(&self.rot[i], p)? {
                Some(t) => t,
                None => dot(&self.rot[i], p)?,
            };
            *o = o.add(&term)?;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_units::{Dimension, Toleranced, VarId};

    fn len() -> Dim {
        Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
    }

    fn ang() -> Dim {
        Dim::new(Dimension::dimensionless(), 1)
    }

    /// An angle of `n/d` degrees as an exact affine constant.
    fn deg(n: i64, d: i64) -> Affine {
        let m = Magnitude::new(Rational::new(n, 180 * d).unwrap(), 1);
        Affine::constant(ang(), m)
    }

    fn length(n: i64) -> Affine {
        Affine::constant(len(), mag(n, 1))
    }

    fn exact_val(c: &Coord) -> Magnitude {
        match c {
            Coord::Exact(a) => a.center().clone(),
            Coord::Approx(_) => panic!("expected exact"),
        }
    }

    // frob:tests crates/cadetta-geometry/src/transform.rs::quarter_turns_are_exact kind=unit
    #[test]
    fn quarter_turns_are_exact() {
        for n in [-3, -1, 0, 1, 2, 3, 4, 7] {
            let t = Transform::identity(len())
                .rotate(Axis::Z, &deg(90 * n, 1))
                .unwrap();
            assert!(t.is_exact(), "{n} quarter turns");
            assert!(t.root().is_none());
        }
        let t = Transform::identity(len())
            .rotate(Axis::Z, &deg(90, 1))
            .unwrap()
            .offset(&[Some(length(1)), None, None])
            .unwrap();
        // x axis maps to y: R[1][0] = sin(90) = 1, R[0][0] = 0; offset along local x goes to y.
        assert_eq!(exact_val(t.rot(1, 0)), mag(1, 1));
        assert_eq!(exact_val(t.rot(0, 0)), mag(0, 1));
        assert_eq!(exact_val(t.trans(1)), mag(1, 1));
        assert_eq!(exact_val(t.trans(0)), mag(0, 1));
        let f = Transform::identity(len()).flip(Axis::X).unwrap();
        assert!(f.is_exact());
        assert_eq!(exact_val(f.rot(2, 2)), mag(-1, 1));
    }

    // frob:tests crates/cadetta-geometry/src/transform.rs::other_angles_are_inexact_with_root kind=unit
    #[test]
    fn other_angles_are_inexact_with_root() {
        for (n, d) in [(45, 1), (60, 1), (30, 1), (1, 2)] {
            let t = Transform::identity(len())
                .rotate(Axis::Y, &deg(n, d))
                .unwrap();
            assert!(!t.is_exact(), "{n}/{d} deg");
            assert!(matches!(t.root(), Some(RootOp::Cos | RootOp::Sin)));
            assert!(!t.is_conservative());
        }
        let t = Transform::identity(len())
            .rotate(Axis::Z, &deg(45, 1))
            .unwrap();
        let Coord::Approx(p) = t.rot(0, 0) else {
            panic!()
        };
        assert!((p.lo - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
    }

    fn toleranced_angle(n: i64) -> Affine {
        let nominal = Magnitude::new(Rational::new(n, 180).unwrap(), 1);
        let dev = (ang(), Magnitude::new(Rational::new(1, 1800).unwrap(), 1));
        Toleranced::symmetric(ang(), nominal, dev)
            .unwrap()
            .to_affine(VarId(7))
            .unwrap()
    }

    // frob:tests crates/cadetta-geometry/src/transform.rs::toleranced_angle_is_conservative kind=unit
    #[test]
    fn toleranced_angle_is_conservative() {
        let t = Transform::identity(len())
            .rotate(Axis::Z, &toleranced_angle(90))
            .unwrap();
        assert!(!t.is_exact());
        assert!(t.is_conservative());
        // the nominal is a quarter turn, so no Q006 root
        assert!(t.root().is_none());
        let p = [
            Coord::Exact(length(10)),
            Coord::constant(len(), 0),
            Coord::constant(len(), 0),
        ];
        let q = t.apply(&p).unwrap();
        let Coord::Approx(y) = &q[1] else { panic!() };
        // y = 10 * sin(90deg +- 0.1deg) must contain the true values
        let s = (90.0f64 + 0.1).to_radians().sin() * 10.0;
        assert!(y.conservative && y.lo <= s && s <= y.hi);
        assert!(y.hi >= 10.0);
        let t45 = Transform::identity(len())
            .rotate(Axis::Z, &toleranced_angle(45))
            .unwrap();
        assert!(t45.is_conservative());
        assert!(t45.root().is_some());
    }

    // frob:tests crates/cadetta-geometry/src/transform.rs::offsets_keep_source_variables kind=unit
    #[test]
    fn offsets_keep_source_variables() {
        let nominal = mag(10, 1);
        let dev = (len(), mag(1, 20));
        let off = Toleranced::symmetric(len(), nominal, dev)
            .unwrap()
            .to_affine(VarId(3))
            .unwrap();
        let t = Transform::identity(len())
            .offset(&[Some(off.clone()), None, None])
            .unwrap()
            .rotate(Axis::Z, &deg(90, 1))
            .unwrap()
            .offset(&[Some(off.clone()), None, None])
            .unwrap();
        assert!(t.is_exact());
        // x gets the first offset only; y gets the second, rotated: both keep variable 3.
        let Coord::Exact(x) = t.trans(0) else {
            panic!()
        };
        let Coord::Exact(y) = t.trans(1) else {
            panic!()
        };
        assert_eq!(x.coefficient(VarId(3)), off.coefficient(VarId(3)));
        assert_eq!(y.coefficient(VarId(3)), off.coefficient(VarId(3)));
        assert_eq!(x.center(), &mag(10, 1));
        // same variable composed twice into one coordinate doubles one term
        let t2 = Transform::identity(len())
            .offset(&[Some(off.clone()), None, None])
            .unwrap()
            .offset(&[Some(off.clone()), None, None])
            .unwrap();
        let Coord::Exact(x2) = t2.trans(0) else {
            panic!()
        };
        assert_eq!(
            x2.coefficient(VarId(3)).unwrap(),
            &off.coefficient(VarId(3)).unwrap().mul(&mag(2, 1))
        );
    }

    /// A `[position 0.2mm]` point at the origin (radius 0.1mm) on disc variable 0.
    fn disc_point() -> [Coord; 3] {
        let r = mag(1, 10_000);
        let z = mag(0, 1);
        let x = Affine::disc(len(), z.clone(), VarId(0), r.clone(), z.clone());
        let y = Affine::disc(len(), z.clone(), VarId(0), z.clone(), r);
        [Coord::Exact(x), Coord::Exact(y), Coord::constant(len(), 0)]
    }

    // frob:tests crates/cadetta-geometry/src/transform.rs::Transform.apply kind=unit
    #[test]
    fn inexact_rotation_bounds_a_disc_projection_by_the_euclidean_norm() {
        // Rotating by 45deg: x' = cos*x + sin*y, each a 0.1mm disc coordinate. The L1 reach is
        // 0.1*(cos+sin) = 0.1414mm; the true and Euclidean reach is 0.1mm.
        let t = Transform::identity(len())
            .rotate(Axis::Z, &deg(45, 1))
            .unwrap();
        let out = t.apply(&disc_point()).unwrap();
        for c in &out[..2] {
            let Coord::Approx(a) = c else {
                panic!("inexact rotation")
            };
            assert!(a.conservative);
            assert!(a.hi >= 1.0e-4 && a.hi < 1.0001e-4, "{}", a.hi);
            assert!(a.lo <= -1.0e-4 && a.lo > -1.0001e-4, "{}", a.lo);
        }
    }

    // frob:tests crates/cadetta-geometry/src/transform.rs::Transform.apply kind=unit
    #[test]
    fn exact_quarter_turns_keep_the_disc_identity() {
        let t = Transform::identity(len())
            .rotate(Axis::Z, &deg(90, 1))
            .unwrap();
        let out = t.apply(&disc_point()).unwrap();
        let Coord::Exact(x) = &out[0] else {
            panic!("exact")
        };
        assert_eq!(x.disc_vars(), vec![VarId(0)]);
    }
}
