//! Datum-typed positions and vectors, and their arithmetic (geometry.md 3, 7).
//!
//! A `Point2 on top` is a point in `top`'s plane; a vector made by `Vec2(x, y)` is free until it
//! meets a point, and `Point - Point` is a vector fixed to the points' datum. Arithmetic across
//! datums, and array + point, is G001; nothing converts silently (the way named kinds never do,
//! Q003). Coordinates are exact [`Affine`] values (no f64), so tolerances propagate. Hand-listed
//! coordinates lower to [`Member`]s with canonical paths such as `Bracket.holes[1].x`.
// frob:ticket 01M4SWFX82NWXV79EQTA5QX081

use std::fmt;

use cadetta_sema::resolve::{CanonPath, Index, MemberSeg};
use cadetta_syntax::ast::{Expr, Type, TypeKind};
use cadetta_units::{Affine, ToleranceError};
use tracing::{debug, warn};

use crate::datum::{Axis, Member};

/// The diagnostic code for arithmetic or comparison across datums (geometry.md 7).
pub const G001: &str = "G001";

/// The name of a datum a position is expressed in, e.g. `top` or `origin`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DatumId(pub String);

impl DatumId {
    /// A datum id from its written path.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl fmt::Display for DatumId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Which point type a declared type names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PointKind {
    /// `Point2`.
    Point2,
    /// `Point3`.
    Point3,
}

impl PointKind {
    /// The number of coordinates.
    pub fn arity(self) -> usize {
        match self {
            PointKind::Point2 => 2,
            PointKind::Point3 => 3,
        }
    }
}

/// A point type `Point2 on D` or `Point3 on D`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PointType {
    /// Plane or space.
    pub kind: PointKind,
    /// The datum the type is on.
    pub datum: DatumId,
}

/// Reads `Point2 on D` / `Point3 on D` from a syntax type; `None` for any other type.
pub fn point_type(ty: &Type) -> Option<PointType> {
    let TypeKind::OnDatum { base, datum } = &ty.kind else {
        return None;
    };
    let kind = match base.text().as_str() {
        "Point2" => PointKind::Point2,
        "Point3" => PointKind::Point3,
        _ => return None,
    };
    Some(PointType {
        kind,
        datum: DatumId::new(datum.text()),
    })
}

/// What went wrong in position arithmetic.
#[derive(Clone, PartialEq, Debug)]
pub enum PositionError {
    /// G001: positions in different datums, or array + point.
    G001 {
        /// What was attempted.
        message: String,
    },
    /// The operands do not combine (rank, shape or operator not defined).
    Invalid {
        /// What is wrong.
        message: String,
    },
    /// A coordinate operation failed (dimension mismatch and so on).
    Coordinate(ToleranceError),
}

impl PositionError {
    /// The diagnostic code, if this error has one.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            PositionError::G001 { .. } => Some(G001),
            _ => None,
        }
    }
}

impl fmt::Display for PositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PositionError::G001 { message } => write!(f, "{G001}: {message}"),
            PositionError::Invalid { message } => f.write_str(message),
            PositionError::Coordinate(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PositionError {}

impl From<ToleranceError> for PositionError {
    fn from(e: ToleranceError) -> Self {
        PositionError::Coordinate(e)
    }
}

fn g001<T>(message: String) -> Result<T, PositionError> {
    warn!(code = G001, %message, "datum mismatch");
    Err(PositionError::G001 { message })
}

fn invalid<T>(message: String) -> Result<T, PositionError> {
    warn!(%message, "invalid position operation");
    Err(PositionError::Invalid { message })
}

/// A point: coordinates in one datum.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Point {
    /// The datum the coordinates are in.
    pub datum: DatumId,
    /// Two (`Point2`) or three (`Point3`) coordinates, in x, y, z order.
    pub coords: Vec<Affine>,
}

/// A vector. `datum` is `None` for a free vector written `Vec2(x, y)`; `Point - Point` fixes it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Vector {
    /// The datum it is fixed to, if any.
    pub datum: Option<DatumId>,
    /// Two (`Vec2`) or three (`Vec3`) components.
    pub comps: Vec<Affine>,
}

/// A position-domain value: a point, a vector, or an array of them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Pos {
    /// `Point2` / `Point3`.
    Point(Point),
    /// `Vec2` / `Vec3`.
    Vector(Vector),
    /// `[Point2 on D; N]` and the like.
    Array(Vec<Pos>),
}

impl Point {
    /// A literal point of declared type `ty` (`Point2(5mm, 0mm) on top`); arity must match.
    pub fn literal(ty: &PointType, coords: Vec<Affine>) -> Result<Self, PositionError> {
        if coords.len() != ty.kind.arity() {
            return invalid(format!(
                "{:?} takes {} coordinates, got {}",
                ty.kind,
                ty.kind.arity(),
                coords.len()
            ));
        }
        debug!(datum = %ty.datum, "typed point literal");
        Ok(Self {
            datum: ty.datum.clone(),
            coords,
        })
    }

    /// The declared type of this point.
    pub fn ty(&self) -> PointType {
        let kind = if self.coords.len() == 2 {
            PointKind::Point2
        } else {
            PointKind::Point3
        };
        PointType {
            kind,
            datum: self.datum.clone(),
        }
    }
}

impl Vector {
    /// A free vector `Vec2(x, y)` or `Vec3(x, y, z)`.
    pub fn free(comps: Vec<Affine>) -> Result<Self, PositionError> {
        if comps.len() != 2 && comps.len() != 3 {
            return invalid(format!(
                "a vector has 2 or 3 components, got {}",
                comps.len()
            ));
        }
        Ok(Self { datum: None, comps })
    }

    /// Scales by a dimensionless `s` (`Vec * s`); the datum is kept.
    pub fn scale(&self, s: &Affine) -> Result<Self, PositionError> {
        if !s.dim().dimension.is_dimensionless() || s.dim().angle != 0 {
            return invalid("a vector scales only by a dimensionless value".into());
        }
        let comps = self
            .comps
            .iter()
            .map(|c| c.mul(s))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            datum: self.datum.clone(),
            comps,
        })
    }

    /// Vector sum; a free vector adopts the other's datum, two datums must agree (G001).
    pub fn add(&self, o: &Self) -> Result<Self, PositionError> {
        let datum = match (&self.datum, &o.datum) {
            (Some(a), Some(b)) if a != b => {
                return g001(format!("vector in `{a}` plus vector in `{b}`; use `in`"));
            }
            (a, b) => a.clone().or_else(|| b.clone()),
        };
        Ok(Self {
            datum,
            comps: zip_with(&self.comps, &o.comps, Affine::add)?,
        })
    }
}

fn zip_with(
    a: &[Affine],
    b: &[Affine],
    f: fn(&Affine, &Affine) -> Result<Affine, ToleranceError>,
) -> Result<Vec<Affine>, PositionError> {
    if a.len() != b.len() {
        return invalid(format!(
            "{}-D operand with a {}-D operand",
            a.len(),
            b.len()
        ));
    }
    a.iter()
        .zip(b)
        .map(|(x, y)| f(x, y).map_err(PositionError::from))
        .collect()
}

fn check_vec_datum(p: &Point, v: &Vector) -> Result<(), PositionError> {
    match &v.datum {
        Some(d) if *d != p.datum => g001(format!(
            "point in `{}` with vector in `{d}`; use `in`",
            p.datum
        )),
        _ => Ok(()),
    }
}

/// `Point - Point`: a vector in the shared datum; different datums are G001.
pub fn point_sub(a: &Point, b: &Point) -> Result<Vector, PositionError> {
    if a.datum != b.datum {
        return g001(format!(
            "point in `{}` minus point in `{}`; use `in`",
            a.datum, b.datum
        ));
    }
    Ok(Vector {
        datum: Some(a.datum.clone()),
        comps: zip_with(&a.coords, &b.coords, Affine::sub)?,
    })
}

/// `Point + Vec`: a point in the same datum; a vector fixed to another datum is G001.
pub fn point_add_vec(p: &Point, v: &Vector) -> Result<Point, PositionError> {
    check_vec_datum(p, v)?;
    Ok(Point {
        datum: p.datum.clone(),
        coords: zip_with(&p.coords, &v.comps, Affine::add)?,
    })
}

/// `Vec * s` for a dimensionless `s`.
pub fn vec_scale(v: &Vector, s: &Affine) -> Result<Vector, PositionError> {
    v.scale(s)
}

/// `a + b` over position values: array + vector shifts every element, point + vector moves a
/// point, vector + vector adds; array + point and any cross-datum pair are G001.
pub fn add(a: &Pos, b: &Pos) -> Result<Pos, PositionError> {
    match (a, b) {
        (Pos::Point(p), Pos::Vector(v)) | (Pos::Vector(v), Pos::Point(p)) => {
            point_add_vec(p, v).map(Pos::Point)
        }
        (Pos::Vector(x), Pos::Vector(y)) => x.add(y).map(Pos::Vector),
        (Pos::Array(_), Pos::Point(_)) | (Pos::Point(_), Pos::Array(_)) => {
            g001("array plus a point; add a vector, or subtract a point to get one".into())
        }
        (Pos::Array(items), v @ Pos::Vector(_)) | (v @ Pos::Vector(_), Pos::Array(items)) => items
            .iter()
            .map(|e| add(e, v))
            .collect::<Result<Vec<_>, _>>()
            .map(Pos::Array),
        (Pos::Point(_), Pos::Point(_)) => {
            invalid("point plus point is undefined; subtract points to get a vector".into())
        }
        (Pos::Array(_), Pos::Array(_)) => invalid("array plus array is undefined".into()),
    }
}

/// `a - b` over position values: point - point is a vector, point - vector a point, vector -
/// vector a vector; cross-datum is G001.
pub fn sub(a: &Pos, b: &Pos) -> Result<Pos, PositionError> {
    match (a, b) {
        (Pos::Point(p), Pos::Point(q)) => point_sub(p, q).map(Pos::Vector),
        (Pos::Point(p), Pos::Vector(v)) => {
            let neg = v.scale(&minus_one())?;
            point_add_vec(p, &neg).map(Pos::Point)
        }
        (Pos::Vector(x), Pos::Vector(y)) => {
            let neg = y.scale(&minus_one())?;
            x.add(&neg).map(Pos::Vector)
        }
        (Pos::Array(_), Pos::Point(_)) | (Pos::Point(_), Pos::Array(_)) => {
            g001("array minus a point (or the reverse)".into())
        }
        (Pos::Array(items), v @ Pos::Vector(_)) => items
            .iter()
            .map(|e| sub(e, v))
            .collect::<Result<Vec<_>, _>>()
            .map(Pos::Array),
        _ => invalid("subtraction of these position operands is undefined".into()),
    }
}

fn minus_one() -> Affine {
    use cadetta_units::{Dim, Magnitude, Rational};
    Affine::constant(Dim::default(), Magnitude::new(Rational::from_int(-1), 0))
}

/// Comparison or `distance` precondition: both points must share a datum (G001 otherwise).
pub fn require_same_datum(a: &Point, b: &Point) -> Result<(), PositionError> {
    if a.datum == b.datum {
        Ok(())
    } else {
        g001(format!(
            "comparison of point in `{}` with point in `{}`; use `in`",
            a.datum, b.datum
        ))
    }
}

/// Lowers hand-listed points to coordinate members: point `i` of `array` yields
/// `array[i].x`, `.y` (and `.z`), each with its written expression (geometry.md 3).
pub fn coordinate_members(array: &CanonPath, points: &[Vec<Expr>]) -> Vec<Member> {
    let mut out = Vec::new();
    for (i, coords) in points.iter().enumerate() {
        for (expr, axis) in coords.iter().zip([Axis::X, Axis::Y, Axis::Z]) {
            let mut path = array.clone();
            path.members.push(MemberSeg::Index(Index::At(i as u64)));
            path.members.push(MemberSeg::Name(axis.name().to_owned()));
            debug!(%path, "coordinate member");
            out.push(Member {
                path,
                value: expr.clone(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::{ExprKind, FileEntry, ItemKind, MemberKind as AstMember};
    use cadetta_syntax::parser::parse;
    use cadetta_units::{Dim, Dimension, Magnitude, Rational};

    fn len() -> Dim {
        let mut si = [0; cadetta_units::SI_BASE_COUNT];
        si[1] = 1;
        Dim::new(Dimension::from_si(si), 0)
    }

    fn mm(n: i64) -> Affine {
        Affine::constant(len(), Magnitude::new(Rational::from_int(n), 0))
    }

    fn scalar(n: i64) -> Affine {
        Affine::constant(Dim::default(), Magnitude::new(Rational::from_int(n), 0))
    }

    fn pt(d: &str, x: i64, y: i64) -> Point {
        Point {
            datum: DatumId::new(d),
            coords: vec![mm(x), mm(y)],
        }
    }

    fn vec2(x: i64, y: i64) -> Vector {
        Vector::free(vec![mm(x), mm(y)]).unwrap()
    }

    // frob:tests crates/cadetta-geometry/src/point.rs::point_type kind=unit
    #[test]
    fn declared_and_literal_points_carry_the_datum() {
        let parsed = parse("model M {\n  hole: Point2 on top = Point2(5mm, 0mm)\n}\n");
        assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
        let mut found = None;
        for e in &parsed.file.entries {
            let FileEntry::Item(i) = e else { continue };
            let ItemKind::Component(c) = &i.kind else {
                continue;
            };
            for mem in &c.members {
                if let AstMember::Param(p) = &mem.kind {
                    found = p.ty.as_ref().and_then(point_type);
                }
            }
        }
        let ty = found.expect("Point2 on top");
        assert_eq!(ty.datum, DatumId::new("top"));
        let lit = Point::literal(&ty, vec![mm(5), mm(0)]).unwrap();
        assert_eq!(lit.datum, DatumId::new("top"));
        assert_eq!(lit.ty(), ty);
        assert!(Point::literal(&ty, vec![mm(5)]).is_err());
    }

    // frob:tests crates/cadetta-geometry/src/point.rs::point_sub kind=unit
    #[test]
    fn point_vector_arithmetic_stays_in_the_datum() {
        let a = pt("top", 7, 3);
        let b = pt("top", 2, 1);
        let d = point_sub(&a, &b).unwrap();
        assert_eq!(d.datum, Some(DatumId::new("top")));
        assert_eq!(d.comps, vec![mm(5), mm(2)]);
        let moved = point_add_vec(&b, &d).unwrap();
        assert_eq!(moved, a);
        let scaled = vec_scale(&d, &scalar(2)).unwrap();
        assert_eq!(scaled.datum, Some(DatumId::new("top")));
        assert_eq!(scaled.comps, vec![mm(10), mm(4)]);
        assert!(vec_scale(&d, &mm(2)).is_err());
        let arr = Pos::Array(vec![
            Pos::Point(pt("top", 0, 0)),
            Pos::Point(pt("top", 1, 1)),
        ]);
        let Pos::Array(shifted) = add(&arr, &Pos::Vector(vec2(5, 0))).unwrap() else {
            panic!("array expected");
        };
        assert_eq!(shifted[1], Pos::Point(pt("top", 6, 1)));
        let Pos::Vector(v) = sub(&Pos::Point(a), &Pos::Point(b)).unwrap() else {
            panic!("vector expected");
        };
        assert_eq!(v.comps, vec![mm(5), mm(2)]);
    }

    // frob:tests crates/cadetta-geometry/src/point.rs::add kind=unit
    #[test]
    fn cross_datum_and_array_plus_point_are_g001() {
        let top = pt("top", 1, 1);
        let other = pt("base", 1, 1);
        let e = point_sub(&top, &other).unwrap_err();
        assert_eq!(e.code(), Some("G001"));
        assert_eq!(
            require_same_datum(&top, &other).unwrap_err().code(),
            Some("G001")
        );
        let fixed = Vector {
            datum: Some(DatumId::new("base")),
            comps: vec![mm(1), mm(0)],
        };
        assert_eq!(
            point_add_vec(&top, &fixed).unwrap_err().code(),
            Some("G001")
        );
        let arr = Pos::Array(vec![Pos::Point(top.clone())]);
        let e = add(&arr, &Pos::Point(top)).unwrap_err();
        assert_eq!(e.code(), Some("G001"));
        assert!(e.to_string().starts_with("G001"));
    }

    // frob:tests crates/cadetta-geometry/src/point.rs::coordinate_members kind=unit
    #[test]
    fn hand_listed_coordinates_become_members() {
        let n = |s: &str| Expr {
            kind: ExprKind::Number(s.into()),
            span: cadetta_syntax::ast::Span { start: 0, end: 0 },
        };
        let base = CanonPath {
            package: "acme".into(),
            modules: vec![],
            item: "Bracket".into(),
            parts: vec![],
            members: vec![MemberSeg::Name("holes".into())],
        };
        let ms = coordinate_members(&base, &[vec![n("1"), n("2")], vec![n("3"), n("4")]]);
        let names: Vec<String> = ms.iter().map(|m| m.path.to_string()).collect();
        assert_eq!(
            names,
            [
                "acme::Bracket.holes[0].x",
                "acme::Bracket.holes[0].y",
                "acme::Bracket.holes[1].x",
                "acme::Bracket.holes[1].y",
            ]
        );
    }
}
