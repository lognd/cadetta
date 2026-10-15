//! JSON decoding of the `cadetta.ir/1` model (ir.md 3): the inverse of the [`crate::json`]
//! encoder for the entities the IR-only lint rules read.
//!
//! Input is a document that already passed the JSON Schema (`ir-validate`), so shape errors here
//! are reported with a JSON pointer but are not expected. Decoded: package, modules, components
//! (members, parts, fits, citation, attrs, span), solvers and citations. Not yet decoded, and
//! left empty on the result: dependencies, checks, bindings, interfaces, tables, waivers, frames,
//! instances, and a component's datums, points and placements. `digest` fields are carried as
//! given, never recomputed.
// frob:ticket 01M4YNMDMKRAP06VPQP71ZWJXV

use std::collections::BTreeMap;

use cadetta_units::{Magnitude, Rational};
use serde_json::Value as Json;
use tracing::{debug, trace};

use crate::dim::DimWire;
use crate::model::{
    Affine, AffineTerm, Bound, BoundOp, CiteBuiltin, CiteField, CiteRef, Citation, CitationType,
    Component, ComponentClass, Digest, Exact, Fit, Flavour, Inexact, Member, MemberKind, Nominal,
    PackageId, Part, Project, Provenance, Solver, SpanRef, Value,
};
use crate::path::Path;

/// A decode failure: the JSON pointer of the offending node and what was wrong.
pub type DecodeError = String;

/// Decoding result.
type Res<T> = Result<T, DecodeError>;

fn at(ptr: &str, what: impl std::fmt::Display) -> DecodeError {
    format!("{ptr}: {what}")
}

fn field<'a>(o: &'a Json, ptr: &str, key: &str) -> Res<&'a Json> {
    o.get(key).ok_or_else(|| at(ptr, format!("missing `{key}`")))
}

fn text(o: &Json, ptr: &str, key: &str) -> Res<String> {
    field(o, ptr, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| at(&format!("{ptr}/{key}"), "expected a string"))
}

fn opt_text(o: &Json, key: &str) -> Option<String> {
    o.get(key).and_then(Json::as_str).map(str::to_owned)
}

fn flag(o: &Json, key: &str) -> bool {
    o.get(key).and_then(Json::as_bool).unwrap_or(false)
}

fn items<'a>(o: &'a Json, key: &str) -> &'a [Json] {
    o.get(key).and_then(Json::as_array).map_or(&[], Vec::as_slice)
}

fn string_map(o: &Json, key: &str) -> BTreeMap<String, String> {
    o.get(key)
        .and_then(Json::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
        .collect()
}

fn strings(o: &Json, key: &str) -> Vec<String> {
    items(o, key).iter().filter_map(|v| v.as_str().map(str::to_owned)).collect()
}

fn path_text(s: &str, ptr: &str) -> Res<Path> {
    s.parse::<Path>().map_err(|e| at(ptr, e))
}

fn path(o: &Json, ptr: &str, key: &str) -> Res<Path> {
    path_text(&text(o, ptr, key)?, &format!("{ptr}/{key}"))
}

fn paths(o: &Json, ptr: &str, key: &str) -> Res<Vec<Path>> {
    items(o, key)
        .iter()
        .enumerate()
        .map(|(i, v)| path_text(v.as_str().unwrap_or_default(), &format!("{ptr}/{key}/{i}")))
        .collect()
}

/// An item path (`pkg::mod::Name`, no member tail): parsed with a placeholder tail, then cleared.
fn item_path(s: &str, ptr: &str) -> Res<Path> {
    let mut p = path_text(&format!("{s}.x"), ptr)?;
    p.members.clear();
    Ok(p)
}

fn exact(o: &Json, ptr: &str) -> Res<Exact> {
    let r = text(o, ptr, "r")?;
    let (n, d) = r.split_once('/').ok_or_else(|| at(ptr, "`r` is not `num/den`"))?;
    let rat = |s: &str| Rational::parse_decimal(s).map_err(|e| at(ptr, e));
    let q = rat(n)?.div(&rat(d)?).map_err(|e| at(ptr, e))?;
    let pi = field(o, ptr, "pi")?.as_i64().ok_or_else(|| at(ptr, "`pi` is not an integer"))?;
    let pi = i32::try_from(pi).map_err(|_| at(ptr, "`pi` out of range"))?;
    Ok(Exact(Magnitude::new(q, pi)))
}

fn bound(o: &Json, ptr: &str) -> Res<Bound> {
    let op = match text(o, ptr, "op")?.as_str() {
        "<=" => BoundOp::Le,
        ">=" => BoundOp::Ge,
        "in" => BoundOp::In,
        other => return Err(at(ptr, format!("unknown bound op `{other}`"))),
    };
    let limit = |key: &str| o.get(key).map(|v| exact(v, &format!("{ptr}/{key}"))).transpose();
    Ok(Bound { op, lo: limit("lo")?, hi: limit("hi")? })
}

fn value(o: &Json, ptr: &str) -> Res<Value> {
    let n = field(o, ptr, "nominal")?;
    let nominal = if n.get("f64").is_some() {
        let f = n["f64"].as_f64().ok_or_else(|| at(ptr, "`nominal.f64` is not a number"))?;
        Nominal::Inexact(Inexact { f64: f, root: path(n, &format!("{ptr}/nominal"), "root")? })
    } else {
        Nominal::Exact(exact(n, &format!("{ptr}/nominal"))?)
    };
    let wire: DimWire = serde_json::from_value(field(o, ptr, "dim")?.clone())
        .map_err(|e| at(&format!("{ptr}/dim"), e))?;
    let tol = match o.get("tol") {
        None => None,
        Some(t) => {
            let mut terms = Vec::new();
            for (i, term) in items(t, "terms").iter().enumerate() {
                let tp = format!("{ptr}/tol/terms/{i}");
                terms.push(AffineTerm {
                    var: path(term, &tp, "var")?,
                    lo: exact(field(term, &tp, "lo")?, &format!("{tp}/lo"))?,
                    hi: exact(field(term, &tp, "hi")?, &format!("{tp}/hi"))?,
                });
            }
            Some(Affine { terms })
        }
    };
    Ok(Value {
        nominal,
        dim: (&wire).into(),
        angle: i32::try_from(o.get("angle").and_then(Json::as_i64).unwrap_or(0))
            .map_err(|_| at(ptr, "`angle` out of range"))?,
        tol,
        conservative: flag(o, "conservative"),
        drawn: o.get("drawn").map(|d| exact(d, &format!("{ptr}/drawn"))).transpose()?,
        exact: o.get("exact").and_then(Json::as_bool).unwrap_or(true),
        reason: opt_text(o, "reason"),
        unit_assumed: opt_text(o, "unit_assumed"),
        authored: text(o, ptr, "authored")?,
    })
}

fn span(o: &Json) -> Option<SpanRef> {
    let s = o.get("span")?;
    Some(SpanRef {
        file: s.get("file")?.as_str()?.to_owned(),
        start: u32::try_from(s.get("start")?.as_u64()?).ok()?,
        end: u32::try_from(s.get("end")?.as_u64()?).ok()?,
    })
}

fn digest(o: &Json, key: &str) -> Option<Digest> {
    opt_text(o, key).map(Digest)
}

fn cite_ref(o: &Json, ptr: &str) -> Res<Option<CiteRef>> {
    let Some(c) = o.get("citation") else { return Ok(None) };
    let builtin = match opt_text(c, "builtin").as_deref() {
        None => None,
        Some("note") => Some(CiteBuiltin::Note),
        Some("measured") => Some(CiteBuiltin::Measured),
        Some("assumed") => Some(CiteBuiltin::Assumed),
        Some("estimate") => Some(CiteBuiltin::Estimate),
        Some(other) => return Err(at(&format!("{ptr}/citation"), format!("unknown builtin `{other}`"))),
    };
    Ok(Some(CiteRef {
        cite: opt_text(c, "cite"),
        builtin,
        text: opt_text(c, "text"),
        locator: opt_text(c, "locator"),
    }))
}

fn provenance(o: &Json, ptr: &str) -> Res<Provenance> {
    match field(o, ptr, "provenance")? {
        Json::String(s) if s == "authored" => Ok(Provenance::Authored),
        Json::String(s) if s == "derived" => Ok(Provenance::Derived),
        other => {
            let imp = other.get("imported").ok_or_else(|| at(ptr, "unknown provenance"))?;
            let ip = format!("{ptr}/provenance/imported");
            Ok(Provenance::Imported {
                adapter: text(imp, &ip, "adapter")?,
                file: text(imp, &ip, "file")?,
                native_name: text(imp, &ip, "native_name")?,
            })
        }
    }
}

fn member(o: &Json, ptr: &str) -> Res<Member> {
    let kind = match text(o, ptr, "kind")?.as_str() {
        "param" => MemberKind::Param,
        "let" => MemberKind::Let,
        "bound" => MemberKind::Bound,
        "datum" => MemberKind::Datum,
        "place" => MemberKind::Place,
        other => return Err(at(ptr, format!("unknown member kind `{other}`"))),
    };
    let variants = match o.get("variants").and_then(Json::as_object) {
        None => None,
        Some(m) => Some(
            m.iter()
                .map(|(k, v)| Ok((k.clone(), value(v, &format!("{ptr}/variants/{k}"))?)))
                .collect::<Res<BTreeMap<_, _>>>()?,
        ),
    };
    Ok(Member {
        path: path(o, ptr, "path")?,
        kind,
        analysis: flag(o, "analysis"),
        private: flag(o, "priv"),
        value: o.get("value").map(|v| value(v, &format!("{ptr}/value"))).transpose()?,
        bound: o.get("bound").map(|b| bound(b, &format!("{ptr}/bound"))).transpose()?,
        expr: opt_text(o, "expr"),
        deps: paths(o, ptr, "deps")?,
        role: opt_text(o, "role"),
        citation: cite_ref(o, ptr)?,
        was: strings(o, "was"),
        attrs: string_map(o, "attrs"),
        provenance: provenance(o, ptr)?,
        doc: opt_text(o, "doc"),
        span: span(o),
        variants,
        digest: digest(o, "digest"),
        closure: digest(o, "closure"),
    })
}

fn part(o: &Json, ptr: &str) -> Res<Part> {
    Ok(Part {
        path: path(o, ptr, "path")?,
        type_name: text(o, ptr, "type")?,
        count: o.get("count").and_then(Json::as_u64).and_then(|n| u32::try_from(n).ok()),
        primary: text(o, ptr, "primary")?,
        alternates: strings(o, "alternates"),
        args: string_map(o, "args"),
    })
}

fn fit(o: &Json, ptr: &str) -> Res<Fit> {
    Ok(Fit {
        path: path(o, ptr, "path")?,
        hole: path(o, ptr, "hole")?,
        shaft: path(o, ptr, "shaft")?,
        clearance: value(field(o, ptr, "clearance")?, &format!("{ptr}/clearance"))?,
        fit_type: text(o, ptr, "type")?,
        required: o.get("required").map(|b| bound(b, &format!("{ptr}/required"))).transpose()?,
    })
}

fn many<T>(o: &Json, ptr: &str, key: &str, f: impl Fn(&Json, &str) -> Res<T>) -> Res<Vec<T>> {
    items(o, key)
        .iter()
        .enumerate()
        .map(|(i, v)| f(v, &format!("{ptr}/{key}/{i}")))
        .collect()
}

fn component(o: &Json, ptr: &str) -> Res<Component> {
    let class = match text(o, ptr, "class")?.as_str() {
        "ots" => ComponentClass::Ots,
        "model" => ComponentClass::Model,
        "assembly" => ComponentClass::Assembly,
        "process" => ComponentClass::Process,
        other => return Err(at(ptr, format!("unknown class `{other}`"))),
    };
    Ok(Component {
        path: item_path(&text(o, ptr, "path")?, &format!("{ptr}/path"))?,
        class,
        params_sig: opt_text(o, "params_sig"),
        implements: strings(o, "implements"),
        citation: cite_ref(o, ptr)?,
        attrs: string_map(o, "attrs"),
        members: many(o, ptr, "members", member)?,
        parts: many(o, ptr, "parts", part)?,
        fits: many(o, ptr, "fits", fit)?,
        datums: Vec::new(),
        points: Vec::new(),
        placements: Vec::new(),
        span: span(o),
        digest: digest(o, "digest"),
    })
}

fn solver(o: &Json, ptr: &str) -> Res<Solver> {
    Ok(Solver {
        path: path(o, ptr, "path")?,
        script: text(o, ptr, "script")?,
        inputs: string_map(o, "inputs"),
        outputs: string_map(o, "outputs"),
        deps: paths(o, ptr, "deps")?,
        result_digest: digest(o, "result_digest"),
        stale: flag(o, "stale"),
    })
}

fn citation(o: &Json, ptr: &str) -> Res<Citation> {
    let ty_name = text(o, ptr, "type")?;
    let ty = CitationType::parse(&ty_name).ok_or_else(|| at(ptr, format!("unknown citation type `{ty_name}`")))?;
    let fields = o
        .get("fields")
        .and_then(Json::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| {
            let f = match v {
                Json::String(s) => CiteField::Text(s.clone()),
                Json::Array(a) => CiteField::List(a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()),
                _ => return None,
            };
            Some((k.clone(), f))
        })
        .collect();
    Ok(Citation {
        path: text(o, ptr, "path")?,
        ty,
        fields,
        file_digest: digest(o, "file_digest"),
        digest: Digest(text(o, ptr, "digest")?),
    })
}

/// Decode a schema-valid `cadetta.ir/1` document into the model; see the module docs for which
/// entities are carried. The error names the JSON pointer of the node that could not be read.
pub fn decode_project(doc: &Json) -> Result<Project, DecodeError> {
    let pkg = field(doc, "", "package")?;
    let package = PackageId { name: text(pkg, "/package", "name")?, version: text(pkg, "/package", "version")? };
    let flavour = match text(doc, "", "flavour")?.as_str() {
        "design" => Flavour::Design,
        "observed" => Flavour::Observed,
        other => return Err(at("/flavour", format!("unknown flavour `{other}`"))),
    };
    trace!(package = %package.name, "ir decode: start");
    let mut project = Project::new(flavour, package);
    project.modules = strings(doc, "modules");
    project.components = many(doc, "", "components", component)?;
    project.solvers = many(doc, "", "solvers", solver)?;
    project.citations = many(doc, "", "citations", citation)?;
    project.digest = digest(doc, "digest");
    debug!(
        components = project.components.len(),
        solvers = project.solvers.len(),
        citations = project.citations.len(),
        "ir decode: done"
    );
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests decode_roundtrips_a_serialized_project
    #[test]
    fn decode_roundtrips_a_serialized_project() {
        let pkg = PackageId { name: "acme.rover".into(), version: "1.0.0".into() };
        let mut p = Project::new(Flavour::Design, pkg);
        let mp: Path = "acme.rover::Axle.bore".parse().unwrap();
        let mut m = Member::new(mp, MemberKind::Param);
        let dim = cadetta_units::Dim::new(cadetta_units::Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0);
        let mag = Magnitude::new(Rational::new(25, 1000).unwrap(), 0);
        m.value = Some(Value::new(Nominal::Exact(Exact(mag)), dim, "25 mm"));
        m.expr = Some("25 mm".into());
        let mut c = Component {
            path: item_path("acme.rover::Axle", "").unwrap(),
            class: ComponentClass::Model,
            params_sig: None,
            implements: Vec::new(),
            citation: None,
            attrs: BTreeMap::new(),
            members: vec![m],
            parts: Vec::new(),
            fits: Vec::new(),
            datums: Vec::new(),
            points: Vec::new(),
            placements: Vec::new(),
            span: Some(SpanRef { file: "a.cdt".into(), start: 1, end: 9 }),
            digest: None,
        };
        c.attrs.insert("k".into(), "v".into());
        p.components.push(c);
        let json = serde_json::to_value(&p).unwrap();
        let back = decode_project(&json).unwrap();
        assert_eq!(back, p);
    }

    // frob:tests decode_reports_a_json_pointer
    #[test]
    fn decode_reports_a_json_pointer() {
        let doc = serde_json::json!({
            "package": {"name": "a", "version": "1"}, "flavour": "design",
            "components": [{"path": "a::C", "class": "bogus"}]
        });
        let err = decode_project(&doc).unwrap_err();
        assert!(err.starts_with("/components/0:"), "{err}");
    }
}
