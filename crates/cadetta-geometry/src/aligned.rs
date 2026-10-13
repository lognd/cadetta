//! `distance()` and `aligned()` over tolerance ranges, and the G004 count check (geometry.md 3, 5, 7).
//!
//! Both reduce to one question about a difference vector: is its length within an allowance for
//! every value in tolerance? The bound is computed over the vector's coordinate forms by
//! [`sum_of_squares`], so a later disc term (geometry.md 3.1, position zones) only has to extend
//! the vector. Squares are square-aware (quantities.md 5.4): a coordinate straddling zero never
//! gives a negative lower bound. The verdict compares the squared length against the squared
//! allowance, which keeps exact inputs in exact arithmetic; the reported distance interval is the
//! square root, which is inexact and conservative.
// frob:ticket 01M4SWG6T58Q489WASV5DYGF7F

use std::fmt;

use cadetta_units::compare::{CompareDiagnostic, Comparison, Verdict};
use cadetta_units::square::square;
use cadetta_units::{Affine, CmpOp, InexactInterval, Operand, ToleranceError, compare};
use tracing::{debug, info, warn};

use crate::point::{DatumId, Pos, PositionError, point_sub};
use crate::reexpress::{FrameTable, Located, ReexpressError, Reexpressed};
use crate::transform::{Coord, TransformError};

/// The diagnostic code for mismatched element counts (geometry.md 7).
pub const G004: &str = "G004";

/// What went wrong measuring a distance.
#[derive(Debug, Clone, PartialEq)]
pub enum AlignedError {
    /// G004: the two operands of `aligned` have different element counts.
    G004 {
        /// What was attempted.
        message: String,
    },
    /// The operands do not make a distance (arity, shape, or a negative allowance).
    Invalid {
        /// What is wrong.
        message: String,
    },
    /// Position arithmetic failed (G001 across datums).
    Position(PositionError),
    /// Converting to the datum of `b` failed (G003 and so on).
    Reexpress(ReexpressError),
    /// A coordinate operation failed.
    Transform(TransformError),
    /// A tolerance operation failed.
    Tolerance(ToleranceError),
}

impl AlignedError {
    /// The diagnostic code, if this error has one.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            AlignedError::G004 { .. } => Some(G004),
            AlignedError::Position(e) => e.code(),
            AlignedError::Reexpress(e) => e.code(),
            _ => None,
        }
    }
}

impl fmt::Display for AlignedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlignedError::G004 { message } => write!(f, "{G004}: {message}"),
            AlignedError::Invalid { message } => f.write_str(message),
            AlignedError::Position(e) => write!(f, "{e}"),
            AlignedError::Reexpress(e) => write!(f, "{e}"),
            AlignedError::Transform(e) => write!(f, "{e}"),
            AlignedError::Tolerance(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AlignedError {}

impl From<PositionError> for AlignedError {
    fn from(e: PositionError) -> Self {
        AlignedError::Position(e)
    }
}

impl From<ReexpressError> for AlignedError {
    fn from(e: ReexpressError) -> Self {
        AlignedError::Reexpress(e)
    }
}

impl From<TransformError> for AlignedError {
    fn from(e: TransformError) -> Self {
        AlignedError::Transform(e)
    }
}

impl From<ToleranceError> for AlignedError {
    fn from(e: ToleranceError) -> Self {
        AlignedError::Tolerance(e)
    }
}

/// The squared length of a vector of coordinates, as a comparison operand.
#[derive(Debug, Clone)]
pub struct SquaredNorm {
    /// The squared length: an exact-arithmetic form, or a binary64 interval for inexact input.
    pub operand: Operand,
    /// True when any coordinate was inexact or an outer bound.
    pub conservative: bool,
}

/// One measured distance: its interval and whether it holds against the allowance.
#[derive(Debug, Clone)]
pub struct Measured {
    /// The distance over every value in tolerance (inexact and conservative).
    pub distance: InexactInterval,
    /// The comparison `distance <= allowance`.
    pub comparison: Comparison,
}

impl Measured {
    /// The diagnostics of the comparison (C002 when undecided).
    pub fn diagnostics(&self) -> Vec<CompareDiagnostic> {
        self.comparison.diagnostics()
    }
}

/// The result of `aligned(a, b, allowance)`: every pair, and the combined verdict.
#[derive(Debug, Clone)]
pub struct AlignedReport {
    /// One entry per pair `(a[i], b[i])`, in index order.
    pub pairs: Vec<Measured>,
    /// `Violates` if any pair does, else `Undecided` if any is, else `Holds`.
    pub verdict: Verdict,
}

impl AlignedReport {
    /// The diagnostics of every pair (C002 for each undecided one).
    pub fn diagnostics(&self) -> Vec<CompareDiagnostic> {
        self.pairs.iter().flat_map(Measured::diagnostics).collect()
    }
}

fn pad_down(x: f64) -> f64 {
    x - 4.0 * f64::EPSILON * x.abs().max(f64::MIN_POSITIVE)
}

fn pad_up(x: f64) -> f64 {
    x + 4.0 * f64::EPSILON * x.abs().max(f64::MIN_POSITIVE)
}

fn coord_bounds(c: &Coord) -> Result<(f64, f64), AlignedError> {
    match c {
        Coord::Approx(p) => Ok((p.lo, p.hi)),
        Coord::Exact(a) => {
            let (lo, hi) = a.bounds()?;
            Ok((lo.to_f64(), hi.to_f64()))
        }
    }
}

/// The squared length of a difference vector, square-aware per coordinate (quantities.md 5.4).
///
/// All-exact coordinates stay exact [`Affine`] forms; any binary64 coordinate switches to an
/// outward-padded interval. Disc terms for position zones (geometry.md 3.1) extend `diff`.
pub fn sum_of_squares(diff: &[Coord]) -> Result<SquaredNorm, AlignedError> {
    let Some(first) = diff.first() else {
        return Err(AlignedError::Invalid {
            message: "a distance needs at least one coordinate".into(),
        });
    };
    let sq_dim = first.dim().mul(first.dim());
    if diff.iter().all(Coord::is_exact) {
        let mut total: Option<Affine> = None;
        for c in diff {
            let Coord::Exact(a) = c else { unreachable!() };
            let s = square(a)?;
            total = Some(match total {
                None => s,
                Some(t) => t.add(&s)?,
            });
        }
        let operand = total.expect("diff is non-empty");
        let conservative = operand.is_conservative();
        debug!(conservative, "aligned: exact sum of squares");
        return Ok(SquaredNorm {
            operand: Operand::Form(operand),
            conservative,
        });
    }
    let (mut lo, mut hi) = (0.0_f64, 0.0_f64);
    for c in diff {
        let (a, b) = coord_bounds(c)?;
        let (slo, shi) = if a <= 0.0 && b >= 0.0 {
            (0.0, a.abs().max(b.abs()).powi(2))
        } else {
            let (x, y) = (a.abs().min(b.abs()), a.abs().max(b.abs()));
            (x * x, y * y)
        };
        lo += pad_down(slo).max(0.0);
        hi += pad_up(shi);
    }
    debug!(lo, hi, "aligned: inexact sum of squares (conservative)");
    Ok(SquaredNorm {
        operand: Operand::Interval(
            sq_dim,
            InexactInterval {
                lo: pad_down(lo).max(0.0),
                hi: pad_up(hi),
                conservative: true,
            },
        ),
        conservative: true,
    })
}

fn operand_bounds(o: &Operand) -> Result<(f64, f64), AlignedError> {
    match o {
        Operand::Form(a) => {
            let (lo, hi) = a.bounds()?;
            Ok((lo.to_f64(), hi.to_f64()))
        }
        Operand::Interval(_, i) => Ok((i.lo, i.hi)),
    }
}

/// Measures a difference vector against a length allowance (`distance(..) <= allowance`).
pub fn measure(diff: &[Coord], allowance: &Affine) -> Result<Measured, AlignedError> {
    let (alo, _) = allowance.bounds()?;
    if alo.to_f64() < 0.0 {
        return Err(AlignedError::Invalid {
            message: "an allowance must not be negative".into(),
        });
    }
    let norm = sum_of_squares(diff)?;
    let (lo, hi) = operand_bounds(&norm.operand)?;
    let distance = InexactInterval {
        lo: pad_down(lo.max(0.0).sqrt()).max(0.0),
        hi: pad_up(hi.sqrt()),
        conservative: true,
    };
    let allowed = square(allowance)?;
    let comparison = compare(CmpOp::Le, &norm.operand, &Operand::Form(allowed))?;
    debug!(
        lo = distance.lo,
        hi = distance.hi,
        verdict = ?comparison.verdict,
        "aligned: distance measured"
    );
    Ok(Measured {
        distance,
        comparison,
    })
}

fn located_diff(a: &Located, b: &Located) -> Result<Vec<Coord>, AlignedError> {
    if a.datum != b.datum {
        warn!(a = %a.datum, b = %b.datum, "aligned: distance across datums");
        return Err(PositionError::G001 {
            message: format!(
                "distance between points in `{}` and `{}`; use `in`",
                a.datum, b.datum
            ),
        }
        .into());
    }
    a.coords
        .iter()
        .zip(&b.coords)
        .map(|(x, y)| x.add(&y.neg()).map_err(AlignedError::from))
        .collect()
}

/// `distance(p, q)` for two points in one datum: the full length, axial component included.
pub fn distance(
    p: &crate::point::Point,
    q: &crate::point::Point,
    allowance: &Affine,
) -> Result<Measured, AlignedError> {
    if p.coords.len() != q.coords.len() {
        return Err(AlignedError::Invalid {
            message: "distance of a Point2 and a Point3; lift the plane point first".into(),
        });
    }
    let v = point_sub(p, q)?;
    let diff: Vec<Coord> = v.comps.into_iter().map(Coord::Exact).collect();
    measure(&diff, allowance)
}

/// `distance(p, q)` for two re-expressed points in one datum, axial component included.
pub fn distance_located(
    p: &Located,
    q: &Located,
    allowance: &Affine,
) -> Result<Measured, AlignedError> {
    measure(&located_diff(p, q)?, allowance)
}

fn flatten<'a>(p: &'a Pos, out: &mut Vec<&'a crate::point::Point>) -> Result<(), AlignedError> {
    match p {
        Pos::Point(pt) => out.push(pt),
        Pos::Array(items) => {
            for i in items {
                match i {
                    Pos::Point(pt) => out.push(pt),
                    _ => {
                        return Err(AlignedError::Invalid {
                            message: "aligned takes a point or a flat array of points".into(),
                        });
                    }
                }
            }
        }
        Pos::Vector(_) => {
            return Err(AlignedError::Invalid {
                message: "aligned takes points, not vectors".into(),
            });
        }
    }
    Ok(())
}

fn to_datum(
    table: &FrameTable,
    p: &crate::point::Point,
    datum: &DatumId,
) -> Result<Located, AlignedError> {
    match table.reexpress(&Pos::Point(p.clone()), datum)? {
        Reexpressed::Point(l) => Ok(l),
        Reexpressed::Array(_) => Err(AlignedError::Invalid {
            message: "a point re-expressed to an array".into(),
        }),
    }
}

/// `aligned(a, b, allowance)`: pairs by index, both in the datum of `b`, xy plane only (geometry.md 5).
///
/// Different element counts are G004. A bare point counts as one element. The z coordinate of
/// the datum of `b` is ignored, so axial offset never fails an alignment.
pub fn aligned(
    table: &FrameTable,
    a: &Pos,
    b: &Pos,
    allowance: &Affine,
) -> Result<AlignedReport, AlignedError> {
    let (mut pa, mut pb) = (Vec::new(), Vec::new());
    flatten(a, &mut pa)?;
    flatten(b, &mut pb)?;
    if pa.len() != pb.len() {
        warn!(
            a = pa.len(),
            b = pb.len(),
            "aligned: element count mismatch (G004)"
        );
        return Err(AlignedError::G004 {
            message: format!(
                "aligned takes equal element counts, got {} and {}",
                pa.len(),
                pb.len()
            ),
        });
    }
    let mut pairs = Vec::with_capacity(pa.len());
    for (i, (x, y)) in pa.iter().zip(&pb).enumerate() {
        let to = &y.datum;
        let lx = to_datum(table, x, to)?;
        let ly = to_datum(table, y, to)?;
        let mut diff = located_diff(&lx, &ly)?;
        diff.truncate(2);
        let m = measure(&diff, allowance)?;
        debug!(pair = i, verdict = ?m.comparison.verdict, "aligned: pair measured");
        pairs.push(m);
    }
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
    info!(pairs = pairs.len(), ?verdict, "aligned: evaluated");
    Ok(AlignedReport { pairs, verdict })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::length_dim;
    use crate::placement::tests::{build, eval, src};
    use crate::point::Point;
    use crate::transform::Approx;
    use cadetta_units::{Magnitude, Rational, Toleranced, VarId};

    /// Metres from micrometres.
    fn um(n: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, 1_000_000).unwrap(), 0)
    }

    fn exact(n_um: i64) -> Affine {
        Affine::constant(length_dim(), um(n_um))
    }

    /// `nominal +- tol` micrometres on source variable `v`.
    fn var(v: u32, nominal: i64, tol: i64) -> Affine {
        Toleranced::symmetric(length_dim(), um(nominal), (length_dim(), um(tol)))
            .unwrap()
            .to_affine(VarId(v))
            .unwrap()
    }

    fn pt(datum: &str, c: &[Affine]) -> Point {
        Point {
            datum: DatumId::new(datum),
            coords: c.to_vec(),
        }
    }

    fn pos(datum: &str, c: &[(i64, i64)]) -> Pos {
        Pos::Array(
            c.iter()
                .map(|(x, y)| Pos::Point(pt(datum, &[exact(*x), exact(*y)])))
                .collect(),
        )
    }

    fn rail_table() -> FrameTable {
        let a = build(
            &src("assembly Mount {\n  part rail: Rail\n  place rail.origin at origin\n}\n"),
            &[],
        );
        FrameTable::for_part(&a.nodes[0].datums, &eval).unwrap()
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::distance kind=unit
    #[test]
    fn section_one_holds_within_tolerance() {
        // Four independent +-0.05mm coordinates: each difference is in [-0.1, 0.1]mm.
        let p = pt("origin", &[var(0, 0, 50), var(1, 0, 50)]);
        let q = pt("origin", &[var(2, 0, 50), var(3, 0, 50)]);
        let m = distance(&p, &q, &exact(250)).unwrap();
        assert_eq!(m.distance.lo, 0.0);
        assert!(m.distance.hi >= 0.1414e-3 && m.distance.hi < 0.14143e-3);
        assert!(m.distance.conservative);
        assert_eq!(m.comparison.verdict, Verdict::Holds);
        assert!(m.diagnostics().is_empty());
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::distance kind=unit
    #[test]
    fn section_one_wide_tolerance_is_c002_not_violation() {
        // +-0.2mm against +-0.1mm: differences reach +-0.3mm, so 0.25mm is neither held nor broken.
        let p = pt("origin", &[var(0, 0, 200), var(1, 0, 200)]);
        let q = pt("origin", &[var(2, 0, 100), var(3, 0, 100)]);
        let m = distance(&p, &q, &exact(250)).unwrap();
        assert_eq!(m.comparison.verdict, Verdict::Undecided);
        let d = m.diagnostics();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "C002");
        assert!(d[0].message.contains("conservative"), "{}", d[0].message);
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::distance kind=unit
    #[test]
    fn nominal_offset_beyond_allowance_violates() {
        let p = pt("origin", &[exact(500), exact(0)]);
        let q = pt("origin", &[exact(0), exact(0)]);
        let m = distance(&p, &q, &exact(250)).unwrap();
        assert_eq!(m.comparison.verdict, Verdict::Violates);
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::distance kind=unit
    #[test]
    fn distance_includes_the_axial_component() {
        let p = pt("origin", &[exact(0), exact(0), exact(300)]);
        let q = pt("origin", &[exact(0), exact(0), exact(0)]);
        let m = distance(&p, &q, &exact(250)).unwrap();
        assert_eq!(m.comparison.verdict, Verdict::Violates);
        let flat = distance(&pt("origin", &[exact(0), exact(0)]), &q, &exact(250));
        assert!(matches!(flat, Err(AlignedError::Invalid { .. })));
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::distance kind=unit
    #[test]
    fn distance_across_datums_is_g001() {
        let p = pt("top", &[exact(0), exact(0)]);
        let q = pt("origin", &[exact(0), exact(0)]);
        let e = distance(&p, &q, &exact(250)).unwrap_err();
        assert_eq!(e.code(), Some("G001"));
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::measure kind=unit
    #[test]
    fn negative_allowance_is_invalid() {
        let p = pt("origin", &[exact(0), exact(0)]);
        let e = distance(&p, &p, &exact(-1)).unwrap_err();
        assert!(matches!(e, AlignedError::Invalid { .. }));
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::sum_of_squares kind=unit
    #[test]
    fn inexact_coordinate_gives_a_conservative_interval() {
        let approx = Coord::Approx(Approx {
            lo: 0.1e-3,
            hi: 0.1e-3,
            root: None,
            conservative: false,
            dim: length_dim(),
        });
        let diff = [approx, Coord::constant(length_dim(), 0)];
        let n = sum_of_squares(&diff).unwrap();
        assert!(n.conservative);
        assert!(matches!(n.operand, Operand::Interval(..)));
        let m = measure(&diff, &exact(250)).unwrap();
        assert_eq!(m.comparison.verdict, Verdict::Holds);
        assert!(m.distance.conservative && m.distance.lo <= 0.1e-3 && m.distance.hi >= 0.1e-3);
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::aligned kind=unit
    #[test]
    fn aligned_pairs_by_index() {
        let t = rail_table();
        let a = pos("origin", &[(-25_000, 0), (25_000, 0)]);
        let same = pos("origin", &[(-25_000, 0), (25_000, 0)]);
        let swapped = pos("origin", &[(25_000, 0), (-25_000, 0)]);
        let ok = aligned(&t, &a, &same, &exact(250)).unwrap();
        assert_eq!(ok.verdict, Verdict::Holds);
        assert_eq!(ok.pairs.len(), 2);
        let bad = aligned(&t, &a, &swapped, &exact(250)).unwrap();
        assert_eq!(bad.verdict, Verdict::Violates);
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::aligned kind=unit
    #[test]
    fn aligned_ignores_z_and_converts_to_the_datum_of_b() {
        let t = rail_table();
        // `site` sits at x = 40mm, z = 30mm of the rail origin; the axial 30mm must not count.
        let a = Pos::Point(pt("site", &[exact(1_000), exact(2_000)]));
        let b = Pos::Point(pt("origin", &[exact(41_000), exact(2_000)]));
        let r = aligned(&t, &a, &b, &exact(250)).unwrap();
        assert_eq!(r.verdict, Verdict::Holds);
        // Distance with the axial component kept is the 30mm the plane test ignores.
        let la = match t.reexpress(&a, &DatumId::new("origin")).unwrap() {
            Reexpressed::Point(l) => l,
            _ => panic!("point"),
        };
        let lb = match t.reexpress(&b, &DatumId::new("origin")).unwrap() {
            Reexpressed::Point(l) => l,
            _ => panic!("point"),
        };
        let full = distance_located(&la, &lb, &exact(250)).unwrap();
        assert_eq!(full.comparison.verdict, Verdict::Violates);
        // Converting the other way, into `site`, gives the same plane answer.
        let r2 = aligned(&t, &b, &a, &exact(250)).unwrap();
        assert_eq!(r2.verdict, Verdict::Holds);
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::aligned kind=unit
    #[test]
    fn aligned_reports_c002_for_an_undecided_pair() {
        let t = rail_table();
        let a = Pos::Point(pt("origin", &[var(0, 0, 200), var(1, 0, 200)]));
        let b = Pos::Point(pt("origin", &[var(2, 0, 100), var(3, 0, 100)]));
        let r = aligned(&t, &a, &b, &exact(250)).unwrap();
        assert_eq!(r.verdict, Verdict::Undecided);
        let d = r.diagnostics();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "C002");
        let tight = aligned(
            &t,
            &Pos::Point(pt("origin", &[var(0, 0, 50), var(1, 0, 50)])),
            &Pos::Point(pt("origin", &[var(2, 0, 50), var(3, 0, 50)])),
            &exact(250),
        )
        .unwrap();
        assert_eq!(tight.verdict, Verdict::Holds);
    }

    // frob:tests crates/cadetta-geometry/src/aligned.rs::aligned kind=unit
    #[test]
    fn aligned_count_mismatch_is_g004() {
        let t = rail_table();
        let three = pos("origin", &[(0, 0), (10_000, 0), (20_000, 0)]);
        let two = pos("origin", &[(0, 0), (10_000, 0)]);
        let e = aligned(&t, &three, &two, &exact(250)).unwrap_err();
        assert_eq!(e.code(), Some("G004"));
        assert!(e.to_string().starts_with("G004"), "{e}");
        assert!(aligned(&t, &two, &two, &exact(250)).is_ok());
    }
}
