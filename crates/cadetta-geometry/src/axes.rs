//! Axis builtins `coaxial`, `parallel` and `angle_between` over datum z axes (geometry.md 5.1).
//!
//! Each datum's axis is its z direction through its origin. The axis of `a` is sampled at two
//! points, the origin and the point `over` along z; both are carried into the datum of `b` through
//! the [`FrameTable`], so tilt, flips and tolerances on the placement all flow into the result.
//! `coaxial` bounds the radial distance of both points from the z axis of `b` (a tilt is included
//! and the zone is a radius); `parallel` bounds only the lateral drift between them, so an offset
//! is ignored; `angle_between` is the angle between the two z directions in `[0, pi]`, exact at
//! multiples of a quarter turn when nothing is toleranced and otherwise inexact and conservative
//! (Q006).
// frob:ticket 01M4V43BJ1Z4EZTYJXMMCJWA2D

use cadetta_units::compare::Verdict;
use cadetta_units::{Affine, Magnitude, Rational};
use tracing::{debug, info};

use crate::aligned::{
    AlignedError, AlignedReport, coord_bounds, located_diff, measure, pad_down, pad_up, to_datum,
};
use crate::placement::length_dim;
use crate::point::{DatumId, Point};
use crate::reexpress::{FrameTable, Located};
use crate::transform::Coord;

/// The angle between two z axes: a nominal, an outer interval, and the exact value when known.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisAngle {
    /// The nominal angle in radians.
    pub nominal: f64,
    /// Lower bound over every value in tolerance, in radians (outward padded).
    pub lo: f64,
    /// Upper bound over every value in tolerance, in radians (outward padded).
    pub hi: f64,
    /// The exact angle (a rational multiple of pi) when nothing is toleranced and it is one.
    pub exact: Option<Magnitude>,
}

fn z_point(datum: &DatumId, z: Affine) -> Point {
    let zero = || Affine::constant(length_dim(), Magnitude::new(Rational::from_int(0), 0));
    Point {
        datum: datum.clone(),
        coords: vec![zero(), zero(), z],
    }
}

/// The origin and the point `over` along z of datum `a`, both in the datum `b`.
fn carried_axis(
    table: &FrameTable,
    a: &DatumId,
    b: &DatumId,
    over: &Affine,
) -> Result<(Located, Located), AlignedError> {
    if over.dim() != &length_dim() {
        return Err(AlignedError::Invalid {
            message: "the length an axis is sampled over must be a length".into(),
        });
    }
    let zero = Affine::constant(length_dim(), Magnitude::new(Rational::from_int(0), 0));
    let p0 = to_datum(table, &z_point(a, zero), b)?;
    let p1 = to_datum(table, &z_point(a, over.clone()), b)?;
    Ok((p0, p1))
}

fn combine(pairs: Vec<crate::aligned::Measured>) -> AlignedReport {
    let verdict = if pairs
        .iter()
        .any(|m| m.comparison.verdict == Verdict::Violates)
    {
        Verdict::Violates
    } else if pairs
        .iter()
        .any(|m| m.comparison.verdict == Verdict::Undecided)
    {
        Verdict::Undecided
    } else {
        Verdict::Holds
    };
    AlignedReport { pairs, verdict }
}

/// `coaxial(a, b, allowance, over)`: both sampled points of axis `a` within `allowance` of axis `b`.
pub fn coaxial(
    table: &FrameTable,
    a: &DatumId,
    b: &DatumId,
    allowance: &Affine,
    over: &Affine,
) -> Result<AlignedReport, AlignedError> {
    let (p0, p1) = carried_axis(table, a, b, over)?;
    let mut pairs = Vec::with_capacity(2);
    for p in [&p0, &p1] {
        pairs.push(measure(&p.coords[..2], allowance)?);
    }
    let report = combine(pairs);
    info!(%a, %b, verdict = ?report.verdict, "axes: coaxial evaluated");
    Ok(report)
}

/// `parallel(a, b, allowance, over)`: the lateral drift of axis `a` over `over`, offset ignored.
pub fn parallel(
    table: &FrameTable,
    a: &DatumId,
    b: &DatumId,
    allowance: &Affine,
    over: &Affine,
) -> Result<AlignedReport, AlignedError> {
    let (p0, p1) = carried_axis(table, a, b, over)?;
    let mut lateral = located_diff(&p1, &p0)?;
    lateral.truncate(2);
    let report = combine(vec![measure(&lateral, allowance)?]);
    info!(%a, %b, verdict = ?report.verdict, "axes: parallel evaluated");
    Ok(report)
}

/// The exact value of a coordinate with no tolerance, as a magnitude.
fn exact_value(c: &Coord) -> Option<Magnitude> {
    match c {
        Coord::Exact(a) => {
            let (lo, hi) = a.bounds().ok()?;
            (lo == hi).then(|| a.center().clone())
        }
        Coord::Approx(_) => None,
    }
}

/// The direction of the z axis of `a` in the datum `b`, one metre long, as three coordinates.
fn direction(table: &FrameTable, a: &DatumId, b: &DatumId) -> Result<[Coord; 3], AlignedError> {
    let one = Affine::constant(length_dim(), Magnitude::new(Rational::from_int(1), 0));
    let (p0, p1) = carried_axis(table, a, b, &one)?;
    let d = located_diff(&p1, &p0)?;
    let [x, y, z]: [Coord; 3] = d.try_into().map_err(|_| AlignedError::Invalid {
        message: "an axis direction has three coordinates".into(),
    })?;
    Ok([x, y, z])
}

fn norm_bounds(x: (f64, f64), y: (f64, f64)) -> (f64, f64) {
    let sq = |(a, b): (f64, f64)| {
        if a <= 0.0 && b >= 0.0 {
            (0.0, a.abs().max(b.abs()).powi(2))
        } else {
            let (m, n) = (a.abs().min(b.abs()), a.abs().max(b.abs()));
            (m * m, n * n)
        }
    };
    let (xl, xh) = sq(x);
    let (yl, yh) = sq(y);
    (pad_down(xl + yl).max(0.0).sqrt(), pad_up(xh + yh).sqrt())
}

/// `angle_between(a, b)`: the angle between the z axes of `a` and `b`, in `[0, pi]`.
///
/// `form` carries tolerances (the bounds) and `nominal` the nominal frames (the nominal angle and
/// exactness). The angle is `atan2(r, dz)` of the carried direction, with `r` the lateral length;
/// atan2 is monotone in `r` and in `dz`, so the interval ends are found at the box corners.
pub fn angle_between(
    form: &FrameTable,
    nominal: &FrameTable,
    a: &DatumId,
    b: &DatumId,
) -> Result<AxisAngle, AlignedError> {
    let [fx, fy, fz] = direction(form, a, b)?;
    let [nx, ny, nz] = direction(nominal, a, b)?;
    let (xb, yb, zb) = (coord_bounds(&fx)?, coord_bounds(&fy)?, coord_bounds(&fz)?);
    let (rl, rh) = norm_bounds(xb, yb);
    // dz is the cosine of the angle: an angle is smallest at the largest dz and the smallest r.
    let lo = pad_down(rl.atan2(zb.1)).max(0.0);
    let hi = pad_up(rh.atan2(zb.0)).min(std::f64::consts::PI);
    let (nxb, nyb, nzb) = (coord_bounds(&nx)?, coord_bounds(&ny)?, coord_bounds(&nz)?);
    let nr = (nxb.0.powi(2) + nyb.0.powi(2)).sqrt();
    let nominal_angle = nr.atan2(nzb.0);
    let toleranced = [&fx, &fy, &fz].iter().any(|c| exact_value(c).is_none());
    let exact = if toleranced {
        None
    } else {
        exact_angle(&nx, &ny, &nz)
    };
    debug!(%a, %b, nominal = nominal_angle, lo, hi, exact = exact.is_some(), "axes: angle_between");
    Ok(AxisAngle {
        nominal: nominal_angle,
        lo,
        hi,
        exact,
    })
}

/// The angle as an exact multiple of pi when the lateral part is zero or the axial part is zero.
fn exact_angle(x: &Coord, y: &Coord, z: &Coord) -> Option<Magnitude> {
    let (mx, my, mz) = (exact_value(x)?, exact_value(y)?, exact_value(z)?);
    let zero = |m: &Magnitude| m.is_zero();
    let pi_over = |num: i64, den: i64| Rational::new(num, den).ok().map(|r| Magnitude::new(r, 1));
    match (zero(&mx) && zero(&my), zero(&mz)) {
        (true, true) => None,
        (true, false) => {
            if mz.to_f64() > 0.0 {
                Some(Magnitude::new(Rational::from_int(0), 0))
            } else {
                pi_over(1, 1)
            }
        }
        (false, true) => pi_over(1, 2),
        (false, false) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::tests::{build, eval, src};

    fn um(n: i64) -> Affine {
        Affine::constant(
            length_dim(),
            Magnitude::new(Rational::new(n, 1_000_000).unwrap(), 0),
        )
    }

    fn table() -> FrameTable {
        let a = build(
            &src(
                "assembly Mount {\n  part bracket: Bracket\n  place bracket.origin at origin\n}\n",
            ),
            &[],
        );
        FrameTable::for_part(&a.nodes[0].datums, &eval).unwrap()
    }

    fn id(s: &str) -> DatumId {
        DatumId::new(s)
    }

    // frob:tests crates/cadetta-geometry/src/axes.rs::coaxial kind=unit
    #[test]
    fn coaxial_bounds_the_radial_offset() {
        let t = table();
        let held = coaxial(&t, &id("base"), &id("origin"), &um(500), &um(20_000)).unwrap();
        assert_eq!(held.verdict, Verdict::Holds);
        let broken = coaxial(&t, &id("holes"), &id("origin"), &um(500), &um(20_000)).unwrap();
        assert_eq!(broken.verdict, Verdict::Violates);
        let flipped = coaxial(&t, &id("under"), &id("origin"), &um(500), &um(20_000)).unwrap();
        assert_eq!(flipped.verdict, Verdict::Holds);
    }

    // frob:tests crates/cadetta-geometry/src/axes.rs::coaxial kind=unit
    #[test]
    fn a_non_length_sampling_distance_is_invalid() {
        let t = table();
        let bad = Affine::constant(
            crate::placement::length_dim().mul(&length_dim()),
            Magnitude::new(Rational::from_int(1), 0),
        );
        let e = coaxial(&t, &id("base"), &id("origin"), &um(500), &bad).unwrap_err();
        assert!(matches!(e, AlignedError::Invalid { .. }));
    }

    // frob:tests crates/cadetta-geometry/src/axes.rs::parallel kind=unit
    #[test]
    fn parallel_ignores_offset() {
        let t = table();
        let r = parallel(&t, &id("holes"), &id("origin"), &um(1_000), &um(20_000)).unwrap();
        assert_eq!(r.verdict, Verdict::Holds);
        assert_eq!(r.pairs.len(), 1);
    }

    // frob:tests crates/cadetta-geometry/src/axes.rs::angle_between kind=unit
    #[test]
    fn angle_between_is_exact_at_zero_and_half_turn() {
        let t = table();
        let zero = angle_between(&t, &t, &id("holes"), &id("origin")).unwrap();
        assert_eq!(zero.exact, Some(Magnitude::new(Rational::from_int(0), 0)));
        let flip = angle_between(&t, &t, &id("under"), &id("origin")).unwrap();
        assert_eq!(flip.exact, Some(Magnitude::new(Rational::from_int(1), 1)));
        assert!(flip.lo <= std::f64::consts::PI && flip.hi >= std::f64::consts::PI - 1e-9);
    }
}
