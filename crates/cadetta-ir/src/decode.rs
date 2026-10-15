//! JSON decoding of the `cadetta.ir/1` model (ir.md 3): the inverse of the [`crate::json`]
//! encoder for the entities the IR-only lint rules read.
//!
//! Input is a document that already passed the JSON Schema (`ir-validate`), so shape errors here
//! are reported with a JSON pointer but are not expected. Decoded: package, modules, components
//! (members, parts, fits, citation, attrs, span, datums, points, placements), dependencies,
//! checks, solvers, bindings, citations, interfaces, tables, waivers and the observed frames and
//! instances: every top-level entity of the document. `digest` fields are carried as given,
//! never recomputed.
// frob:ticket 01M4YNMDMKRAP06VPQP71ZWJXV
// frob:ticket 01M4YP4QF161QNHCR0Y06GWDX5

use std::collections::BTreeMap;

use cadetta_units::{Magnitude, Rational};
use serde_json::Value as Json;
use tracing::{debug, trace};

use crate::dim::DimWire;
use crate::entities::{
    Cell, Interface, InterfaceBound, InterfaceMember, Table, TableCol, TableRow, Waiver,
};
use crate::geometry::{Axis, Datum, OffsetArgs, Op, Placement, Point};
use crate::model::{
    Affine, AffineTerm, Binding, Bound, BoundOp, CiteBuiltin, CiteField, CiteRef, Citation,
    CitationType, Check, Component, ComponentClass, Dependency, Digest, Exact, Fit, Flavour,
    Implicit, Inexact, Locator, LocatorKind, LocatorMode, Member, MemberKind, Nominal,
    ObservedFrame, ObservedInstance, PackageId, Part, Project, Provenance, Sign, Solver, SpanRef,
    TolerancePolicy, Value, VerdictState, Verdicts,
};
use crate::path::Path;
use crate::variant_name::{ComponentPath, VariantName};

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

/// A frame reference: a datum member path, or a bare item path naming the component origin.
fn frame_path(o: &Json, ptr: &str, key: &str) -> Res<Path> {
    let s = text(o, ptr, key)?;
    path_text(&s, &format!("{ptr}/{key}")).or_else(|_| item_path(&s, &format!("{ptr}/{key}")))
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
        datums: many(o, ptr, "datums", datum)?,
        points: many(o, ptr, "points", point)?,
        placements: many(o, ptr, "placements", placement)?,
        span: span(o),
        digest: digest(o, "digest"),
    })
}

fn axis(s: &str, ptr: &str) -> Res<Axis> {
    match s {
        "x" => Ok(Axis::X),
        "y" => Ok(Axis::Y),
        "z" => Ok(Axis::Z),
        other => Err(at(ptr, format!("unknown axis `{other}`"))),
    }
}

fn op(o: &Json, ptr: &str) -> Res<Op> {
    if let Some(off) = o.get("offset") {
        let axis_path = |k: &str| {
            off.get(k)
                .and_then(Json::as_str)
                .map(|s| path_text(s, &format!("{ptr}/offset/{k}")))
                .transpose()
        };
        return Ok(Op::Offset(Box::new(OffsetArgs { x: axis_path("x")?, y: axis_path("y")?, z: axis_path("z")? })));
    }
    if let Some(rot) = o.get("rotate") {
        let rp = format!("{ptr}/rotate");
        return Ok(Op::Rotate { axis: axis(&text(rot, &rp, "axis")?, &rp)?, angle: path(rot, &rp, "angle")? });
    }
    match o.get("flip").and_then(Json::as_str) {
        Some(a) => Ok(Op::Flip { axis: axis(a, &format!("{ptr}/flip"))? }),
        None => Err(at(ptr, "unknown op")),
    }
}

fn op_chain(arr: &Json, ptr: &str) -> Res<Vec<Op>> {
    arr.as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .enumerate()
        .map(|(i, v)| op(v, &format!("{ptr}/{i}")))
        .collect()
}

fn op_variants(o: &Json, ptr: &str) -> Res<Option<BTreeMap<String, Vec<Op>>>> {
    let Some(m) = o.get("variants").and_then(Json::as_object) else { return Ok(None) };
    m.iter()
        .map(|(k, v)| Ok((k.clone(), op_chain(v, &format!("{ptr}/variants/{k}"))?)))
        .collect::<Res<BTreeMap<_, _>>>()
        .map(Some)
}

fn datum(o: &Json, ptr: &str) -> Res<Datum> {
    Ok(Datum {
        path: path(o, ptr, "path")?,
        parent: frame_path(o, ptr, "parent")?,
        ops: op_chain(field(o, ptr, "ops")?, &format!("{ptr}/ops"))?,
        variants: op_variants(o, ptr)?,
        digest: digest(o, "digest"),
        closure: digest(o, "closure"),
    })
}

fn point(o: &Json, ptr: &str) -> Res<Point> {
    Ok(Point { path: path(o, ptr, "path")?, datum: frame_path(o, ptr, "datum")?, coords: many(o, ptr, "coords", value)? })
}

fn placement(o: &Json, ptr: &str) -> Res<Placement> {
    Ok(Placement {
        path: path(o, ptr, "path")?,
        part: path(o, ptr, "part")?,
        datum: frame_path(o, ptr, "datum")?,
        at: path(o, ptr, "at")?,
        ops: op_chain(field(o, ptr, "ops")?, &format!("{ptr}/ops"))?,
        variants: op_variants(o, ptr)?,
        digest: digest(o, "digest"),
        closure: digest(o, "closure"),
    })
}

fn verdict(s: &str, ptr: &str) -> Res<VerdictState> {
    match s {
        "pass" => Ok(VerdictState::Pass),
        "fail" => Ok(VerdictState::Fail),
        "unknown" => Ok(VerdictState::Unknown),
        other => Err(at(ptr, format!("unknown verdict `{other}`"))),
    }
}

fn check(o: &Json, ptr: &str) -> Res<Check> {
    let v = field(o, ptr, "verdicts")?;
    let vp = format!("{ptr}/verdicts");
    let primary = v.get("primary").and_then(Json::as_str).map(|s| verdict(s, &vp)).transpose()?;
    let per_alternate = v
        .get("per_alternate")
        .and_then(Json::as_object)
        .into_iter()
        .flatten()
        .map(|(k, s)| Ok((k.clone(), verdict(s.as_str().unwrap_or_default(), &format!("{vp}/per_alternate/{k}"))?)))
        .collect::<Res<BTreeMap<_, _>>>()?;
    Ok(Check {
        path: path(o, ptr, "path")?,
        expr: text(o, ptr, "expr")?,
        verdicts: Verdicts { primary, per_alternate },
        conservative: flag(o, "conservative"),
        waived_for: strings(o, "waived_for"),
    })
}

fn locator(o: &Json, ptr: &str) -> Res<Locator> {
    let kind = match text(o, ptr, "kind")?.as_str() {
        "global" => LocatorKind::Global,
        "dimension" => LocatorKind::Dimension,
        "property" => LocatorKind::Property,
        "frame" => LocatorKind::Frame,
        "instance" => LocatorKind::Instance,
        other => return Err(at(ptr, format!("unknown locator kind `{other}`"))),
    };
    let mode = match text(o, ptr, "mode")?.as_str() {
        "value" => LocatorMode::Value,
        "ref" => LocatorMode::Ref,
        other => return Err(at(ptr, format!("unknown locator mode `{other}`"))),
    };
    let sign = match o.get("sign").and_then(Json::as_i64) {
        None => None,
        Some(1) => Some(Sign::Plus),
        Some(-1) => Some(Sign::Minus),
        Some(other) => return Err(at(ptr, format!("sign must be 1 or -1, got {other}"))),
    };
    let implicit = match opt_text(o, "implicit").as_deref() {
        None => None,
        Some("global") => Some(Implicit::Global),
        Some("link") => Some(Implicit::Link),
        Some("frame") => Some(Implicit::Frame),
        Some("reference") => Some(Implicit::Reference),
        Some(other) => return Err(at(ptr, format!("unknown implicit `{other}`"))),
    };
    let tolerance = match opt_text(o, "tolerance").as_deref() {
        None => None,
        Some("drive") => Some(TolerancePolicy::Drive),
        Some("ignore") => Some(TolerancePolicy::Ignore),
        Some(other) => return Err(at(ptr, format!("unknown tolerance `{other}`"))),
    };
    Ok(Locator {
        locator: text(o, ptr, "locator")?,
        kind,
        mode,
        native_unit: text(o, ptr, "native_unit")?,
        ref_to: opt_text(o, "ref_to"),
        for_alternate: opt_text(o, "for"),
        config: opt_text(o, "config"),
        sign,
        component: opt_text(o, "component"),
        implicit,
        tolerance,
    })
}

fn binding(o: &Json, ptr: &str) -> Res<Binding> {
    Ok(Binding {
        path: path(o, ptr, "path")?,
        adapter: text(o, ptr, "adapter")?,
        locators: many(o, ptr, "locators", locator)?,
    })
}

fn dependency(o: &Json, ptr: &str) -> Res<Dependency> {
    Ok(Dependency {
        name: text(o, ptr, "name")?,
        version: text(o, ptr, "version")?,
        source: text(o, ptr, "source")?,
        digest: text(o, ptr, "digest")?,
    })
}

fn component_path(s: &str, ptr: &str) -> Res<ComponentPath> {
    ComponentPath::parse(s).map_err(|e| at(ptr, e))
}

fn interface(o: &Json, ptr: &str) -> Res<Interface> {
    Ok(Interface {
        path: component_path(&text(o, ptr, "path")?, &format!("{ptr}/path"))?,
        members: many(o, ptr, "members", |m, p| {
            Ok(InterfaceMember {
                name: text(m, p, "name")?,
                type_name: text(m, p, "type")?,
                optional: flag(m, "optional"),
            })
        })?,
        bounds: many(o, ptr, "bounds", |b, p| {
            Ok(InterfaceBound { name: text(b, p, "name")?, bound: bound(field(b, p, "bound")?, &format!("{p}/bound"))? })
        })?,
        datums: strings(o, "datums"),
    })
}

fn col(o: &Json, ptr: &str) -> Res<TableCol> {
    Ok(TableCol { name: text(o, ptr, "name")?, type_name: text(o, ptr, "type")? })
}

fn cell(o: &Json, ptr: &str) -> Res<Cell> {
    match o.get("variant").and_then(Json::as_str) {
        Some(name) => Ok(Cell::Variant(name.to_owned())),
        None => Ok(Cell::Value(Box::new(value(o, ptr)?))),
    }
}

fn table(o: &Json, ptr: &str) -> Res<Table> {
    Ok(Table {
        path: component_path(&text(o, ptr, "path")?, &format!("{ptr}/path"))?,
        key: col(field(o, ptr, "key")?, &format!("{ptr}/key"))?,
        cols: many(o, ptr, "cols", col)?,
        rows: many(o, ptr, "rows", |r, p| {
            Ok(TableRow { key: cell(field(r, p, "key")?, &format!("{p}/key"))?, cells: many(r, p, "cells", cell)? })
        })?,
        citation: cite_ref(o, ptr)?,
    })
}

fn waiver(o: &Json, ptr: &str) -> Res<Waiver> {
    let mut w = Waiver::new(text(o, ptr, "code")?, text(o, ptr, "glob")?, text(o, ptr, "because")?);
    w.expires = opt_text(o, "expires");
    w.for_alt = opt_text(o, "for_alt")
        .map(|s| s.parse::<VariantName>().map_err(|e| at(&format!("{ptr}/for_alt"), e)))
        .transpose()?;
    w.targets_closure = string_map(o, "targets_closure").into_iter().map(|(k, v)| (k, Digest(v))).collect();
    Ok(w)
}

fn matrix(o: &Json, ptr: &str) -> Res<Vec<Vec<Value>>> {
    items(o, "transform")
        .iter()
        .enumerate()
        .map(|(i, row)| {
            row.as_array()
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .enumerate()
                .map(|(j, v)| value(v, &format!("{ptr}/transform/{i}/{j}")))
                .collect()
        })
        .collect()
}

fn frame(o: &Json, ptr: &str) -> Res<ObservedFrame> {
    Ok(ObservedFrame { name: text(o, ptr, "name")?, document: text(o, ptr, "document")?, transform: matrix(o, ptr)? })
}

fn instance(o: &Json, ptr: &str) -> Res<ObservedInstance> {
    Ok(ObservedInstance {
        component: text(o, ptr, "component")?,
        document: text(o, ptr, "document")?,
        reference: opt_text(o, "reference"),
        transform: matrix(o, ptr)?,
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
    project.dependencies = many(doc, "", "dependencies", dependency)?;
    project.components = many(doc, "", "components", component)?;
    project.checks = many(doc, "", "checks", check)?;
    project.solvers = many(doc, "", "solvers", solver)?;
    project.bindings = many(doc, "", "bindings", binding)?;
    project.citations = many(doc, "", "citations", citation)?;
    project.interfaces = many(doc, "", "interfaces", interface)?;
    project.tables = many(doc, "", "tables", table)?;
    project.waivers = many(doc, "", "waivers", waiver)?;
    project.frames = many(doc, "", "frames", frame)?;
    project.instances = many(doc, "", "instances", instance)?;
    project.digest = digest(doc, "digest");
    debug!(
        components = project.components.len(),
        solvers = project.solvers.len(),
        tables = project.tables.len(),
        checks = project.checks.len(),
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

    // frob:tests decode_roundtrips_every_top_level_entity
    #[test]
    fn decode_roundtrips_every_top_level_entity() {
        let dim = cadetta_units::Dim::new(cadetta_units::Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0);
        let mag = Magnitude::new(Rational::new(25, 1000).unwrap(), 0);
        let cell = serde_json::to_value(Value::new(Nominal::Exact(Exact(mag)), dim, "25 mm")).unwrap();
        let digest = format!("sha256:{}", "ab".repeat(32));
        let doc = serde_json::json!({
            "package": {"name": "acme.rover", "version": "1.0.0"},
            "flavour": "design",
            "dependencies": [{"name": "ots", "version": "1.0.0", "source": "registry", "digest": digest}],
            "components": [{
                "path": "acme.rover::Rail", "class": "model",
                "datums": [
                    {"path": "acme.rover::Rail.site", "parent": "acme.rover::Rail", "ops": [
                        {"offset": {"x": "acme.rover::Rail.dx"}}, {"rotate": {"axis": "z", "angle": "acme.rover::Rail.a"}}, {"flip": "y"}
                    ], "variants": {"acme.rover::Rail/mount=acme.rover::Bolt": [{"flip": "x"}]}, "digest": digest, "closure": digest}
                ],
                "points": [{"path": "acme.rover::Rail.p", "datum": "acme.rover::Rail.site", "coords": [cell.clone()]}],
                "placements": [{"path": "acme.rover::Rail.pl", "part": "acme.rover::Rail.bolt", "datum": "acme.rover::Rail.site", "at": "acme.rover::Rail.p",
                                "ops": [{"flip": "z"}]}]
            }],
            "checks": [{"path": "acme.rover::Rail.ok", "expr": "a < b", "conservative": true,
                        "verdicts": {"primary": "pass", "per_alternate": {"acme.rover::Rail/mount=acme.rover::Bolt": "unknown"}},
                        "waived_for": ["acme.rover::Rail/mount=acme.rover::Bolt"]}],
            "bindings": [{"path": "acme.rover::Rail.x", "adapter": "sw", "locators": [
                {"locator": "D1@Sketch", "kind": "dimension", "mode": "ref", "native_unit": "mm",
                 "ref_to": "acme.rover::Rail.len", "sign": -1, "implicit": "link", "tolerance": "drive"}
            ]}],
            "interfaces": [{"path": "acme.rover::Rail", "members": [{"name": "w", "type": "Length", "optional": true}],
                            "bounds": [], "datums": ["site"]}],
            "tables": [{"path": "acme.rover::Rail", "key": {"name": "k", "type": "Length"},
                        "cols": [{"name": "c", "type": "Length"}],
                        "rows": [{"key": cell.clone(), "cells": [cell, {"variant": "Bolt"}]}]}],
            "waivers": [{"code": "D001", "glob": "acme.rover::*", "because": "ok", "expires": "2030-01-01",
                         "for_alt": "acme.rover::Rail/mount=acme.rover::Bolt", "targets_closure": {"acme.rover::Rail": digest}}]
        });
        let decoded = decode_project(&doc).unwrap();
        assert_eq!(decoded.dependencies.len(), 1);
        assert_eq!(decoded.checks.len(), 1);
        assert_eq!(decoded.bindings.len(), 1);
        assert_eq!(decoded.interfaces.len(), 1);
        assert_eq!(decoded.tables.len(), 1);
        assert_eq!(decoded.waivers.len(), 1);
        let c = &decoded.components[0];
        assert_eq!((c.datums.len(), c.points.len(), c.placements.len()), (1, 1, 1));
        assert_eq!(c.datums[0].ops.len(), 3);
        // Re-serializing the decoded model must reproduce every decoded entity.
        let back = serde_json::to_value(&decoded).unwrap();
        for key in ["dependencies", "checks", "bindings", "interfaces", "tables", "waivers"] {
            assert_eq!(back[key], doc[key], "{key}");
        }
        assert_eq!(back["components"][0]["datums"], doc["components"][0]["datums"]);
        assert_eq!(back["components"][0]["points"], doc["components"][0]["points"]);
        assert_eq!(back["components"][0]["placements"], doc["components"][0]["placements"]);
    }

    // frob:tests decode_rejects_a_bad_locator_kind
    #[test]
    fn decode_rejects_a_bad_locator_kind() {
        let doc = serde_json::json!({
            "package": {"name": "a", "version": "1"}, "flavour": "design",
            "bindings": [{"path": "a::C.x", "adapter": "sw", "locators": [
                {"locator": "x", "kind": "bogus", "mode": "value", "native_unit": "mm"}]}]
        });
        let err = decode_project(&doc).unwrap_err();
        assert!(err.starts_with("/bindings/0/locators/0:"), "{err}");
    }
}
