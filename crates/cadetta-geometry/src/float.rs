//! Bolt float: the implicit placement float of a non-locating `ScrewJoint` (geometry.md 5.2).
//!
//! A bolted part in clearance holes is not located exactly, so the placement edge between the two
//! parts of the joint carries `[+-float]` on x and y and a rotation `[+-theta]` about z through
//! the centroid of the `onto` points, `theta = 2 * asin(float / s)` with `s` the largest distance
//! between two `onto` points (`180deg` when `float >= s`). [`EdgeFloat`] holds the joints of one
//! edge as the caller (which knows the pattern points and the source variables) lowered them;
//! [`select`] picks the one joint whose whole bound the edge takes, and [`float_ops`] turns it
//! into frame operations. [`unconstrained_rotation`] is the G008 condition.
//!
//! Decisions made here: the float operations are expressed in the frame of the placement datum
//! that coincides with the anchor (every datum of a part moves with the part), so the caller
//! gives `onto` points in that frame; `theta` is rounded up to a multiple of `1e-9 pi` so the
//! exact bound stays outward; a joint's own `aligned` check reads the edge without the float
//! ([`FloatMode::Without`]).

// frob:ticket 01M4SWG6Z6JVP5Z7H38HWJP0GQ
use std::cmp::Ordering;

use cadetta_syntax::span::Span;
use cadetta_units::{Affine, Dim, Dimension, Magnitude, Rational, Toleranced, VarId};
use tracing::{debug, trace};

use crate::datum::{Axis, GeometryError};
use crate::placement::{FrameOp, length_dim};
#[cfg(test)]
use crate::transform::Coord;

/// Resolution of the rounded-up `theta`, in turns of `pi`.
const THETA_STEP: i64 = 1_000_000_000;

/// One joint's claim on a placement edge.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct JointFloat {
    /// The joint's canonical path; the last tie-break.
    pub path: String,
    /// The joint's exported `float: Length`.
    pub float: Affine,
    /// True when a pilot, dowel or fit locates the part.
    pub locating: bool,
    /// The nominal `onto` points (x, y) in the frame of the placement datum.
    pub onto: Vec<[Affine; 2]>,
    /// Source variables for the x translation, the y translation and the rotation.
    pub vars: [VarId; 3],
}

/// The non-locating-joint candidates of one placement edge, named by its child node.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EdgeFloat {
    /// The placed node (child of the edge).
    pub node: String,
    /// Every joint between the edge's two parts.
    pub joints: Vec<JointFloat>,
}

/// Whether a read of the tree sees the floats.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FloatMode<'a> {
    /// Every path reads the floats.
    With,
    /// The `aligned` check of the named joint reads its own edge without the float.
    Without(&'a str),
}

fn fail(message: impl Into<String>) -> GeometryError {
    let message = message.into();
    debug!(%message, "float error");
    GeometryError {
        span: Span::default(),
        message,
    }
}

fn tol(e: impl std::fmt::Display) -> GeometryError {
    fail(format!("placement float: {e}"))
}

fn upper(a: &Affine) -> Result<f64, GeometryError> {
    Ok(a.bounds().map_err(tol)?.1.to_f64())
}

fn nominal(a: &Affine) -> f64 {
    a.center().to_f64()
}

/// The largest distance between two `onto` points, `s` of geometry.md 5.2.
pub fn spread(onto: &[[Affine; 2]]) -> f64 {
    let mut s = 0.0_f64;
    for (i, a) in onto.iter().enumerate() {
        for b in &onto[i + 1..] {
            let dx = nominal(&a[0]) - nominal(&b[0]);
            let dy = nominal(&a[1]) - nominal(&b[1]);
            s = s.max(dx.hypot(dy));
        }
    }
    s
}

/// `theta` in turns of `pi`, rounded up: `2 asin(float / s) / pi`, and `1` (180deg) when
/// `float >= s` or `s = 0`.
pub fn theta_pi(float: f64, s: f64) -> i64 {
    if s <= 0.0 || float >= s {
        return THETA_STEP;
    }
    let ratio = (float / s) * (1.0 + 4.0 * f64::EPSILON);
    if ratio >= 1.0 {
        return THETA_STEP;
    }
    let turns = 2.0 * ratio.asin() / std::f64::consts::PI;
    ((turns * THETA_STEP as f64).ceil() as i64).clamp(0, THETA_STEP)
}

/// G008: the joint does not fix rotation (one point, coincident points, or `float >= s`).
pub fn unconstrained_rotation(j: &JointFloat) -> Result<bool, GeometryError> {
    Ok(theta_pi(upper(&j.float)?, spread(&j.onto)) >= THETA_STEP)
}

/// The joint whose whole bound the edge takes: smallest `theta`, then smallest float, then the
/// first path in byte order; `None` when any joint locates the part or there is no joint.
pub fn select(e: &EdgeFloat) -> Result<Option<&JointFloat>, GeometryError> {
    if e.joints.iter().any(|j| j.locating) {
        trace!(node = %e.node, "edge located; no float");
        return Ok(None);
    }
    let mut best: Option<(&JointFloat, i64, f64)> = None;
    for j in &e.joints {
        let f = upper(&j.float)?;
        let t = theta_pi(f, spread(&j.onto));
        let better = match &best {
            None => true,
            Some((b, bt, bf)) => {
                t.cmp(bt)
                    .then(f.partial_cmp(bf).unwrap_or(Ordering::Equal))
                    .then_with(|| j.path.as_bytes().cmp(b.path.as_bytes()))
                    == Ordering::Less
            }
        };
        if better {
            best = Some((j, t, f));
        }
    }
    debug!(node = %e.node, chosen = ?best.map(|b| b.0.path.as_str()), "float joint selected");
    Ok(best.map(|b| b.0))
}

fn sym(dim: &Dim, radius: Magnitude, var: VarId) -> Result<Affine, GeometryError> {
    let zero = Magnitude::new(Rational::from_int(0), 0);
    Toleranced::symmetric(dim.clone(), zero, (dim.clone(), radius))
        .and_then(|t| t.to_affine(var))
        .map_err(tol)
}

/// The frame operations of one joint's bound: `[+-float]` on x and y, then a rotation
/// `[+-theta]` about z through the centroid of the `onto` points.
pub(crate) fn float_ops(j: &JointFloat) -> Result<Vec<FrameOp>, GeometryError> {
    let len = length_dim();
    let f = j.float.bounds().map_err(tol)?.1;
    let tx = sym(&len, f.clone(), j.vars[0])?;
    let ty = sym(&len, f, j.vars[1])?;
    let n = j.onto.len().max(1) as i64;
    let mut cx = Affine::constant(len.clone(), Magnitude::new(Rational::from_int(0), 0));
    let mut cy = cx.clone();
    for p in &j.onto {
        cx = cx.add(&p[0].clone_nominal()).map_err(tol)?;
        cy = cy.add(&p[1].clone_nominal()).map_err(tol)?;
    }
    let inv = Affine::constant(
        Dim::default(),
        Magnitude::new(Rational::new(1, n).map_err(tol)?, 0),
    );
    let (cx, cy) = (cx.mul(&inv).map_err(tol)?, cy.mul(&inv).map_err(tol)?);
    let ang = Dim::new(Dimension::dimensionless(), 1);
    let theta = Magnitude::new(
        Rational::new(theta_pi(upper(&j.float)?, spread(&j.onto)), THETA_STEP).map_err(tol)?,
        1,
    );
    let rot = sym(&ang, theta, j.vars[2])?;
    debug!(joint = %j.path, points = j.onto.len(), "float operations built");
    Ok(vec![
        FrameOp::Offset(Box::new([Some(tx), Some(ty), None])),
        FrameOp::Offset(Box::new([Some(cx.clone()), Some(cy.clone()), None])),
        FrameOp::Rotate(Axis::Z, Box::new(rot)),
        FrameOp::Offset(Box::new([Some(cx.neg()), Some(cy.neg()), None])),
    ])
}

/// The float operations of the edge into `node` under `mode`; empty when there is none.
pub(crate) fn edge_ops(
    floats: &[EdgeFloat],
    node: &str,
    mode: FloatMode<'_>,
) -> Result<Vec<FrameOp>, GeometryError> {
    let Some(e) = floats.iter().find(|e| e.node == node) else {
        return Ok(Vec::new());
    };
    if let FloatMode::Without(own) = mode
        && e.joints.iter().any(|j| j.path == own)
    {
        trace!(node, own, "joint reads its own edge without the float");
        return Ok(Vec::new());
    }
    match select(e)? {
        Some(j) => float_ops(j),
        None => Ok(Vec::new()),
    }
}

trait Nominal {
    fn clone_nominal(&self) -> Affine;
}

impl Nominal for Affine {
    /// The same quantity with its tolerance dropped (the nominal point).
    fn clone_nominal(&self) -> Affine {
        Affine::constant(self.dim().clone(), self.center().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::place_assembly_reading;
    use crate::placement::tests::{build, eval, src};

    fn mm(n: i64) -> Affine {
        Affine::constant(
            length_dim(),
            Magnitude::new(Rational::new(n, 1000).unwrap(), 0),
        )
    }

    fn joint(path: &str, float_mm: i64, xs: &[i64], locating: bool, v: u32) -> JointFloat {
        JointFloat {
            path: path.to_owned(),
            float: mm(float_mm),
            locating,
            onto: xs.iter().map(|x| [mm(*x), mm(0)]).collect(),
            vars: [VarId(v), VarId(v + 1), VarId(v + 2)],
        }
    }

    fn edge(joints: Vec<JointFloat>) -> EdgeFloat {
        EdgeFloat {
            node: "bracket".into(),
            joints,
        }
    }

    // frob:tests crates/cadetta-geometry/src/float.rs::theta_pi kind=unit
    #[test]
    fn theta_is_two_asin_and_half_turn_when_unconstrained() {
        let t = theta_pi(1.0, 10.0) as f64 / THETA_STEP as f64 * std::f64::consts::PI;
        assert!((t - 2.0 * 0.1_f64.asin()).abs() < 1e-8);
        assert!(t >= 2.0 * 0.1_f64.asin());
        assert_eq!(theta_pi(10.0, 10.0), THETA_STEP);
        assert_eq!(theta_pi(1.0, 0.0), THETA_STEP);
    }

    // frob:tests crates/cadetta-geometry/src/float.rs::unconstrained_rotation kind=unit
    #[test]
    fn one_point_pattern_is_g008() {
        assert!(unconstrained_rotation(&joint("a", 1, &[0], false, 1)).unwrap());
        assert!(unconstrained_rotation(&joint("a", 1, &[5, 5], false, 1)).unwrap());
        assert!(!unconstrained_rotation(&joint("a", 1, &[0, 10], false, 1)).unwrap());
    }

    // frob:tests crates/cadetta-geometry/src/float.rs::select kind=unit
    #[test]
    fn smallest_theta_then_float_then_path() {
        let e = edge(vec![
            joint("b", 1, &[0, 10], false, 1),
            joint("c", 1, &[0, 20], false, 4),
            joint("a", 1, &[0, 20], false, 7),
        ]);
        assert_eq!(select(&e).unwrap().unwrap().path, "a");
        let e = edge(vec![
            joint("a", 2, &[0, 20], false, 1),
            joint("b", 1, &[0, 20], false, 4),
        ]);
        assert_eq!(select(&e).unwrap().unwrap().path, "b");
    }

    // frob:tests crates/cadetta-geometry/src/float.rs::select kind=unit
    #[test]
    fn locating_joint_removes_the_float() {
        let e = edge(vec![
            joint("a", 1, &[0, 20], false, 1),
            joint("b", 1, &[0, 20], true, 4),
        ]);
        assert!(select(&e).unwrap().is_none());
    }

    fn mount(floats: Vec<EdgeFloat>) -> crate::placement::Assembly {
        let mut a = build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  part bracket: Bracket\n  place rail.origin at origin\n  place bracket.base at rail.top\n}\n",
            ),
            &[],
        );
        a.floats = floats;
        a
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::place_assembly kind=unit
    #[test]
    fn edge_carries_translation_and_rotation_bound() {
        let a = mount(vec![edge(vec![joint("j", 1, &[0, 10], false, 1)])]);
        let placed = place_assembly_reading(&a, &eval, FloatMode::With).unwrap();
        let t = &placed[1].transform;
        assert!(t.is_conservative() || !t.is_exact(), "rotation is inexact");
        let (lo, hi) = span(t.trans(0));
        assert!(lo <= -0.001 && hi >= 0.001, "x covers +-float: {lo} {hi}");
        let (lo, hi) = span(t.trans(1));
        assert!(lo <= -0.001 && hi >= 0.001, "y covers +-float: {lo} {hi}");
    }

    fn span(c: &Coord) -> (f64, f64) {
        match c {
            Coord::Exact(a) => {
                let (lo, hi) = a.bounds().unwrap();
                (lo.to_f64(), hi.to_f64())
            }
            Coord::Approx(a) => (a.lo, a.hi),
        }
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::place_assembly_reading kind=unit
    #[test]
    fn own_check_reads_edge_without_float() {
        let a = mount(vec![edge(vec![joint("j", 1, &[0, 10], false, 1)])]);
        let placed = place_assembly_reading(&a, &eval, FloatMode::Without("j")).unwrap();
        let t = &placed[1].transform;
        assert!(t.is_exact());
        assert_eq!(span(t.trans(0)).0, span(t.trans(0)).1);
        let other = place_assembly_reading(&a, &eval, FloatMode::Without("other")).unwrap();
        let (lo, hi) = span(other[1].transform.trans(0));
        assert!(hi - lo >= 0.002);
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::place_assembly kind=unit
    #[test]
    fn locating_joint_leaves_transform_exact() {
        let a = mount(vec![edge(vec![joint("j", 1, &[0, 10], true, 1)])]);
        let placed = place_assembly_reading(&a, &eval, FloatMode::With).unwrap();
        assert!(placed[1].transform.is_exact());
    }
}
