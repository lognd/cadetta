//! IR geometry entities `Op`, `Datum`, `Point` and `Placement` (ir.md 3; geometry.md 2, 4, 6).
//!
//! `Datum` and `Placement` share one op chain (`Op`); the composed transform is derived from the
//! ops and never stored. Wire shapes: `Op` is `{ "offset": { x?, y?, z? } }`,
//! `{ "rotate": { axis, angle } }`, or `{ "flip": axis }`; a placement's point form `D.offset(p)` lowers to `offset`; offset and rotate
//! arguments are member paths (each argument is also a Member with provenance `authored`).
//! Variants are keyed `SLOT=ALT` like `Member.variants`; here a variant replaces the whole op
//! chain for that alternate.
// frob:ticket 01M4SWK96MJF28HVDDJNPCBMPP

use std::collections::BTreeMap;

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use tracing::debug;

use crate::model::{Digest, Exact, Value};
use crate::path::Path;

/// A coordinate axis; `flip` is restricted to `X` and `Y` by sema (geometry.md 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    /// The x axis.
    X,
    /// The y axis.
    Y,
    /// The z axis.
    Z,
}

impl Serialize for Axis {
    /// Serialize as `"x"`, `"y"` or `"z"`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        })
    }
}

/// One frame operation, shared by `Datum` and `Placement` (ir.md 3).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Op {
    /// Translate by the given axes (boxed: three paths are large).
    Offset(Box<OffsetArgs>),
    /// Rotate about an axis by the angle member.
    Rotate {
        /// The rotation axis.
        axis: Axis,
        /// Member path of the angle.
        angle: Path,
    },
    /// Flip about an axis (sugar for a 180 degree rotation; no member).
    Flip {
        /// The flip axis.
        axis: Axis,
    },
}

impl Serialize for Op {
    /// Serialize as a one-key object naming the op.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(1))?;
        match self {
            Op::Offset(o) => m.serialize_entry("offset", o)?,
            Op::Rotate { axis, angle } => {
                m.serialize_entry("rotate", &RotateArgs { axis, angle })?;
            }
            Op::Flip { axis } => m.serialize_entry("flip", axis)?,
        }
        m.end()
    }
}

/// Offset arguments: each present axis names its offset member.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct OffsetArgs {
    /// Member path of the x offset, if given.
    pub x: Option<Path>,
    /// Member path of the y offset, if given.
    pub y: Option<Path>,
    /// Member path of the z offset, if given.
    pub z: Option<Path>,
}

impl Serialize for OffsetArgs {
    /// Serialize the present axes only.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        for (k, v) in [("x", &self.x), ("y", &self.y), ("z", &self.z)] {
            if let Some(p) = v {
                m.serialize_entry(k, p)?;
            }
        }
        m.end()
    }
}

/// Borrowed wire form of the rotate arguments.
struct RotateArgs<'a> {
    axis: &'a Axis,
    angle: &'a Path,
}

impl Serialize for RotateArgs<'_> {
    /// Serialize as `{ axis, angle }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        m.serialize_entry("axis", self.axis)?;
        m.serialize_entry("angle", self.angle)?;
        m.end()
    }
}

/// A datum: a frame built from its parent by an op chain (geometry.md 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datum {
    /// The definition path (`Rail.site`).
    pub path: Path,
    /// The parent datum (or component origin) path.
    pub parent: Path,
    /// The op chain, applied left to right.
    pub ops: Vec<Op>,
    /// Per-alternate op chains keyed `SLOT=ALT`.
    pub variants: Option<BTreeMap<String, Vec<Op>>>,
    /// Content digest.
    pub digest: Option<Digest>,
    /// Closure digest.
    pub closure: Option<Digest>,
}

impl Datum {
    /// A datum with no variants or digests yet.
    pub fn new(path: Path, parent: Path, ops: Vec<Op>) -> Self {
        debug!(path = %path, ops = ops.len(), "new datum");
        Datum {
            path,
            parent,
            ops,
            variants: None,
            digest: None,
            closure: None,
        }
    }
}

impl Serialize for Datum {
    /// Serialize as `{ path, parent, ops, variants?, digest?, closure? }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("parent", &self.parent)?;
        m.serialize_entry("ops", &self.ops)?;
        if let Some(v) = &self.variants {
            m.serialize_entry("variants", v)?;
        }
        if let Some(d) = &self.digest {
            m.serialize_entry("digest", d)?;
        }
        if let Some(c) = &self.closure {
            m.serialize_entry("closure", c)?;
        }
        m.end()
    }
}

/// A named point on a datum; each coordinate is also a Member (ir.md 3).
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    /// The point path.
    pub path: Path,
    /// The datum it is expressed in.
    pub datum: Path,
    /// The coordinate values.
    pub coords: Vec<Value>,
    /// The position-tolerance zone diameter, when the point is `[position d]`-toleranced.
    // frob:ticket 01M50C0NEZM1R9KYFTRJCJY1Z8
    pub position: Option<Exact>,
}

impl Serialize for Point {
    /// Serialize as `{ path, datum, coords }` plus `position` when toleranced.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(3 + usize::from(self.position.is_some())))?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("datum", &self.datum)?;
        m.serialize_entry("coords", &self.coords)?;
        if let Some(p) = &self.position {
            m.serialize_entry("position", p)?;
        }
        m.end()
    }
}

/// A placement edge of the part tree: `place part.datum at target` (geometry.md 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The placement's own path, `Assembly/part.place` (`Assembly/feet[2].place` per instance).
    pub path: Path,
    /// The placed part.
    pub part: Path,
    /// The part's datum that is brought onto the target.
    pub datum: Path,
    /// The datum the part's datum is brought to.
    pub at: Path,
    /// The op chain applied after the frames coincide.
    pub ops: Vec<Op>,
    /// Per-alternate op chains keyed `SLOT=ALT`.
    pub variants: Option<BTreeMap<String, Vec<Op>>>,
    /// Content digest.
    pub digest: Option<Digest>,
    /// Closure digest.
    pub closure: Option<Digest>,
}

impl Serialize for Placement {
    /// Serialize as `{ path, part, datum, at, ops, variants?, digest?, closure? }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("part", &self.part)?;
        m.serialize_entry("datum", &self.datum)?;
        m.serialize_entry("at", &self.at)?;
        m.serialize_entry("ops", &self.ops)?;
        if let Some(v) = &self.variants {
            m.serialize_entry("variants", v)?;
        }
        if let Some(d) = &self.digest {
            m.serialize_entry("digest", d)?;
        }
        if let Some(c) = &self.closure {
            m.serialize_entry("closure", c)?;
        }
        m.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathContext;

    fn p(t: &str) -> Path {
        Path::parse(t, PathContext::General).unwrap()
    }

    #[test]
    fn datum_and_placement_share_op_form() {
        let ops = vec![
            Op::Offset(Box::new(OffsetArgs {
                x: Some(p("acme.r::Rail.site.offset.x")),
                ..Default::default()
            })),
            Op::Rotate {
                axis: Axis::Z,
                angle: p("acme.r::Rail.site.rotate.z"),
            },
            Op::Flip { axis: Axis::X },
        ];
        let d = Datum::new(
            p("acme.r::Rail.site"),
            p("acme.r::Rail.origin"),
            ops.clone(),
        );
        let pl = Placement {
            path: p("acme.r::Asm/bracket.place"),
            part: p("acme.r::Asm/bracket.x"),
            datum: p("acme.r::Bracket.base"),
            at: p("acme.r::Rail.site"),
            ops,
            variants: None,
            digest: None,
            closure: None,
        };
        let dj = serde_json::to_value(&d).unwrap();
        let pj = serde_json::to_value(&pl).unwrap();
        assert_eq!(dj["ops"], pj["ops"]);
        assert_eq!(dj["ops"][0]["offset"]["x"], "acme.r::Rail.site.offset.x");
        assert!(dj["ops"][0]["offset"].get("y").is_none());
        assert_eq!(dj["ops"][1]["rotate"]["axis"], "z");
        assert_eq!(dj["ops"][2]["flip"], "x");
        assert!(dj.get("variants").is_none());
    }

    #[test]
    fn placement_variants_keyed_like_members() {
        let mut v = BTreeMap::new();
        v.insert("a/w=b::C".to_string(), vec![Op::Flip { axis: Axis::Y }]);
        let pl = Placement {
            path: p("acme.r::Asm/b.place"),
            part: p("acme.r::Asm/b.x"),
            datum: p("acme.r::B.base"),
            at: p("acme.r::R.site"),
            ops: vec![],
            variants: Some(v),
            digest: None,
            closure: None,
        };
        let j = serde_json::to_value(&pl).unwrap();
        assert_eq!(j["variants"]["a/w=b::C"][0]["flip"], "y");
        assert_eq!(j["at"], "acme.r::R.site");
        assert_eq!(j["path"], "acme.r::Asm/b.place");
    }
}
