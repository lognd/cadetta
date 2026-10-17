//! Re-expression `p in D` and G003 (geometry.md 3, 4, 7).
//!
//! A [`FrameTable`] holds the frame of every datum a model knows, as operations from one root
//! frame: a part's own datums for `p in D` inside a part, or `origin` plus every datum of every
//! placed part for an assembly. `p in D` carries the point out of its datum into the root and
//! back into `D`, which is composition of the placements on the path and never a solve. A
//! conversion naming a part the model lacks or never placed is G003.
// frob:ticket 01M4SWG6N6DT76DRYYJHFB4656

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use tracing::{debug, trace, warn};

use crate::datum::{DatumDef, GeometryError, ORIGIN};
use crate::float::FloatMode;
use crate::placement::{
    Assembly, Eval, FrameOp, datum_frames, datum_ops, fold, invert, length_dim,
};
use crate::point::{DatumId, Point, PointKind, Pos};
use crate::transform::{Coord, TransformError};

/// The diagnostic code for a conversion out of an unplaced part (geometry.md 7).
pub const G003: &str = "G003";

/// What went wrong re-expressing a position.
#[derive(Clone, PartialEq, Debug)]
pub enum ReexpressError {
    /// G003: the model lacks the part or its placement.
    G003 {
        /// What was attempted.
        message: String,
    },
    /// The operand cannot be re-expressed (a vector, or an unknown datum of a placed part).
    Invalid {
        /// What is wrong.
        message: String,
    },
    /// Composing a frame failed.
    Geometry(GeometryError),
    /// Carrying the point failed.
    Transform(TransformError),
}

impl ReexpressError {
    /// The diagnostic code, if this error has one.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            ReexpressError::G003 { .. } => Some(G003),
            _ => None,
        }
    }
}

impl fmt::Display for ReexpressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReexpressError::G003 { message } => write!(f, "{G003}: {message}"),
            ReexpressError::Invalid { message } => f.write_str(message),
            ReexpressError::Geometry(e) => write!(f, "{e}"),
            ReexpressError::Transform(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ReexpressError {}

impl From<GeometryError> for ReexpressError {
    fn from(e: GeometryError) -> Self {
        ReexpressError::Geometry(e)
    }
}

impl From<TransformError> for ReexpressError {
    fn from(e: TransformError) -> Self {
        ReexpressError::Transform(e)
    }
}

/// A re-expressed point; coordinates stay [`Coord`]s because a rotation off 90deg is inexact.
#[derive(Clone, PartialEq, Debug)]
pub struct Located {
    /// The datum the coordinates are now in.
    pub datum: DatumId,
    /// Always three coordinates: `in` lifts a `Point2` to the `Point3` at z = 0 of its datum.
    pub coords: Vec<Coord>,
}

impl Located {
    /// The exact point, or `None` when any coordinate is inexact (Q006 marks those).
    pub fn to_point(&self) -> Option<Point> {
        let coords = self
            .coords
            .iter()
            .map(|c| match c {
                Coord::Exact(a) => Some(a.clone()),
                Coord::Approx(_) => None,
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Point {
            datum: self.datum.clone(),
            coords,
        })
    }
}

/// The result of `p in D` mapped over a position: a point, or an array of results.
#[derive(Clone, PartialEq, Debug)]
pub enum Reexpressed {
    /// A single re-expressed point.
    Point(Located),
    /// `in` mapped over an array, element by element.
    Array(Vec<Reexpressed>),
}

/// Whether a table is one part's own datums or an assembly's placed parts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Scope {
    Part,
    Assembly,
}

/// The frames of every datum a model knows, from one root frame.
pub struct FrameTable {
    scope: Scope,
    frames: BTreeMap<String, Vec<FrameOp>>,
    placed: BTreeSet<String>,
}

impl FrameTable {
    /// The table for `p in D` inside one part: its own datums, relative to its `origin`.
    pub fn for_part(defs: &[DatumDef], eval: Eval<'_>) -> Result<Self, ReexpressError> {
        let mut frames = BTreeMap::new();
        frames.insert(ORIGIN.to_owned(), Vec::new());
        for d in defs {
            let ops = datum_ops(defs, &d.name, eval, d.span)?;
            frames.insert(d.name.clone(), ops);
        }
        debug!(datums = frames.len(), "part frame table");
        Ok(Self {
            scope: Scope::Part,
            frames,
            placed: BTreeSet::new(),
        })
    }

    /// The table for an assembly: `origin` and `part.datum` for every placed part, composing
    /// the placements on the path (subassembly datums are `sub/part.datum`).
    pub fn for_assembly(a: &Assembly, eval: Eval<'_>) -> Result<Self, ReexpressError> {
        Self::for_assembly_reading(a, eval, FloatMode::With)
    }

    /// [`FrameTable::for_assembly`] under a float read mode: a joint's own `aligned` check reads
    /// its edge with [`FloatMode::Without`] (geometry.md 5.2).
    // frob:ticket 01M4Y2Z95B99FTVX68T4XTSTXR
    pub fn for_assembly_reading(
        a: &Assembly,
        eval: Eval<'_>,
        mode: FloatMode<'_>,
    ) -> Result<Self, ReexpressError> {
        let df = datum_frames(a, eval, mode)?;
        debug!(assembly = %a.name, placed = df.placed.len(), "assembly frame table");
        Ok(Self {
            scope: Scope::Assembly,
            frames: df.frames.into_iter().collect(),
            placed: df.placed,
        })
    }

    /// Looks up a datum's frame; a missing part or placement is G003 (geometry.md 3, 4).
    fn frame(&self, id: &DatumId) -> Result<&[FrameOp], ReexpressError> {
        if let Some(f) = self.frames.get(&id.0) {
            return Ok(f);
        }
        let owner = id.0.rsplit_once('.').map(|(part, _)| part);
        let known_part = match (self.scope, owner) {
            (Scope::Assembly, Some(part)) => self.placed.contains(part),
            (Scope::Part, None) => true,
            _ => false,
        };
        if known_part {
            warn!(datum = %id, "unknown datum");
            return Err(ReexpressError::Invalid {
                message: format!("no datum `{id}` (geometry.md 3)"),
            });
        }
        warn!(code = G003, datum = %id, "part or placement missing");
        Err(ReexpressError::G003 {
            message: format!(
                "`{id}` is not in this model: the part is absent or has no placement (geometry.md 3, 4)"
            ),
        })
    }

    /// `p in D`: re-expresses `p` in datum `to`, mapped over arrays. A `Point2` becomes the
    /// `Point3` at z = 0 of its datum before it is carried.
    pub fn reexpress(&self, p: &Pos, to: &DatumId) -> Result<Reexpressed, ReexpressError> {
        match p {
            Pos::Array(items) => items
                .iter()
                .map(|i| self.reexpress(i, to))
                .collect::<Result<Vec<_>, _>>()
                .map(Reexpressed::Array),
            Pos::Vector(_) => {
                warn!("`in` applied to a vector");
                Err(ReexpressError::Invalid {
                    message: "`in` re-expresses points, not vectors (geometry.md 3)".into(),
                })
            }
            Pos::Point(pt) => self.reexpress_point(pt, to).map(Reexpressed::Point),
        }
    }

    fn reexpress_point(&self, pt: &Point, to: &DatumId) -> Result<Located, ReexpressError> {
        let from_ops = self.frame(&pt.datum)?;
        let to_ops = self.frame(to)?;
        let dim = pt
            .coords
            .first()
            .map(|c| c.dim().clone())
            .unwrap_or_else(length_dim);
        let mut local: [Coord; 3] = std::array::from_fn(|i| match pt.coords.get(i) {
            Some(c) => Coord::Exact(c.clone()),
            None => Coord::constant(dim.clone(), 0),
        });
        if pt.ty().kind == PointKind::Point2 {
            local[2] = Coord::constant(dim, 0);
        }
        let in_root = fold(from_ops)?.apply(&local)?;
        let out = fold(&invert(to_ops))?.apply(&in_root)?;
        trace!(from = %pt.datum, to = %to, "point re-expressed");
        Ok(Located {
            datum: to.clone(),
            coords: out.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::tests::{build, eval, src};
    use cadetta_units::{Affine, Magnitude, Rational};

    fn mm(n: i64) -> Affine {
        Affine::constant(
            length_dim(),
            Magnitude::new(Rational::new(n, 1000).unwrap(), 0),
        )
    }

    fn pt(datum: &str, c: &[i64]) -> Pos {
        Pos::Point(Point {
            datum: DatumId::new(datum),
            coords: c.iter().map(|n| mm(*n)).collect(),
        })
    }

    fn coords(r: &Reexpressed) -> Vec<Coord> {
        match r {
            Reexpressed::Point(l) => l.coords.clone(),
            Reexpressed::Array(_) => panic!("array"),
        }
    }

    fn val(c: &Coord) -> Magnitude {
        match c {
            Coord::Exact(a) => a.center().clone(),
            Coord::Approx(_) => panic!("inexact"),
        }
    }

    fn m(n: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, 1000).unwrap(), 0)
    }

    fn mount() -> Assembly {
        build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  part bracket: Bracket\n  part loose: Bracket\n  place rail.origin at origin\n  place bracket.base at rail.site\n}\n",
            ),
            &[],
        )
    }

    // frob:tests crates/cadetta-geometry/src/reexpress.rs::FrameTable.reexpress kind=unit
    #[test]
    fn in_part_carries_through_own_datums() {
        let a = mount();
        let rail = &a.nodes[0].datums;
        let t = FrameTable::for_part(rail, &eval).unwrap();
        // `site` is x = 40mm, z = 30mm from the rail origin.
        let r = t
            .reexpress(&pt("site", &[1, 2, 3]), &DatumId::new("origin"))
            .unwrap();
        let c = coords(&r);
        assert_eq!(val(&c[0]), m(41));
        assert_eq!(val(&c[1]), m(2));
        assert_eq!(val(&c[2]), m(33));
        let back = t
            .reexpress(&pt("origin", &[41, 2, 33]), &DatumId::new("site"))
            .unwrap();
        assert_eq!(val(&coords(&back)[0]), m(1));
        assert_eq!(val(&coords(&back)[2]), m(3));
    }

    // frob:tests crates/cadetta-geometry/src/reexpress.rs::FrameTable.for_assembly kind=unit
    #[test]
    fn in_assembly_composes_placements() {
        let t = FrameTable::for_assembly(&mount(), &eval).unwrap();
        // bracket.base sits on rail.site (x 40, z 30); a point 1mm along the bracket's base x
        // is at x = 41 in the rail's origin and at x = 1 on rail.site.
        let r = t
            .reexpress(
                &pt("bracket.base", &[1, 0, 0]),
                &DatumId::new("rail.origin"),
            )
            .unwrap();
        let c = coords(&r);
        assert_eq!(val(&c[0]), m(41));
        assert_eq!(val(&c[2]), m(30));
        let r = t
            .reexpress(&pt("bracket.base", &[1, 0, 0]), &DatumId::new("rail.site"))
            .unwrap();
        assert_eq!(val(&coords(&r)[0]), m(1));
    }

    // frob:tests crates/cadetta-geometry/src/reexpress.rs::FrameTable.for_assembly_reading kind=unit
    #[test]
    fn reading_without_a_joint_drops_only_its_edge_float() {
        use crate::float::{EdgeFloat, JointFloat};
        use cadetta_units::VarId;
        let mut a = mount();
        a.floats = vec![EdgeFloat {
            node: "bracket".into(),
            joints: vec![JointFloat {
                path: "j".into(),
                float: mm(1),
                locating: false,
                onto: vec![[mm(0), mm(0)], [mm(10), mm(0)]],
                vars: [VarId(1), VarId(2), VarId(3)],
            }],
        }];
        let read = |mode| {
            let t = FrameTable::for_assembly_reading(&a, &eval, mode).unwrap();
            let r = t
                .reexpress(
                    &pt("bracket.base", &[0, 0, 0]),
                    &DatumId::new("rail.origin"),
                )
                .unwrap();
            coords(&r)[0].clone()
        };
        assert!(matches!(read(FloatMode::With), Coord::Approx(_)), "floats");
        assert_eq!(val(&read(FloatMode::Without("j"))), m(40), "own joint");
        assert!(matches!(read(FloatMode::Without("k")), Coord::Approx(_)));
    }

    // frob:tests crates/cadetta-geometry/src/reexpress.rs::FrameTable.reexpress kind=unit
    #[test]
    fn g003_on_unplaced_or_absent_part() {
        let t = FrameTable::for_assembly(&mount(), &eval).unwrap();
        let unplaced = t.reexpress(&pt("loose.base", &[0, 0, 0]), &DatumId::new("origin"));
        assert_eq!(unplaced.unwrap_err().code(), Some(G003));
        let absent = t.reexpress(&pt("rail.top", &[0, 0, 0]), &DatumId::new("motor.mount"));
        assert_eq!(absent.unwrap_err().code(), Some(G003));
        // A part-local datum converted out of its part is G003 too.
        let local = t.reexpress(&pt("top", &[0, 0, 0]), &DatumId::new("origin"));
        assert_eq!(local.unwrap_err().code(), Some(G003));
    }

    // frob:tests crates/cadetta-geometry/src/reexpress.rs::FrameTable.reexpress kind=unit
    #[test]
    fn g003_must_not_fire_on_placed_parts() {
        let t = FrameTable::for_assembly(&mount(), &eval).unwrap();
        assert!(
            t.reexpress(&pt("rail.top", &[0, 0, 0]), &DatumId::new("bracket.base"))
                .is_ok()
        );
        let bad = t.reexpress(&pt("rail.nope", &[0, 0, 0]), &DatumId::new("origin"));
        assert_eq!(bad.unwrap_err().code(), None);
    }

    // frob:tests crates/cadetta-geometry/src/reexpress.rs::FrameTable.reexpress kind=unit
    #[test]
    fn arrays_map_and_point2_lifts_to_z0() {
        let t = FrameTable::for_assembly(&mount(), &eval).unwrap();
        let arr = Pos::Array(vec![pt("rail.top", &[1, 2]), pt("rail.top", &[3, 4])]);
        let Reexpressed::Array(items) = t.reexpress(&arr, &DatumId::new("rail.origin")).unwrap()
        else {
            panic!("expected array")
        };
        assert_eq!(items.len(), 2);
        let c = coords(&items[1]);
        assert_eq!(c.len(), 3);
        assert_eq!(val(&c[0]), m(3));
        assert_eq!(val(&c[1]), m(4));
        // top is 30mm above the rail origin and the plane point is at z = 0 of top.
        assert_eq!(val(&c[2]), m(30));
        let same = t
            .reexpress(&pt("rail.top", &[1, 2]), &DatumId::new("rail.top"))
            .unwrap();
        let c = coords(&same);
        assert_eq!(c.len(), 3);
        assert_eq!(val(&c[2]), m(0));
    }
}
