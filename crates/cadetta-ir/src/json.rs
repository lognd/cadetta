//! JSON encoding of the `cadetta.ir/1` model (ir.md 3): serde `Serialize` for every IR type.
//!
//! The one rule: magnitudes are never JSON floats. `Exact` is `{ "r": "num/den", "pi": int }`
//! with a reduced fraction and `den > 0` (an integer is `"n/1"`, so the shape never varies);
//! `Inexact` is `{ "f64": float, "root": path }` and is the only place a float appears. The
//! impls are serializer-agnostic; `serde_json::to_string` gives the wire text. Absent optional
//! fields are omitted, maps and lists keep model order (maps are `BTreeMap`, so sorted).
//!
//! Decisions made here (written into ir.md 3): `Provenance` is the string `"authored"` or
//! `"derived"`, or `{ "imported": { adapter, file, native_name } }`; enums are lowercase
//! strings (`BoundOp` is `"<="`, `">="`, `"in"`; `Sign` is the integer `1` or `-1`); renamed
//! Rust fields are `priv` (`private`), `type` (`type_name`, `fit_type`) and `for`
//! (`for_alternate`); a non-finite `Inexact.f64` is a serialization error, as JSON cannot hold it.
// frob:ticket 01M4SPWQ6VDD0177RREW273PYG

use serde::ser::{Error as _, SerializeMap, SerializeStruct};
use serde::{Serialize, Serializer};
use tracing::{debug, warn};

use crate::dim::DimWire;
use crate::model::{
    // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
    Affine,
    AffineTerm,
    Binding,
    Bound,
    BoundOp,
    Check,
    Citation,
    CitationType,
    CiteBuiltin,
    CiteField,
    CiteRef,
    Component,
    ComponentClass,
    Dependency,
    Digest,
    Exact,
    Fit,
    Flavour,
    Implicit,
    Inexact,
    Locator,
    LocatorKind,
    LocatorMode,
    Member,
    MemberKind,
    Nominal,
    ObservedFrame,
    ObservedInstance,
    PackageId,
    Part,
    Project,
    Provenance,
    Sign,
    Solver,
    SpanRef,
    TolerancePolicy,
    Value,
    VerdictState,
    Verdicts,
};
use crate::path::Path;

/// The reduced `"num/den"` text of an exact magnitude's rational (den > 0, always present).
pub fn rational_text(x: &Exact) -> String {
    let r = x.0.rational();
    format!("{}/{}", r.numer(), r.denom())
}

/// Serialize one optional field only when present.
fn opt<M: SerializeMap, T: Serialize>(m: &mut M, key: &str, v: &Option<T>) -> Result<(), M::Error> {
    match v {
        Some(x) => m.serialize_entry(key, x),
        None => Ok(()),
    }
}

/// Map a unit-like enum to its lowercase wire name.
macro_rules! str_enum {
    ($t:ty { $($v:path => $s:literal),+ $(,)? }) => {
        impl Serialize for $t {
            /// Serialize as its wire string.
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(match self { $($v => $s),+ })
            }
        }
    };
}

str_enum!(Flavour { Flavour::Design => "design", Flavour::Observed => "observed" });
str_enum!(BoundOp { BoundOp::Le => "<=", BoundOp::Ge => ">=", BoundOp::In => "in" });
str_enum!(ComponentClass {
    ComponentClass::Ots => "ots", ComponentClass::Model => "model",
    ComponentClass::Assembly => "assembly", ComponentClass::Process => "process",
});
str_enum!(MemberKind {
    MemberKind::Param => "param", MemberKind::Let => "let", MemberKind::Bound => "bound",
    MemberKind::Datum => "datum", MemberKind::Place => "place",
});
str_enum!(CitationType {
    CitationType::Standard => "standard", CitationType::Datasheet => "datasheet",
    CitationType::Drawing => "drawing", CitationType::Catalog => "catalog",
    CitationType::Book => "book", CitationType::Article => "article",
    CitationType::Web => "web", CitationType::Quote => "quote",
    CitationType::Document => "document",
});
str_enum!(CiteBuiltin {
    CiteBuiltin::Note => "note", CiteBuiltin::Measured => "measured",
    CiteBuiltin::Assumed => "assumed", CiteBuiltin::Estimate => "estimate",
});
str_enum!(VerdictState {
    VerdictState::Pass => "pass", VerdictState::Fail => "fail", VerdictState::Unknown => "unknown",
});
str_enum!(LocatorKind {
    LocatorKind::Global => "global", LocatorKind::Dimension => "dimension",
    LocatorKind::Property => "property", LocatorKind::Frame => "frame",
    LocatorKind::Instance => "instance",
});
str_enum!(Implicit {
    Implicit::Global => "global", Implicit::Link => "link",
    Implicit::Frame => "frame", Implicit::Reference => "reference",
});
str_enum!(TolerancePolicy {
    TolerancePolicy::Drive => "drive", TolerancePolicy::Ignore => "ignore",
});
str_enum!(LocatorMode { LocatorMode::Value => "value", LocatorMode::Ref => "ref" });

impl Serialize for Sign {
    /// Serialize as the integer `1` or `-1`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_i8(match self {
            Sign::Plus => 1,
            Sign::Minus => -1,
        })
    }
}

impl Serialize for Path {
    /// Serialize as the canonical path text.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl Serialize for Digest {
    /// Serialize as the lowercase hex string.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl Serialize for Exact {
    /// Serialize as `{ "r": "num/den", "pi": int }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Exact", 2)?;
        st.serialize_field("r", &rational_text(self))?;
        st.serialize_field("pi", &self.0.pi_exp())?;
        st.end()
    }
}

impl Serialize for Inexact {
    /// Serialize as `{ "f64": float, "root": path }`; the only float in the IR.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if !self.f64.is_finite() {
            warn!(root = %self.root, "ir json: non-finite inexact value");
            return Err(S::Error::custom("Inexact.f64 must be finite"));
        }
        debug!(root = %self.root, "ir json: inexact value");
        let mut st = s.serialize_struct("Inexact", 2)?;
        st.serialize_field("f64", &self.f64)?;
        st.serialize_field("root", &self.root)?;
        st.end()
    }
}

impl Serialize for Nominal {
    /// Serialize the `Exact` or `Inexact` form directly (the shapes are told apart by keys).
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Nominal::Exact(e) => e.serialize(s),
            Nominal::Inexact(i) => i.serialize(s),
        }
    }
}

impl Serialize for SpanRef {
    /// Serialize as `{ file, start, end }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("SpanRef", 3)?;
        st.serialize_field("file", &self.file)?;
        st.serialize_field("start", &self.start)?;
        st.serialize_field("end", &self.end)?;
        st.end()
    }
}

impl Serialize for AffineTerm {
    /// Serialize as `{ var, lo, hi }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("AffineTerm", 3)?;
        st.serialize_field("var", &self.var)?;
        st.serialize_field("lo", &self.lo)?;
        st.serialize_field("hi", &self.hi)?;
        st.end()
    }
}

impl Serialize for Affine {
    /// Serialize as `{ terms: [...] }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Affine", 1)?;
        st.serialize_field("terms", &self.terms)?;
        st.end()
    }
}

impl Serialize for Bound {
    /// Serialize as `{ op, lo?, hi? }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("op", &self.op)?;
        opt(&mut m, "lo", &self.lo)?;
        opt(&mut m, "hi", &self.hi)?;
        m.end()
    }
}

impl Serialize for Value {
    /// Serialize per ir.md 3 `Value`; `dim` is the wire dimension vector.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("nominal", &self.nominal)?;
        m.serialize_entry("dim", &DimWire::from(&self.dim))?;
        m.serialize_entry("angle", &self.angle)?;
        opt(&mut m, "tol", &self.tol)?;
        m.serialize_entry("conservative", &self.conservative)?;
        opt(&mut m, "drawn", &self.drawn)?;
        m.serialize_entry("exact", &self.exact)?;
        opt(&mut m, "reason", &self.reason)?;
        opt(&mut m, "unit_assumed", &self.unit_assumed)?;
        m.serialize_entry("authored", &self.authored)?;
        m.end()
    }
}

impl Serialize for Provenance {
    /// Serialize as `"authored"`, `"derived"` or `{ "imported": { adapter, file, native_name } }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Provenance::Authored => s.serialize_str("authored"),
            Provenance::Derived => s.serialize_str("derived"),
            Provenance::Imported {
                adapter,
                file,
                native_name,
            } => {
                #[derive(Serialize)]
                struct Imp<'a> {
                    adapter: &'a str,
                    file: &'a str,
                    native_name: &'a str,
                }
                let mut m = s.serialize_map(Some(1))?;
                m.serialize_entry(
                    "imported",
                    &Imp {
                        adapter,
                        file,
                        native_name,
                    },
                )?;
                m.end()
            }
        }
    }
}

impl Serialize for CiteField {
    /// Serialize as a string, or a list of strings for `author`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            CiteField::Text(t) => s.serialize_str(t),
            CiteField::List(l) => l.serialize(s),
        }
    }
}

impl Serialize for Citation {
    /// Serialize as `{ path, type, fields, file_digest?, digest }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("type", &self.ty)?;
        m.serialize_entry("fields", &self.fields)?;
        opt(&mut m, "file_digest", &self.file_digest)?;
        m.serialize_entry("digest", &self.digest)?;
        m.end()
    }
}

impl Serialize for CiteRef {
    /// Serialize as `{ cite?, builtin?, text?, locator? }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        opt(&mut m, "cite", &self.cite)?;
        opt(&mut m, "builtin", &self.builtin)?;
        opt(&mut m, "text", &self.text)?;
        opt(&mut m, "locator", &self.locator)?;
        m.end()
    }
}

impl Serialize for Member {
    /// Serialize per ir.md 3 `Member`; `private` is written as `priv`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("kind", &self.kind)?;
        m.serialize_entry("analysis", &self.analysis)?;
        m.serialize_entry("priv", &self.private)?;
        opt(&mut m, "value", &self.value)?;
        opt(&mut m, "bound", &self.bound)?;
        opt(&mut m, "expr", &self.expr)?;
        m.serialize_entry("deps", &self.deps)?;
        opt(&mut m, "role", &self.role)?;
        opt(&mut m, "citation", &self.citation)?;
        m.serialize_entry("was", &self.was)?;
        m.serialize_entry("attrs", &self.attrs)?;
        m.serialize_entry("provenance", &self.provenance)?;
        opt(&mut m, "doc", &self.doc)?;
        opt(&mut m, "span", &self.span)?;
        opt(&mut m, "variants", &self.variants)?;
        opt(&mut m, "digest", &self.digest)?;
        opt(&mut m, "closure", &self.closure)?;
        m.end()
    }
}

impl Serialize for Part {
    /// Serialize as `{ path, type, count?, primary, alternates, args }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("type", &self.type_name)?;
        opt(&mut m, "count", &self.count)?;
        m.serialize_entry("primary", &self.primary)?;
        m.serialize_entry("alternates", &self.alternates)?;
        m.serialize_entry("args", &self.args)?;
        m.end()
    }
}

impl Serialize for Fit {
    /// Serialize as `{ path, hole, shaft, clearance, type, required? }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("hole", &self.hole)?;
        m.serialize_entry("shaft", &self.shaft)?;
        m.serialize_entry("clearance", &self.clearance)?;
        m.serialize_entry("type", &self.fit_type)?;
        opt(&mut m, "required", &self.required)?;
        m.end()
    }
}

impl Serialize for Component {
    /// Serialize per ir.md 3 `Component`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("class", &self.class)?;
        opt(&mut m, "params_sig", &self.params_sig)?;
        m.serialize_entry("implements", &self.implements)?;
        opt(&mut m, "citation", &self.citation)?;
        m.serialize_entry("attrs", &self.attrs)?;
        m.serialize_entry("members", &self.members)?;
        m.serialize_entry("parts", &self.parts)?;
        m.serialize_entry("fits", &self.fits)?;
        m.serialize_entry("datums", &self.datums)?;
        m.serialize_entry("points", &self.points)?;
        m.serialize_entry("placements", &self.placements)?;
        opt(&mut m, "span", &self.span)?;
        opt(&mut m, "digest", &self.digest)?;
        m.end()
    }
}

impl Serialize for Verdicts {
    /// Serialize as `{ primary?, per_alternate }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        opt(&mut m, "primary", &self.primary)?;
        m.serialize_entry("per_alternate", &self.per_alternate)?;
        m.end()
    }
}

impl Serialize for Check {
    /// Serialize as `{ path, expr, verdicts, conservative, waived_for }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Check", 5)?;
        st.serialize_field("path", &self.path)?;
        st.serialize_field("expr", &self.expr)?;
        st.serialize_field("verdicts", &self.verdicts)?;
        st.serialize_field("conservative", &self.conservative)?;
        st.serialize_field("waived_for", &self.waived_for)?;
        st.end()
    }
}

impl Serialize for Solver {
    /// Serialize per ir.md 3 `Solver`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("path", &self.path)?;
        m.serialize_entry("script", &self.script)?;
        opt(&mut m, "citation", &self.citation)?;
        m.serialize_entry("inputs", &self.inputs)?;
        m.serialize_entry("outputs", &self.outputs)?;
        m.serialize_entry("deps", &self.deps)?;
        opt(&mut m, "result_digest", &self.result_digest)?;
        m.serialize_entry("stale", &self.stale)?;
        m.end()
    }
}

impl Serialize for Locator {
    /// Serialize per ir.md 3 `Locator`; `for_alternate` is written as `for`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("locator", &self.locator)?;
        m.serialize_entry("kind", &self.kind)?;
        m.serialize_entry("mode", &self.mode)?;
        m.serialize_entry("native_unit", &self.native_unit)?;
        opt(&mut m, "ref_to", &self.ref_to)?;
        opt(&mut m, "for", &self.for_alternate)?;
        opt(&mut m, "config", &self.config)?;
        opt(&mut m, "sign", &self.sign)?;
        opt(&mut m, "component", &self.component)?;
        opt(&mut m, "implicit", &self.implicit)?;
        opt(&mut m, "tolerance", &self.tolerance)?;
        m.end()
    }
}

impl Serialize for ObservedFrame {
    /// Serialize as `{ name, document, transform }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("ObservedFrame", 3)?;
        st.serialize_field("name", &self.name)?;
        st.serialize_field("document", &self.document)?;
        st.serialize_field("transform", &self.transform)?;
        st.end()
    }
}

impl Serialize for ObservedInstance {
    /// Serialize as `{ component, document, reference?, transform }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("component", &self.component)?;
        m.serialize_entry("document", &self.document)?;
        opt(&mut m, "reference", &self.reference)?;
        m.serialize_entry("transform", &self.transform)?;
        m.end()
    }
}

impl Serialize for Binding {
    /// Serialize as `{ path, adapter, locators }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Binding", 3)?;
        st.serialize_field("path", &self.path)?;
        st.serialize_field("adapter", &self.adapter)?;
        st.serialize_field("locators", &self.locators)?;
        st.end()
    }
}

impl Serialize for PackageId {
    /// Serialize as `{ name, version }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("PackageId", 2)?;
        st.serialize_field("name", &self.name)?;
        st.serialize_field("version", &self.version)?;
        st.end()
    }
}

impl Serialize for Dependency {
    /// Serialize as `{ name, version, source, digest }`.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Dependency", 4)?;
        st.serialize_field("name", &self.name)?;
        st.serialize_field("version", &self.version)?;
        st.serialize_field("source", &self.source)?;
        st.serialize_field("digest", &self.digest)?;
        st.end()
    }
}

impl Serialize for Project {
    /// Serialize the whole IR as a `cadetta.ir/1` document.
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        debug!(package = %self.package.name, "ir json: serialize project");
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("schema", &self.schema)?;
        m.serialize_entry("flavour", &self.flavour)?;
        m.serialize_entry("package", &self.package)?;
        m.serialize_entry("modules", &self.modules)?;
        m.serialize_entry("dependencies", &self.dependencies)?;
        m.serialize_entry("components", &self.components)?;
        m.serialize_entry("checks", &self.checks)?;
        m.serialize_entry("solvers", &self.solvers)?;
        m.serialize_entry("bindings", &self.bindings)?;
        m.serialize_entry("citations", &self.citations)?;
        // frob:ticket 01M4YKQSGPM7JTKPRTZXSHCDWP
        if !self.interfaces.is_empty() {
            m.serialize_entry("interfaces", &self.interfaces)?;
        }
        if !self.tables.is_empty() {
            m.serialize_entry("tables", &self.tables)?;
        }
        // frob:ticket 01M4YKTX2YMSXVE4CK0JAVMC41
        if !self.requires.is_empty() {
            m.serialize_entry("requires", &self.requires)?;
        }
        if !self.waivers.is_empty() {
            m.serialize_entry("waivers", &self.waivers)?;
        }
        // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
        if !self.frames.is_empty() {
            m.serialize_entry("frames", &self.frames)?;
        }
        if !self.instances.is_empty() {
            m.serialize_entry("instances", &self.instances)?;
        }
        opt(&mut m, "digest", &self.digest)?;
        m.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathContext;
    use cadetta_units::{Dim, Magnitude, Rational, UnitTable};
    use serde_json::Value as J;
    use std::collections::BTreeMap;

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn ex(n: i64, d: i64, pi: i32) -> Exact {
        Exact(Magnitude::new(Rational::new(n, d).unwrap(), pi))
    }

    fn project() -> Project {
        let mut pr = Project::new(
            Flavour::Design,
            PackageId {
                name: "acme.rover".into(),
                version: "0.1.0".into(),
            },
        );
        let mm = UnitTable::builtin().parse("mm").unwrap();
        let mut v = Value::new(Nominal::Exact(ex(127, 5000, 0)), mm.dim.clone(), "5.08 mm");
        v.tol = Some(Affine {
            terms: vec![AffineTerm {
                var: p("acme.rover::Axle.d"),
                lo: ex(-1, 2000, 0),
                hi: ex(1, 2000, 1),
            }],
        });
        v.drawn = Some(ex(3, 1, 0));
        let mut m = Member::new(p("acme.rover::Axle.d"), MemberKind::Param);
        m.value = Some(v);
        m.bound = Some(Bound {
            op: BoundOp::Le,
            lo: None,
            hi: Some(ex(1, 1, 0)),
        });
        let mut comp = Component {
            path: p("acme.rover::Axle.d"),
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: BTreeMap::new(),
            members: vec![m],
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        };
        let mut m2 = Member::new(p("acme.rover::Axle.e"), MemberKind::Param);
        let inexact = Inexact {
            f64: 1.5,
            root: p("acme.rover::Axle.e"),
        };
        m2.value = Some(Value::new(
            Nominal::Inexact(inexact),
            Dim::default(),
            "sqrt(2)",
        ));
        comp.members.push(m2);
        pr.components.push(comp);
        pr
    }

    /// Visit every JSON number; `f` gets (is_float, parent key chain ends in f64?).
    fn floats(v: &J, key: &str, out: &mut Vec<String>) {
        match v {
            J::Number(n) if n.is_f64() => out.push(key.to_owned()),
            J::Array(a) => a.iter().for_each(|x| floats(x, key, out)),
            J::Object(o) => o.iter().for_each(|(k, x)| floats(x, k, out)),
            _ => {}
        }
    }

    // frob:tests json_rational_string_reduced
    #[test]
    fn json_rational_string_reduced() {
        let j = serde_json::to_value(ex(127, 5000, 0)).unwrap();
        assert_eq!(j["r"], "127/5000");
        let j = serde_json::to_value(ex(254, -10000, 0)).unwrap();
        assert_eq!(j["r"], "-127/5000");
        assert_eq!(serde_json::to_value(ex(4, 2, 0)).unwrap()["r"], "2/1");
        assert_eq!(serde_json::to_value(ex(0, 7, 0)).unwrap()["r"], "0/1");
    }

    // frob:tests json_no_magnitude_float
    #[test]
    fn json_no_magnitude_float() {
        let mut pr = project();
        // Strip the inexact member: the rest of the IR must hold no float at all.
        pr.components[0].members.pop();
        let j = serde_json::to_value(&pr).unwrap();
        let mut f = vec![];
        floats(&j, "", &mut f);
        assert!(f.is_empty(), "{f:?}");
        let n = &j["components"][0]["members"][0]["value"];
        assert_eq!(n["nominal"]["r"], "127/5000");
        assert!(n["nominal"]["pi"].is_i64());
        assert_eq!(n["tol"]["terms"][0]["hi"]["pi"], 1);
        assert_eq!(n["dim"]["m"], 1);
    }

    // frob:tests json_inexact_only_float
    #[test]
    fn json_inexact_only_float() {
        let j = serde_json::to_value(project()).unwrap();
        let mut f = vec![];
        floats(&j, "", &mut f);
        assert_eq!(f, vec!["f64"]);
        let n = &j["components"][0]["members"][1]["value"]["nominal"];
        assert_eq!(n["f64"], 1.5);
        assert_eq!(n["root"], "acme.rover::Axle.e");
        let bad = Inexact {
            f64: f64::NAN,
            root: p("acme.rover::Axle.e"),
        };
        assert!(serde_json::to_string(&bad).is_err());
    }

    // frob:tests json_exact_shape
    #[test]
    fn json_exact_shape() {
        let j = serde_json::to_value(ex(1, 2, 2)).unwrap();
        assert_eq!(j, serde_json::json!({ "r": "1/2", "pi": 2 }));
        let j = serde_json::to_value(project()).unwrap();
        assert_eq!(j["schema"], "cadetta.ir/1");
        assert_eq!(j["flavour"], "design");
        let m = &j["components"][0]["members"][0];
        assert_eq!(m["priv"], false);
        assert_eq!(m["provenance"], "authored");
        assert_eq!(m["bound"]["op"], "<=");
        assert!(m["bound"].get("lo").is_none());
    }
}
