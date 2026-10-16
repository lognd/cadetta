//! Position zones as disc terms, and the disc-aware length bound (geometry.md 3.1).
//!
//! A `[position d]` zone is a disc of diameter `d` in the datum plane. [`position_point`] builds
//! the point `(nx + (d/2) u, ny + (d/2) v)` on one disc variable (`u^2 + v^2 <= 1`), and
//! [`project`] reads `.x` / `.y` off it, so a projection is exactly `[-d/2, d/2]` about the
//! nominal. The squared length of a difference vector over such points is bounded by
//! [`disc_bound`] from the Gram matrix of each disc's coefficient columns, not coordinate by
//! coordinate, so two `[position 0.1mm]` points are 0.1mm apart at worst rather than 0.1414mm.
//!
//! Decided here (written into geometry.md 3.1 and quantities.md 5.4):
//! - A disc with orthogonal equal-length columns confined to the xy rows is a true disc in the
//!   plane; with no box part left, the sum of such discs is one disc, and the length range is
//!   exact (sqrt only where the centre offset is not a rational square, then outward rationals
//!   and `conservative`).
//! - Anything else (a tilted disc reaching z, an elliptic image, or box terms mixed in) is
//!   bounded by `|centre| +- (sum of spectral radii + box norm)`, always conservative.
//! - Only rational (pi-free) forms take this path; others fall back to per-coordinate bounds.
// frob:ticket 01M50C0NCKDC43EDJP1BS8CTVB

use std::collections::BTreeSet;

use cadetta_units::{Affine, Magnitude, Rational, ToleranceError, VarId};
use num_bigint::{BigInt, Sign};
use thiserror::Error;
use tracing::{debug, trace};

use crate::aligned::{pad_down, pad_up};
use crate::datum::Axis;
use crate::point::{DatumId, Point};

/// Why a position zone could not be built or projected.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ZoneError {
    /// The zone or projection is malformed (arity, inexact nominal, negative diameter).
    #[error("{0}")]
    Invalid(String),
    /// A tolerance operation failed.
    #[error("tolerance: {0}")]
    Tolerance(#[from] ToleranceError),
}

/// A squared-length range from the disc-aware bound.
#[derive(Debug, Clone, PartialEq)]
pub enum DiscBound {
    /// Rational bounds; `exact` is true when no square root had to be rounded outward.
    Exact {
        /// Lower bound of the squared length.
        lo: Rational,
        /// Upper bound of the squared length.
        hi: Rational,
        /// True when `lo` and `hi` are the attained extremes.
        exact: bool,
    },
    /// Outward-rounded binary64 bounds of an enclosing region (tilted, elliptic or box-mixed).
    Outer {
        /// Lower bound of the squared length.
        lo: f64,
        /// Upper bound of the squared length.
        hi: f64,
    },
}

fn half() -> Rational {
    Rational::new(1, 2).expect("nonzero denominator")
}

fn sign(r: &Rational) -> Sign {
    r.numer().sign()
}

fn is_zero(r: &Rational) -> bool {
    sign(r) == Sign::NoSign
}

fn max0(r: Rational) -> Rational {
    if sign(&r) == Sign::Minus {
        Rational::from_int(0)
    } else {
        r
    }
}

fn sq(r: &Rational) -> Rational {
    r.mul(r)
}

/// Outward rational bounds of `sqrt(x)` for `x >= 0`, tight to `1 / (den * 2^32)`.
fn sqrt_bounds(x: &Rational) -> (Rational, Rational) {
    let scaled: BigInt = (x.numer() * x.denom()) << 64;
    let s = scaled.sqrt();
    let den: BigInt = x.denom() << 32;
    let lo = Rational::new(s.clone(), den.clone()).expect("positive denominator");
    if &s * &s == scaled {
        return (lo.clone(), lo);
    }
    (lo, Rational::new(s + 1, den).expect("positive denominator"))
}

fn rat(m: &Magnitude) -> Option<Rational> {
    (m.pi_exp() == 0).then(|| m.rational().clone())
}

fn as_mag(r: Rational) -> Magnitude {
    Magnitude::new(r, 0)
}

/// Builds a `[position diameter]` point: `nominal` plus a disc of that diameter in the plane.
pub fn position_point(
    datum: DatumId,
    nominal: &[Affine],
    diameter: &Affine,
    var: VarId,
) -> Result<Point, ZoneError> {
    if nominal.len() != 2 {
        return Err(ZoneError::Invalid(
            "a position zone is a disc in the plane; its nominal is a Point2".into(),
        ));
    }
    if !diameter.is_exact() || !nominal.iter().all(Affine::is_exact) {
        return Err(ZoneError::Invalid(
            "a position zone needs an exact nominal and an exact diameter".into(),
        ));
    }
    let dim = diameter.dim().clone();
    if nominal.iter().any(|n| n.dim() != &dim) {
        return Err(ZoneError::Invalid(
            "a position zone's diameter and nominal differ in dimension".into(),
        ));
    }
    if sign(diameter.center().rational()) == Sign::Minus {
        return Err(ZoneError::Invalid(
            "a position zone's diameter must not be negative".into(),
        ));
    }
    let radius = diameter.center().mul(&as_mag(half()));
    let zero = as_mag(Rational::from_int(0));
    let x = Affine::disc(
        dim.clone(),
        nominal[0].center().clone(),
        var,
        radius.clone(),
        zero.clone(),
    );
    let y = Affine::disc(dim, nominal[1].center().clone(), var, zero, radius);
    debug!(%datum, ?var, "zone: position point built");
    Ok(Point {
        datum,
        coords: vec![x, y],
    })
}

/// The `.x` / `.y` / `.z` projection of a point: one coordinate form, exact in its disc.
pub fn project(p: &Point, axis: Axis) -> Result<Affine, ZoneError> {
    let i = match axis {
        Axis::X => 0,
        Axis::Y => 1,
        Axis::Z => 2,
    };
    p.coords.get(i).cloned().ok_or_else(|| {
        ZoneError::Invalid(format!(
            "`.{}` of a point with {} coordinates",
            axis.name(),
            p.coords.len()
        ))
    })
}

/// Bounds the squared length of the vector `forms` where discs are present (`None` if none, or
/// if a pi factor forces the per-coordinate fallback).
pub fn disc_bound(forms: &[&Affine]) -> Result<Option<DiscBound>, ToleranceError> {
    let vars: BTreeSet<VarId> = forms.iter().flat_map(|f| f.disc_vars()).collect();
    if vars.is_empty() {
        return Ok(None);
    }
    let mut centers = Vec::with_capacity(forms.len());
    let mut boxes = Vec::with_capacity(forms.len());
    for f in forms {
        let (Some(c), Some(b)) = (rat(f.center()), rat(&f.box_radius()?)) else {
            debug!("zone: pi factor in a disc form, per-coordinate fallback");
            return Ok(None);
        };
        centers.push(c);
        boxes.push(b);
    }
    let zero = Rational::from_int(0);
    // Gram matrix [[p, q], [q, r]] of each disc's n x 2 coefficient columns.
    let mut grams = Vec::with_capacity(vars.len());
    let mut in_plane = true;
    for v in &vars {
        let (mut p, mut q, mut r) = (zero.clone(), zero.clone(), zero.clone());
        for (i, f) in forms.iter().enumerate() {
            let Some((a, b)) = f.disc_coefficients(*v) else {
                continue;
            };
            let (Some(a), Some(b)) = (rat(a), rat(b)) else {
                return Ok(None);
            };
            p = p.add(&sq(&a));
            q = q.add(&a.mul(&b));
            r = r.add(&sq(&b));
            in_plane &= i < 2 || (is_zero(&a) && is_zero(&b));
        }
        grams.push((p, q, r));
    }
    let no_box = boxes.iter().all(is_zero);
    let isotropic = grams.iter().all(|(p, q, r)| is_zero(q) && p == r);
    let c_sq = |range: std::ops::Range<usize>| {
        centers[range]
            .iter()
            .fold(zero.clone(), |s, c| s.add(&sq(c)))
    };
    if no_box && isotropic && in_plane {
        let n = centers.len();
        let (c_xy, c_rest) = (c_sq(0..n.min(2)), c_sq(n.min(2)..n));
        let (mut r_lo, mut r_hi) = (zero.clone(), zero.clone());
        for (p, _, _) in &grams {
            let (lo, hi) = sqrt_bounds(p);
            r_lo = r_lo.add(&lo);
            r_hi = r_hi.add(&hi);
        }
        let (m_lo, m_hi) = sqrt_bounds(&c_xy);
        let lo = c_rest.add(&sq(&max0(m_lo.sub(&r_hi))));
        let hi = c_rest.add(&sq(&m_hi.add(&r_hi)));
        let exact = r_lo == r_hi && m_lo == m_hi;
        debug!(exact, "zone: coplanar isotropic discs, exact range");
        return Ok(Some(DiscBound::Exact { lo, hi, exact }));
    }
    let f = |r: &Rational| r.to_f64();
    let mut radius = 0.0_f64;
    for (p, q, r) in &grams {
        let (p, q, r) = (f(p), f(q), f(r));
        let half_gap = (p - r) / 2.0;
        let lambda = pad_up((p + r) / 2.0 + pad_up((half_gap * half_gap + q * q).sqrt()));
        radius += pad_up(lambda.sqrt());
    }
    let box_norm = pad_up(boxes.iter().map(|b| f(b).powi(2)).sum::<f64>().sqrt());
    let c_norm = centers.iter().map(|c| f(c).powi(2)).sum::<f64>().sqrt();
    let reach = pad_up(radius + box_norm);
    let lo = pad_down((pad_down(c_norm) - reach).max(0.0).powi(2)).max(0.0);
    let hi = pad_up(pad_up(c_norm + reach).powi(2));
    trace!(
        lo,
        hi, "zone: tilted or mixed discs, conservative enclosure"
    );
    Ok(Some(DiscBound::Outer { lo, hi }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::length_dim;

    fn mm(n: i64, d: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, d * 1000).unwrap(), 0)
    }

    fn exact(n: i64) -> Affine {
        Affine::constant(length_dim(), mm(n, 1))
    }

    fn zone(var: u32, x: i64, y: i64, d_tenth: i64) -> Point {
        let dia = Affine::constant(length_dim(), mm(d_tenth, 10));
        position_point(
            DatumId::new("origin"),
            &[exact(x), exact(y)],
            &dia,
            VarId(var),
        )
        .unwrap()
    }

    fn f64_bounds(a: &Affine) -> (f64, f64) {
        let (lo, hi) = a.bounds().unwrap();
        (lo.to_f64(), hi.to_f64())
    }

    // frob:tests crates/cadetta-geometry/src/zone.rs::project kind=unit
    // frob:tests crates/cadetta-geometry/src/zone.rs::position_point kind=unit
    #[test]
    fn projection_is_half_the_diameter_about_the_nominal() {
        let p = zone(0, 10, 20, 1);
        let (lo, hi) = f64_bounds(&project(&p, Axis::X).unwrap());
        assert!(
            (lo - 0.00995).abs() < 1e-12 && (hi - 0.01005).abs() < 1e-12,
            "{lo} {hi}"
        );
        let (lo, hi) = f64_bounds(&project(&p, Axis::Y).unwrap());
        assert!(
            (lo - 0.01995).abs() < 1e-12 && (hi - 0.02005).abs() < 1e-12,
            "{lo} {hi}"
        );
        let at_origin = zone(1, 0, 0, 2);
        let x = project(&at_origin, Axis::X).unwrap();
        let (lo, hi) = x.bounds().unwrap();
        assert_eq!(lo, mm(-1, 10));
        assert_eq!(hi, mm(1, 10));
        assert!(project(&p, Axis::Z).is_err());
    }

    // frob:tests crates/cadetta-geometry/src/zone.rs::position_point kind=unit
    #[test]
    fn malformed_zones_are_invalid() {
        let d = Affine::constant(length_dim(), mm(1, 10));
        let three = [exact(0), exact(0), exact(0)];
        assert!(position_point(DatumId::new("o"), &three, &d, VarId(0)).is_err());
        let neg = Affine::constant(length_dim(), mm(-1, 10));
        assert!(position_point(DatumId::new("o"), &three[..2], &neg, VarId(0)).is_err());
    }

    // frob:tests crates/cadetta-geometry/src/zone.rs::disc_bound kind=unit
    #[test]
    fn two_coplanar_discs_reach_exactly_the_sum_of_radii() {
        let (p, q) = (zone(0, 0, 0, 1), zone(1, 0, 0, 1));
        let d: Vec<Affine> = p
            .coords
            .iter()
            .zip(&q.coords)
            .map(|(a, b)| a.sub(b).unwrap())
            .collect();
        let refs: Vec<&Affine> = d.iter().collect();
        let Some(DiscBound::Exact { lo, hi, exact }) = disc_bound(&refs).unwrap() else {
            panic!("coplanar isotropic discs are exact");
        };
        assert!(exact && is_zero(&lo));
        // (0.05 + 0.05)mm squared, in metres squared.
        assert_eq!(hi, Rational::new(1, 100_000_000).unwrap());
    }

    // frob:tests crates/cadetta-geometry/src/zone.rs::disc_bound kind=unit
    #[test]
    fn offset_discs_use_the_rational_square_root_when_it_exists() {
        // Centre offset 3,4 (norm 5) mm with radius 1mm: min 4mm, max 6mm.
        let p = zone(0, 3, 4, 20);
        let refs: Vec<&Affine> = p.coords.iter().collect();
        let Some(DiscBound::Exact { lo, hi, exact }) = disc_bound(&refs).unwrap() else {
            panic!("exact");
        };
        assert!(exact);
        assert_eq!(lo, Rational::new(16, 1_000_000).unwrap());
        assert_eq!(hi, Rational::new(36, 1_000_000).unwrap());
    }

    // frob:tests crates/cadetta-geometry/src/zone.rs::disc_bound kind=unit
    #[test]
    fn irrational_offsets_round_outward_and_are_not_exact() {
        let p = zone(0, 1, 1, 0);
        let q = zone(1, 0, 0, 20);
        let d: Vec<Affine> = p
            .coords
            .iter()
            .zip(&q.coords)
            .map(|(a, b)| a.sub(b).unwrap())
            .collect();
        let refs: Vec<&Affine> = d.iter().collect();
        let Some(DiscBound::Exact { lo, hi, exact }) = disc_bound(&refs).unwrap() else {
            panic!("plane discs");
        };
        assert!(!exact);
        // |c| = sqrt(2)mm, R = 1mm: max (sqrt(2) + 1)^2 = 5.828..mm2, min (sqrt(2) - 1)^2 = 0.1715..mm2.
        let (lo, hi) = (lo.to_f64() * 1e6, hi.to_f64() * 1e6);
        assert!(lo <= 0.171573 && lo > 0.17157, "{lo}");
        assert!((5.828427..5.82843).contains(&hi), "{hi}");
    }

    // frob:tests crates/cadetta-geometry/src/zone.rs::disc_bound kind=unit
    #[test]
    fn axial_offset_adds_outside_the_disc() {
        let p = zone(0, 0, 0, 20);
        let z = exact(3);
        let forms = [&p.coords[0], &p.coords[1], &z];
        let Some(DiscBound::Exact { lo, hi, .. }) = disc_bound(&forms).unwrap() else {
            panic!("exact");
        };
        // Disc radius 1mm, axial 3mm: 3^2 .. (1 + 0)^2 + 3^2.
        assert_eq!(lo, Rational::new(9, 1_000_000).unwrap());
        assert_eq!(hi, Rational::new(10, 1_000_000).unwrap());
    }

    // frob:tests crates/cadetta-geometry/src/zone.rs::disc_bound kind=unit
    #[test]
    fn a_tilted_disc_is_a_sound_conservative_enclosure() {
        // A disc whose columns reach z as well: image is an ellipse in a tilted plane.
        let dim = length_dim();
        let a = |x: i64| as_mag(Rational::new(x, 1000).unwrap());
        let zero = as_mag(Rational::from_int(0));
        let var = VarId(0);
        let fx = Affine::disc(dim.clone(), zero.clone(), var, a(1), zero.clone());
        let fy = Affine::disc(dim.clone(), zero.clone(), var, zero.clone(), a(1));
        let fz = Affine::disc(dim, zero.clone(), var, a(1), zero);
        let forms = [&fx, &fy, &fz];
        let Some(DiscBound::Outer { lo, hi }) = disc_bound(&forms).unwrap() else {
            panic!("tilted discs are conservative");
        };
        // True max length is sqrt(2) mm (u = 1): squared 2e-6; the enclosure must contain it.
        assert_eq!(lo, 0.0);
        assert!((2.0e-6..2.1e-6).contains(&hi), "{hi}");
    }

    // frob:tests crates/cadetta-geometry/src/zone.rs::disc_bound kind=unit
    #[test]
    fn box_terms_mixed_with_a_disc_are_conservative() {
        let p = zone(0, 0, 0, 20);
        let tol =
            cadetta_units::Toleranced::symmetric(length_dim(), mm(0, 1), (length_dim(), mm(1, 1)))
                .unwrap()
                .to_affine(VarId(5))
                .unwrap();
        let x = p.coords[0].add(&tol).unwrap();
        let forms = [&x, &p.coords[1]];
        let Some(DiscBound::Outer { hi, .. }) = disc_bound(&forms).unwrap() else {
            panic!("box + disc is conservative");
        };
        // Disc radius 1mm plus box 1mm: at worst 2mm apart, squared 4e-6.
        assert!((4.0e-6..4.1e-6).contains(&hi), "{hi}");
    }

    // frob:tests crates/cadetta-geometry/src/zone.rs::disc_bound kind=unit
    #[test]
    fn forms_without_discs_are_left_to_the_coordinate_path() {
        let forms = [&exact(1), &exact(2)];
        assert_eq!(disc_bound(&forms).unwrap(), None);
    }
}
