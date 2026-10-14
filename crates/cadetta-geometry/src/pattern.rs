//! Pattern generators `rect_pattern` and `circle_pattern` (geometry.md 3; ir.md 2).
//!
//! A generator is a value with named members: `count_x`, `count_y`, `pitch_x`, `pitch_y`,
//! `rotation` for a rectangular pattern, `count`, `radius` and `start` for a circular one. Those
//! members bind and emit (they lower to authored `Param` IR members); the derived coordinates
//! are analysis-only (`Let`, derived provenance), never emitted. The point counts (`nx`, `ny`,
//! `n`) are structural as well as members: each must be an exact whole number known before
//! evaluation (a literal or a parameter-free constant), see [`structural_count`]; one that is
//! not is E006.
//!
//! Decisions made here (written into geometry.md 3): points carry [`Coord`]s, so a rotation by
//! a whole multiple of 90deg keeps them exact and any other angle gives inexact intervals with
//! the same rules as datum rotations; a generator's tolerated member shares one source variable
//! across the points it moves (so the pitch between neighbours varies by the written tolerance,
//! not twice it); [`tolerate_each`] adds one fresh variable per coordinate of every point; an
//! omitted `rotation` has no member; a zero count is an error.

// frob:ticket 01M4SWFXADMC8C5Y451JRVBYZM
// frob:ticket 01M4VB63B9FS1JDEH7V08K12NH

use cadetta_ir::canon::{CanonPath, Index, MemberSeg};
use cadetta_ir::model::{Member as IrMember, MemberKind, Provenance};
use cadetta_syntax::ast::Expr;
use cadetta_syntax::span::Span;
use cadetta_units::{Affine, Magnitude, Rational, ToleranceError, Toleranced, VarId};
use thiserror::Error;
use tracing::{debug, warn};

use crate::datum::{Axis, GeometryError, Member};
use crate::op_members::{OpSource, ir_path, lower_member};
use crate::point::DatumId;
use crate::transform::{Coord, TransformError, cos_sin};

/// Why a pattern could not be evaluated.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PatternError {
    /// A point count was zero.
    #[error("pattern count `{0}` must be at least 1")]
    EmptyCount(&'static str),
    /// Coordinate arithmetic failed (dimension mismatch, bad angle).
    #[error("pattern arithmetic: {0}")]
    Transform(#[from] TransformError),
    /// A tolerance operation failed.
    #[error("pattern tolerance: {0}")]
    Tolerance(#[from] ToleranceError),
}

/// One generated point: planar coordinates in the datum of the declared type.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternPoint {
    /// The datum the pattern is on.
    pub datum: DatumId,
    /// The x and y coordinates.
    pub coords: [Coord; 2],
}

/// `rect_pattern(nx, ny, dx, dy, rotation = 0deg)` with evaluated arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct RectPattern {
    /// Points along x.
    pub nx: u32,
    /// Points along y.
    pub ny: u32,
    /// Centre-to-centre pitch along x (`pitch_x`).
    pub dx: Affine,
    /// Centre-to-centre pitch along y (`pitch_y`).
    pub dy: Affine,
    /// Rotation about the datum origin (`rotation`); `None` is 0deg.
    pub rotation: Option<Affine>,
}

/// `circle_pattern(n, r, start)` with evaluated arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct CirclePattern {
    /// Number of points.
    pub n: u32,
    /// Pattern radius (`radius`).
    pub r: Affine,
    /// Angle of the first point (`start`).
    pub start: Affine,
}

fn scalar(num: i64, den: i64) -> Affine {
    let r = Rational::new(num, den).expect("nonzero denominator");
    Affine::constant(Default::default(), Magnitude::new(r, 0))
}

fn count(n: u32, name: &'static str) -> Result<u32, PatternError> {
    if n == 0 {
        warn!(name, "pattern count is zero");
        return Err(PatternError::EmptyCount(name));
    }
    Ok(n)
}

impl RectPattern {
    /// Points row by row, x fastest, (-,-) to (+,+), centred on the datum origin.
    pub fn points(&self, datum: &DatumId) -> Result<Vec<PatternPoint>, PatternError> {
        let (nx, ny) = (count(self.nx, "nx")?, count(self.ny, "ny")?);
        let rot = self.rotation.as_ref().map(cos_sin).transpose()?;
        debug!(nx, ny, rotated = rot.is_some(), "rect_pattern");
        let mut out = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                // Offset from the centre in half pitches: (2i - (nx - 1)) / 2.
                let fx = scalar(2 * i64::from(i) - (i64::from(nx) - 1), 2);
                let fy = scalar(2 * i64::from(j) - (i64::from(ny) - 1), 2);
                let (x, y) = (
                    Coord::Exact(self.dx.mul(&fx)?),
                    Coord::Exact(self.dy.mul(&fy)?),
                );
                let coords = match &rot {
                    None => [x, y],
                    Some((c, s)) => [
                        c.mul(&x)?.add(&s.mul(&y)?.neg())?,
                        s.mul(&x)?.add(&c.mul(&y)?)?,
                    ],
                };
                out.push(PatternPoint {
                    datum: datum.clone(),
                    coords,
                });
            }
        }
        Ok(out)
    }
}

impl CirclePattern {
    /// Points counter-clockwise from `start`, evenly spaced on radius `r` about the origin.
    pub fn points(&self, datum: &DatumId) -> Result<Vec<PatternPoint>, PatternError> {
        let n = count(self.n, "n")?;
        debug!(n, "circle_pattern");
        let mut out = Vec::new();
        for k in 0..n {
            let step = Magnitude::new(
                Rational::new(2 * i64::from(k), i64::from(n)).expect("n is nonzero"),
                1,
            );
            let angle = self
                .start
                .add(&Affine::constant(self.start.dim().clone(), step))?;
            let (c, s) = cos_sin(&angle)?;
            let r = Coord::Exact(self.r.clone());
            out.push(PatternPoint {
                datum: datum.clone(),
                coords: [r.mul(&c)?, r.mul(&s)?],
            });
        }
        Ok(out)
    }
}

/// Applies a tolerance written after a pattern: every coordinate of every point gets its own
/// fresh source variable (ids from `first_var` upward), so a pitch varies by twice `deviation`.
///
/// `deviation` is the written deviation about zero (`[+-0.05mm]` is nominal 0 with +-0.05mm).
pub fn tolerate_each(
    points: &[PatternPoint],
    deviation: &Toleranced,
    first_var: u32,
) -> Result<Vec<PatternPoint>, PatternError> {
    let mut next = first_var;
    let mut fresh = || {
        let v = VarId(next);
        next += 1;
        v
    };
    let mut out = Vec::with_capacity(points.len());
    for p in points {
        let [x, y] = &p.coords;
        let dx = Coord::Exact(deviation.to_affine(fresh())?);
        let dy = Coord::Exact(deviation.to_affine(fresh())?);
        out.push(PatternPoint {
            datum: p.datum.clone(),
            coords: [x.add(&dx)?, y.add(&dy)?],
        });
    }
    debug!(
        points = out.len(),
        vars = next - first_var,
        "tolerance after pattern"
    );
    Ok(out)
}

/// Which generator a call names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Generator {
    /// `rect_pattern`.
    Rect,
    /// `circle_pattern`.
    Circle,
}

impl Generator {
    /// Parses the generator function name.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "rect_pattern" => Some(Self::Rect),
            "circle_pattern" => Some(Self::Circle),
            _ => None,
        }
    }

    /// Member names for the positional arguments, counts first.
    fn arg_names(self) -> &'static [Option<&'static str>] {
        match self {
            Self::Rect => &[
                Some("count_x"),
                Some("count_y"),
                Some("pitch_x"),
                Some("pitch_y"),
                Some("rotation"),
            ],
            Self::Circle => &[Some("count"), Some("radius"), Some("start")],
        }
    }

    /// Number of required positional arguments (`rotation` is optional).
    fn required(self) -> usize {
        match self {
            Self::Rect => 4,
            Self::Circle => 3,
        }
    }
}

/// Evaluates a pattern count that must be known before evaluation: an exact whole number
/// of at least 1 from integer literals, parentheses and `+`, `-`, `*` (geometry.md 3).
///
/// Anything else (a parameter, a quantity, a fraction, zero) is E006.
pub fn structural_count(e: &Expr) -> Result<u32, GeometryError> {
    fn fail(e: &Expr, why: &str) -> GeometryError {
        let message = format!(
            "E006: pattern count must be an exact whole number known before evaluation: {why}"
        );
        warn!(%message, "structural count");
        GeometryError {
            span: e.span,
            message,
        }
    }
    fn go(e: &Expr) -> Result<i64, GeometryError> {
        use cadetta_syntax::ast::{BinOp, ExprKind};
        match &e.kind {
            ExprKind::Number(s) => s
                .replace('_', "")
                .parse::<i64>()
                .map_err(|_| fail(e, "not a whole-number literal")),
            ExprKind::Paren(inner) => go(inner),
            ExprKind::Binary(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul), a, b) => {
                let (a, b) = (go(a)?, go(b)?);
                let r = match op {
                    BinOp::Add => a.checked_add(b),
                    BinOp::Sub => a.checked_sub(b),
                    _ => a.checked_mul(b),
                };
                r.ok_or_else(|| fail(e, "overflow"))
            }
            _ => Err(fail(e, "not a literal or parameter-free constant")),
        }
    }
    let n = go(e)?;
    let n = u32::try_from(n).map_err(|_| fail(e, "out of range"))?;
    if n == 0 {
        return Err(fail(e, "a count of zero"));
    }
    debug!(n, "structural count");
    Ok(n)
}

/// Lowers the arguments of a generator call bound at `base` (e.g. `Bracket.holes`) to members
/// `base.count_x`, `base.pitch_x` and so on, in argument order, counts included (geometry.md 3).
///
/// Each count argument must be structural ([`structural_count`]), else E006.
pub fn generator_members(
    base: &CanonPath,
    generator: Generator,
    args: &[Expr],
    call: Span,
) -> Result<Vec<Member>, GeometryError> {
    let names = generator.arg_names();
    if args.len() < generator.required() || args.len() > names.len() {
        let message = format!(
            "{generator:?} pattern takes {} to {} arguments, got {}",
            generator.required(),
            names.len(),
            args.len()
        );
        warn!(%message, "generator arity");
        return Err(GeometryError {
            span: call,
            message,
        });
    }
    let mut out = Vec::new();
    for (arg, name) in args.iter().zip(names) {
        let Some(name) = name else { continue };
        if name.starts_with("count") {
            structural_count(arg)?;
        }
        let mut path = base.clone();
        path.members.push(MemberSeg::Name((*name).to_owned()));
        debug!(%path, "generator member");
        out.push(Member {
            path,
            value: arg.clone(),
        });
    }
    Ok(out)
}

/// The generator members as authored `Param` IR members: they bind and emit.
pub fn generator_ir_members(
    members: &[Member],
    from: &OpSource<'_>,
) -> Result<Vec<IrMember>, GeometryError> {
    members
        .iter()
        .map(|m| lower_member(m, MemberKind::Param, from))
        .collect()
}

/// Derived coordinates of `points_count` points at `base` (`holes[i].x`, `.y`) as analysis-only
/// IR members: kind `Let`, derived provenance, never emitted or bound (geometry.md 3).
pub fn derived_coordinate_members(
    base: &CanonPath,
    points_count: usize,
) -> Result<Vec<IrMember>, GeometryError> {
    let mut out = Vec::new();
    for i in 0..points_count {
        for axis in [Axis::X, Axis::Y] {
            let mut path = base.clone();
            path.members.push(MemberSeg::Index(Index::At(i as u64)));
            path.members.push(MemberSeg::Name(axis.name().to_owned()));
            let ir = ir_path(&path).map_err(|message| GeometryError {
                span: Span::new(0, 0),
                message,
            })?;
            let mut m = IrMember::new(ir, MemberKind::Let);
            m.provenance = Provenance::Derived;
            out.push(m);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::ExprKind;
    use cadetta_units::{Dim, Dimension};

    fn len() -> Dim {
        let mut si = [0; cadetta_units::SI_BASE_COUNT];
        si[1] = 1;
        Dim::new(Dimension::from_si(si), 0)
    }

    fn mm(n: i64) -> Affine {
        Affine::constant(len(), Magnitude::new(Rational::from_int(n), 0))
    }

    fn deg90() -> Affine {
        let angle = Dim::new(Dimension::default(), 1);
        let half = Rational::new(1, 2).unwrap();
        Affine::constant(angle, Magnitude::new(half, 1))
    }

    fn center(c: &Coord) -> Magnitude {
        match c {
            Coord::Exact(a) => a.center().clone(),
            Coord::Approx(_) => panic!("exact expected"),
        }
    }

    fn xy(p: &PatternPoint) -> (Magnitude, Magnitude) {
        (center(&p.coords[0]), center(&p.coords[1]))
    }

    fn m(n: i64) -> Magnitude {
        Magnitude::new(Rational::from_int(n), 0)
    }

    fn top() -> DatumId {
        DatumId::new("top")
    }

    fn rect(nx: u32, ny: u32) -> RectPattern {
        RectPattern {
            nx,
            ny,
            dx: mm(30),
            dy: mm(40),
            rotation: None,
        }
    }

    // frob:tests crates/cadetta-geometry/src/pattern.rs::RectPattern kind=unit
    #[test]
    fn rect_points_run_row_by_row_centred_on_the_origin() {
        let pts = rect(2, 2).points(&top()).unwrap();
        let got: Vec<_> = pts.iter().map(xy).collect();
        assert_eq!(
            got,
            [
                (m(-15), m(-20)),
                (m(15), m(-20)),
                (m(-15), m(20)),
                (m(15), m(20))
            ]
        );
        assert!(pts.iter().all(|p| p.datum == top()));
        let row = rect(3, 1).points(&top()).unwrap();
        let xs: Vec<_> = row.iter().map(|p| center(&p.coords[0])).collect();
        assert_eq!(xs, [m(-30), m(0), m(30)]);
        assert!(rect(0, 1).points(&top()).is_err());
    }

    // frob:tests crates/cadetta-geometry/src/pattern.rs::RectPattern kind=unit
    #[test]
    fn rotation_by_a_quarter_turn_stays_exact() {
        let mut r = rect(2, 1);
        r.rotation = Some(deg90());
        let pts = r.points(&top()).unwrap();
        let got: Vec<_> = pts.iter().map(xy).collect();
        assert_eq!(got, [(m(0), m(-15)), (m(0), m(15))]);
    }

    // frob:tests crates/cadetta-geometry/src/pattern.rs::CirclePattern kind=unit
    #[test]
    fn circle_points_are_evenly_spaced_from_start() {
        let c = CirclePattern {
            n: 4,
            r: mm(10),
            start: Affine::constant(
                Dim::new(Dimension::default(), 1),
                Magnitude::new(Rational::from_int(0), 0),
            ),
        };
        let pts = c.points(&top()).unwrap();
        let got: Vec<_> = pts.iter().map(xy).collect();
        assert_eq!(
            got,
            [(m(10), m(0)), (m(0), m(10)), (m(-10), m(0)), (m(0), m(-10))]
        );
    }

    // frob:tests crates/cadetta-geometry/src/pattern.rs::tolerate_each kind=unit
    #[test]
    fn tolerance_after_a_pattern_is_one_variable_per_coordinate() {
        let dev = Toleranced::symmetric(
            len(),
            m(0),
            (len(), Magnitude::new(Rational::new(1, 20).unwrap(), 0)),
        )
        .unwrap();
        let pts = rect(2, 1).points(&top()).unwrap();
        let tol = tolerate_each(&pts, &dev, 0).unwrap();
        let (Coord::Exact(a), Coord::Exact(b)) = (&tol[0].coords[0], &tol[1].coords[0]) else {
            panic!("exact expected");
        };
        let (lo, hi) = b.sub(a).unwrap().bounds().unwrap();
        // Pitch 30mm +- 2 * 0.05mm: independent variables add.
        assert_eq!(lo, Magnitude::new(Rational::new(299, 10).unwrap(), 0));
        assert_eq!(hi, Magnitude::new(Rational::new(301, 10).unwrap(), 0));
    }

    // frob:tests crates/cadetta-geometry/src/pattern.rs::RectPattern kind=unit
    #[test]
    fn a_toleranced_pitch_member_tolerates_the_pitch_itself() {
        let dx = Toleranced::symmetric(
            len(),
            m(30),
            (len(), Magnitude::new(Rational::new(1, 20).unwrap(), 0)),
        )
        .unwrap()
        .to_affine(VarId(0))
        .unwrap();
        let mut r = rect(2, 1);
        r.dx = dx;
        let pts = r.points(&top()).unwrap();
        let (Coord::Exact(a), Coord::Exact(b)) = (&pts[0].coords[0], &pts[1].coords[0]) else {
            panic!("exact expected");
        };
        let (lo, hi) = b.sub(a).unwrap().bounds().unwrap();
        assert_eq!(lo, Magnitude::new(Rational::new(2995, 100).unwrap(), 0));
        assert_eq!(hi, Magnitude::new(Rational::new(3005, 100).unwrap(), 0));
    }

    fn num(s: &str) -> Expr {
        Expr {
            kind: ExprKind::Number(s.into()),
            span: Span::new(0, 0),
        }
    }

    fn path_expr(name: &str) -> Expr {
        Expr {
            kind: ExprKind::Str(name.into()),
            span: Span::new(0, 0),
        }
    }

    // frob:tests crates/cadetta-geometry/src/pattern.rs::structural_count kind=unit
    #[test]
    fn structural_counts_are_whole_numbers_known_before_evaluation() {
        assert_eq!(structural_count(&num("4")).unwrap(), 4);
        assert!(structural_count(&num("0")).is_err());
        assert!(structural_count(&num("2.5")).is_err());
        assert!(structural_count(&path_expr("n")).is_err());
    }

    fn holes() -> CanonPath {
        let mut p = CanonPath::item("acme", &[], "Bracket");
        p.members.push(MemberSeg::Name("holes".into()));
        p
    }

    // frob:tests crates/cadetta-geometry/src/pattern.rs::generator_members kind=unit
    #[test]
    fn generator_members_bind_and_derived_coordinates_are_analysis_only() {
        let args: Vec<Expr> = ["2", "2", "30", "40"].iter().map(|s| num(s)).collect();
        let ms = generator_members(&holes(), Generator::Rect, &args, Span::new(0, 0)).unwrap();
        let from = OpSource {
            file: "b.cdt",
            src: "",
        };
        let ir = generator_ir_members(&ms, &from).unwrap();
        let names: Vec<_> = ir.iter().map(|m| m.path.to_string()).collect();
        assert_eq!(
            names,
            [
                "acme::Bracket.holes.count_x",
                "acme::Bracket.holes.count_y",
                "acme::Bracket.holes.pitch_x",
                "acme::Bracket.holes.pitch_y"
            ]
        );
        assert!(
            ir.iter()
                .all(|m| m.kind == MemberKind::Param && !m.analysis)
        );

        let circle: Vec<Expr> = ["6", "10", "0"].iter().map(|s| num(s)).collect();
        let cm = generator_members(&holes(), Generator::Circle, &circle, Span::new(0, 0)).unwrap();
        let names: Vec<_> = cm.iter().map(|m| m.path.to_string()).collect();
        assert_eq!(
            names,
            [
                "acme::Bracket.holes.count",
                "acme::Bracket.holes.radius",
                "acme::Bracket.holes.start"
            ]
        );

        let mut bad = args.clone();
        bad[0] = path_expr("n");
        let err = generator_members(&holes(), Generator::Rect, &bad, Span::new(0, 0)).unwrap_err();
        assert!(err.message.starts_with("E006"));

        assert!(generator_members(&holes(), Generator::Rect, &args[..3], Span::new(0, 0)).is_err());

        let derived = derived_coordinate_members(&holes(), 2).unwrap();
        assert_eq!(derived.len(), 4);
        assert_eq!(derived[3].path.to_string(), "acme::Bracket.holes[1].y");
        assert!(
            derived
                .iter()
                .all(|m| m.analysis && m.provenance == Provenance::Derived)
        );
    }
}
