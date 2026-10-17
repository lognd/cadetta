//! The ir.md 3 entities `Interface`, `Table`, `Waiver`, `Process` and `Require`, with their JSON encoding.
//!
//! `Part`, `Fit`, `Solver` and `Check` already live in [`crate::model`] (with their encoding in
//! the json module); this module adds the remaining item-level entities so every design entity
//! is visible in the IR. Item-level paths (`acme.rover::Caster`) have no member tail, so they use
//! [`ComponentPath`] rather than [`crate::path::Path`].
//!
//! Decisions made here (written into ir.md 3): `Interface.members[]` are `{ name, type,
//! optional }`, `bounds[]` are `{ name, bound }` and `datums[]` are datum names; `Table.key` and
//! `cols[]` are `{ name, type }` and a row is `{ key, cells[] }` where a cell is a `Value` or an
//! enum variant name (`{ "variant": "M5" }`); `Waiver.targets_closure` maps each matched target
//! path text to its closure digest (empty until reviewed, lints.md 2.1); `Waiver.expires` is the
//! ISO date text; `Waiver.for_alt` is a full-form [`VariantName`] and serializes as
//! `SLOT=ALT`; `Require` is `{ path, expr, message?, citation?, verdicts, conservative }` (a
//! `Check` without `waived_for`, since a require is never suppressed; `citation` is the
//! `CiteRef` form); `Process.params[]` are `Member`s and `rules[]` are `{ role, expr }`.
// frob:ticket 01M4SWK99ZRAYB4EQ355S4E2HA

use std::collections::BTreeMap;

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use tracing::debug;

// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
use crate::model::{Bound, Check, CiteRef, Digest, Member, Value, Verdicts};
use crate::path::Path;
use crate::variant_name::{ComponentPath, VariantName};

/// An interface member requirement `name[?]: type` (language.md 3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceMember {
    /// The member name.
    pub name: String,
    /// The required kind, type text.
    pub type_name: String,
    /// True when written `name?: type`: an implementer may omit it.
    pub optional: bool,
}

/// An interface bound requirement `name <= x` (language.md 3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceBound {
    /// The bounded member name.
    pub name: String,
    /// The required bound.
    pub bound: Bound,
}

/// What a design needs from a part (ir.md 3 `Interface`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    /// Item path.
    pub path: ComponentPath,
    /// Required members.
    pub members: Vec<InterfaceMember>,
    /// Required bounds.
    pub bounds: Vec<InterfaceBound>,
    /// Required datum names.
    pub datums: Vec<String>,
}

/// A named, typed table column (ir.md 3 `Table`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCol {
    /// The column name.
    pub name: String,
    /// The column type text.
    pub type_name: String,
}

/// One table cell: a quantity or an enum variant.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// A quantity.
    Value(Box<Value>),
    /// An enum variant name.
    Variant(String),
}

/// One table row: its key and its column cells (empty for a key-only row).
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    /// The row key.
    pub key: Cell,
    /// One cell per column, in column order.
    pub cells: Vec<Cell>,
}

/// A lookup table (ir.md 3 `Table`).
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// Item path.
    pub path: ComponentPath,
    /// The key column.
    pub key: TableCol,
    /// The value columns.
    pub cols: Vec<TableCol>,
    /// The rows.
    pub rows: Vec<TableRow>,
    /// The table's citation.
    pub citation: Option<CiteRef>,
}

/// A suppression of one code on a path glob (ir.md 3 `Waiver`; lints.md 2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiver {
    /// The suppressed diagnostic code.
    pub code: String,
    /// The path glob it applies to.
    pub glob: String,
    /// The required reason.
    pub because: String,
    /// Expiry date, ISO text.
    pub expires: Option<String>,
    /// Scope to one alternate's variant in one slot.
    pub for_alt: Option<VariantName>,
    /// Closure digest of each matched target, keyed by target path text.
    pub targets_closure: BTreeMap<String, Digest>,
}

/// A `require` obligation (ir.md 3 `Require`; language.md 3.11). Unlike a `Check` it has no
/// `waived_for`: a require is never suppressed (obligations.md 4).
// frob:ticket 01M4YKTX2YMSXVE4CK0JAVMC41
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Require {
    /// Canonical path (`Rover.mass_budget`; instance path for a callee's require).
    pub path: Path,
    /// Normalized expression text.
    pub expr: String,
    /// The optional message string after the expression.
    pub message: Option<String>,
    /// The optional trailing citation (`@customer:"REQ-12"`).
    pub citation: Option<CiteRef>,
    /// Primary and per-alternate verdicts.
    pub verdicts: Verdicts,
    /// True when a verdict rests on a conservative enclosure.
    pub conservative: bool,
}

impl Require {
    /// A new undecided require (no message, citation or verdicts).
    pub fn new(path: Path, expr: impl Into<String>) -> Self {
        let r = Self {
            path,
            expr: expr.into(),
            message: None,
            citation: None,
            verdicts: Verdicts::default(),
            conservative: false,
        };
        debug!(path = %r.path, "ir: require built");
        r
    }

    /// The require as a [`Check`] (no waivers), so graph and digest code treats both alike.
    // frob:ticket 01M4YR871RH1TPH8854X50QNG4
    pub fn as_check(&self) -> Check {
        Check {
            path: self.path.clone(),
            expr: self.expr.clone(),
            verdicts: self.verdicts.clone(),
            conservative: self.conservative,
            waived_for: Vec::new(),
        }
    }
}

/// One compensation rule of a process, by role (ir.md 3 `Process`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRule {
    /// The role name a member opts into.
    pub role: String,
    /// The rule body, normalized expression text.
    pub expr: String,
}

/// A manufacturing process (ir.md 3 `Process`).
#[derive(Debug, Clone, PartialEq)]
pub struct Process {
    /// Item path.
    pub path: ComponentPath,
    /// The process parameters.
    pub params: Vec<Member>,
    /// The rules, by role.
    pub rules: Vec<ProcessRule>,
}

impl Waiver {
    /// A new open waiver (no expiry, no alternate scope, nothing reviewed).
    pub fn new(
        code: impl Into<String>,
        glob: impl Into<String>,
        because: impl Into<String>,
    ) -> Self {
        let w = Self {
            code: code.into(),
            glob: glob.into(),
            because: because.into(),
            expires: None,
            for_alt: None,
            targets_closure: BTreeMap::new(),
        };
        debug!(code = %w.code, glob = %w.glob, "ir: waiver built");
        w
    }
}

impl Serialize for ComponentPath {
    /// Serialize as the canonical item path text.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl Serialize for VariantName {
    /// Serialize as the full-form `SLOT=ALT` key text.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

/// Serialize one optional field only when present.
fn opt<M: SerializeMap, T: Serialize>(m: &mut M, key: &str, v: &Option<T>) -> Result<(), M::Error> {
    match v {
        Some(x) => m.serialize_entry(key, x),
        None => Ok(()),
    }
}

impl Serialize for InterfaceMember {
    /// Serialize as `{ name, type, optional }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("name", &self.name)?;
        m.serialize_entry("type", &self.type_name)?;
        m.serialize_entry("optional", &self.optional)?;
        m.end()
    }
}

impl Serialize for InterfaceBound {
    /// Serialize as `{ name, bound }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("name", &self.name)?;
        m.serialize_entry("bound", &self.bound)?;
        m.end()
    }
}

impl Serialize for Interface {
    /// Serialize as `{ path, members, bounds, datums }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("members", &self.members)?;
        m.serialize_entry("bounds", &self.bounds)?;
        m.serialize_entry("datums", &self.datums)?;
        m.end()
    }
}

impl Serialize for TableCol {
    /// Serialize as `{ name, type }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("name", &self.name)?;
        m.serialize_entry("type", &self.type_name)?;
        m.end()
    }
}

impl Serialize for Cell {
    /// Serialize a quantity as its `Value`, a variant as `{ "variant": name }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Cell::Value(v) => v.serialize(s),
            Cell::Variant(n) => {
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry("variant", n)?;
                m.end()
            }
        }
    }
}

impl Serialize for TableRow {
    /// Serialize as `{ key, cells }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        m.serialize_entry("key", &self.key)?;
        m.serialize_entry("cells", &self.cells)?;
        m.end()
    }
}

impl Serialize for Table {
    /// Serialize as `{ path, key, cols, rows, citation? }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("key", &self.key)?;
        m.serialize_entry("cols", &self.cols)?;
        m.serialize_entry("rows", &self.rows)?;
        opt(&mut m, "citation", &self.citation)?;
        m.end()
    }
}

impl Serialize for Waiver {
    /// Serialize as `{ code, glob, because, expires?, for_alt?, targets_closure }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("code", &self.code)?;
        m.serialize_entry("glob", &self.glob)?;
        m.serialize_entry("because", &self.because)?;
        opt(&mut m, "expires", &self.expires)?;
        opt(&mut m, "for_alt", &self.for_alt)?;
        m.serialize_entry("targets_closure", &self.targets_closure)?;
        m.end()
    }
}

impl Serialize for Require {
    /// Serialize as `{ path, expr, message?, citation?, verdicts, conservative }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("expr", &self.expr)?;
        opt(&mut m, "message", &self.message)?;
        opt(&mut m, "citation", &self.citation)?;
        m.serialize_entry("verdicts", &self.verdicts)?;
        m.serialize_entry("conservative", &self.conservative)?;
        m.end()
    }
}

impl Serialize for ProcessRule {
    /// Serialize as `{ role, expr }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        m.serialize_entry("role", &self.role)?;
        m.serialize_entry("expr", &self.expr)?;
        m.end()
    }
}

impl Serialize for Process {
    /// Serialize as `{ path, params, rules }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("params", &self.params)?;
        m.serialize_entry("rules", &self.rules)?;
        m.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BoundOp, Check, Exact, MemberKind, Nominal, VerdictState, Verdicts};
    use crate::path::{Path, PathContext};
    use cadetta_units::{Dim, Magnitude, Rational};

    fn cp(s: &str) -> ComponentPath {
        ComponentPath::parse(s).unwrap()
    }

    fn ex(n: i64, d: i64) -> Exact {
        Exact(Magnitude::new(Rational::new(n, d).unwrap(), 0))
    }

    fn val(n: i64, d: i64, text: &str) -> Value {
        Value::new(Nominal::Exact(ex(n, d)), Dim::default(), text)
    }

    // frob:tests interface_table_process_shape
    #[test]
    fn interface_table_process_shape() {
        let i = Interface {
            path: cp("acme.rover::Caster"),
            members: vec![InterfaceMember {
                name: "bore".into(),
                type_name: "Length".into(),
                optional: true,
            }],
            bounds: vec![InterfaceBound {
                name: "load".into(),
                bound: Bound {
                    op: BoundOp::Ge,
                    lo: Some(ex(5, 1)),
                    hi: None,
                },
            }],
            datums: vec!["mount".into()],
        };
        let j = serde_json::to_value(&i).unwrap();
        assert_eq!(j["path"], "acme.rover::Caster");
        assert_eq!(j["members"][0]["type"], "Length");
        assert_eq!(j["members"][0]["optional"], true);
        assert_eq!(j["bounds"][0]["bound"]["op"], ">=");
        assert_eq!(j["datums"][0], "mount");

        let t = Table {
            path: cp("acme.rover::Iso273"),
            key: TableCol {
                name: "thread".into(),
                type_name: "MetricThread".into(),
            },
            cols: vec![TableCol {
                name: "fine".into(),
                type_name: "Length".into(),
            }],
            rows: vec![TableRow {
                key: Cell::Variant("M5".into()),
                cells: vec![Cell::Value(Box::new(val(53, 10000, "5.3mm")))],
            }],
            citation: None,
        };
        let j = serde_json::to_value(&t).unwrap();
        assert_eq!(j["key"]["name"], "thread");
        assert_eq!(j["rows"][0]["key"]["variant"], "M5");
        assert_eq!(j["rows"][0]["cells"][0]["nominal"]["r"], "53/10000");
        assert!(j.get("citation").is_none());

        let pm = Member::new(
            Path::parse("acme.rover::Fdm.hole_comp", PathContext::General).unwrap(),
            MemberKind::Param,
        );
        let p = Process {
            path: cp("acme.rover::Fdm"),
            params: vec![pm],
            rules: vec![ProcessRule {
                role: "hole".into(),
                expr: "d + hole_comp".into(),
            }],
        };
        let j = serde_json::to_value(&p).unwrap();
        assert_eq!(j["params"][0]["path"], "acme.rover::Fdm.hole_comp");
        assert_eq!(j["rules"][0]["role"], "hole");
        assert_eq!(j["rules"][0]["expr"], "d + hole_comp");
    }

    // frob:tests waiver_for_alt_keyed_slot_alt
    #[test]
    fn waiver_for_alt_keyed_slot_alt() {
        let mut w = Waiver::new("D001", "acme.parts::Bearing608.*", "vendor dims coincide");
        w.for_alt =
            Some(VariantName::parse("acme.rover::Rover/wheel=acme.rover::Colson2Series").unwrap());
        w.expires = Some("2027-01-01".into());
        w.targets_closure
            .insert("acme.parts::Bearing608.bore".into(), Digest("ab".into()));
        let j = serde_json::to_value(&w).unwrap();
        assert_eq!(
            j["for_alt"],
            "acme.rover::Rover/wheel=acme.rover::Colson2Series"
        );
        assert_eq!(j["expires"], "2027-01-01");
        assert_eq!(j["targets_closure"]["acme.parts::Bearing608.bore"], "ab");
        let open = serde_json::to_value(Waiver::new("D001", "a::B.*", "x")).unwrap();
        assert!(open.get("for_alt").is_none());
        assert!(open.get("expires").is_none());
        assert_eq!(open["targets_closure"], serde_json::json!({}));
    }

    // frob:tests check_verdicts_keyed_slot_alt
    #[test]
    fn check_verdicts_keyed_slot_alt() {
        let mut verdicts = Verdicts {
            primary: Some(VerdictState::Pass),
            ..Verdicts::default()
        };
        verdicts
            .per_alternate
            .insert("Rover/wheel=Colson2Series".into(), VerdictState::Fail);
        let c = Check {
            path: Path::parse("acme.rover::Rover.fits", PathContext::General).unwrap(),
            expr: "a < b".into(),
            verdicts,
            conservative: true,
            waived_for: vec!["Rover/wheel=Colson2Series".into()],
        };
        let j = serde_json::to_value(&c).unwrap();
        assert_eq!(j["verdicts"]["primary"], "pass");
        assert_eq!(
            j["verdicts"]["per_alternate"]["Rover/wheel=Colson2Series"],
            "fail"
        );
        assert_eq!(j["conservative"], true);
        assert_eq!(j["waived_for"][0], "Rover/wheel=Colson2Series");
    }
}
