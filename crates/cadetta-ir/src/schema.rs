//! The `cadetta.ir/1` JSON Schema, generated from one definition table and committed.
//!
//! Specification: `docs/spec/ir.md` 3 and 5. The IR types encode themselves by hand (the
//! `json` and `entities` modules), so the schema cannot be derived from the types; instead
//! [`schema_value`] is the single table of wire shapes, [`schema_json`] renders it
//! deterministically (sorted keys, two-space indent, trailing newline) and
//! `docs/schemas/cadetta.ir.1.json` is its committed output. Tests regenerate the schema and
//! compare it to the committed file, and validate a fully populated `Project` plus every item
//! entity against it with `additionalProperties: false`, so a field added to a serializer
//! without a schema entry (or the reverse) fails a test.
//!
//! Decisions made here (written into ir.md 5): the schema root describes `Project`; the item
//! entities `Interface`, `Table`, `Waiver` and `Process` and the geometry-free entity shapes are
//! `$defs` so a consumer can generate models for them even before `Project` carries them;
//! [`classify`] is the compatibility rule: adding an optional property (or a new `$defs` entry)
//! is [`Change::Minor`], any other difference is [`Change::Breaking`] and bumps `cadetta.ir/N`.
// frob:ticket 01M4SPWQANZ48Y8HG2X84RNK2D

use serde_json::{Map, Value as J, json};
use tracing::{debug, info};

use crate::model::SCHEMA;

/// Compatibility class of a schema change (ir.md 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    /// The schemas are identical.
    None,
    /// Only optional properties or new `$defs` were added: the schema number stays.
    Minor,
    /// Anything else: bump `cadetta.ir/N`.
    Breaking,
}

/// A `$ref` to a named definition.
fn r(name: &str) -> J {
    json!({ "$ref": format!("#/$defs/{name}") })
}

/// A plain string.
fn s() -> J {
    json!({ "type": "string" })
}

/// A boolean.
fn b() -> J {
    json!({ "type": "boolean" })
}

/// An integer.
fn i() -> J {
    json!({ "type": "integer" })
}

/// An array of `item`.
fn arr(item: J) -> J {
    json!({ "type": "array", "items": item })
}

/// A fixed-length array of exactly `n` items.
// frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
fn arr_n(item: J, n: usize) -> J {
    json!({ "type": "array", "items": item, "minItems": n, "maxItems": n })
}

/// A 3x4 transform: three rows of rotation columns then translation (ir.md 3).
// frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
fn transform34() -> J {
    arr_n(arr_n(r("Value"), 4), 3)
}

/// A string-keyed map of `item`.
fn map(item: J) -> J {
    json!({ "type": "object", "additionalProperties": item })
}

/// A string enum.
fn en(desc: &str, vals: &[&str]) -> J {
    json!({ "description": desc, "type": "string", "enum": vals })
}

/// A closed object with required and optional properties.
fn obj(desc: &str, req: &[(&str, J)], opt: &[(&str, J)]) -> J {
    let mut props = Map::new();
    for (k, v) in req.iter().chain(opt.iter()) {
        props.insert((*k).to_owned(), v.clone());
    }
    let mut required: Vec<&str> = req.iter().map(|(k, _)| *k).collect();
    required.sort_unstable();
    json!({
        "description": desc,
        "type": "object",
        "properties": props,
        "required": required,
        "additionalProperties": false,
    })
}

/// The `$defs` table: one entry per wire shape of ir.md 3.
fn defs() -> Map<String, J> {
    let mut d: Vec<(&str, J)> = Vec::new();
    d.push((
        "Path",
        json!({"description": "Canonical member or instance path text.", "type": "string"}),
    ));
    d.push((
        "ComponentPath",
        json!({"description": "Canonical item path text (no member tail).", "type": "string"}),
    ));
    d.push((
        "VariantName",
        json!({"description": "Full-form `SLOT=ALT` key text.", "type": "string"}),
    ));
    d.push(("Digest", json!({"description": "BLAKE3 digest, lowercase hex.", "type": "string", "pattern": "^[0-9a-f]+$"})));
    d.push((
        "Flavour",
        en("Which producer built the IR.", &["design", "observed"]),
    ));
    d.push((
        "SpanRef",
        obj(
            "A source location.",
            &[("file", s()), ("start", i()), ("end", i())],
            &[],
        ),
    ));
    d.push(("Exact", obj(
        "An exact magnitude `r * pi^pi` in SI base units; `r` is a reduced `num/den` with den > 0.",
        &[("r", json!({"type": "string", "pattern": "^-?[0-9]+/[1-9][0-9]*$"})), ("pi", i())],
        &[],
    )));
    d.push((
        "Inexact",
        obj(
            "A float result of an inexact operation; the only float in the IR.",
            &[("f64", json!({"type": "number"})), ("root", r("Path"))],
            &[],
        ),
    ));
    d.push(("Nominal", json!({"description": "Exact unless an inexact operation produced it.", "oneOf": [r("Exact"), r("Inexact")]})));
    d.push((
        "AffineTerm",
        obj(
            "One tolerance source variable.",
            &[("var", r("Path")), ("lo", r("Exact")), ("hi", r("Exact"))],
            &[],
        ),
    ));
    d.push((
        "Affine",
        obj(
            "An affine tolerance form.",
            &[("terms", arr(r("AffineTerm")))],
            &[],
        ),
    ));
    d.push(("BoundOp", en("A bound comparison.", &["<=", ">=", "in"])));
    d.push((
        "Bound",
        obj(
            "A bound member limit.",
            &[("op", r("BoundOp"))],
            &[("lo", r("Exact")), ("hi", r("Exact"))],
        ),
    ));
    d.push(("Dim", json!({
        "description": "A dimension vector: SI base exponents, angle, and a currency exponent map.",
        "type": "object",
        "properties": {
            "m": i(), "kg": i(), "s": i(), "A": i(), "K": i(), "mol": i(), "cd": i(), "angle": i(),
            "currency": map(i()),
        },
        "required": ["A", "K", "angle", "cd", "currency", "kg", "m", "mol", "s"],
        "additionalProperties": false,
    })));
    d.push((
        "Value",
        obj(
            "An evaluated quantity; `nominal` is always the designed value.",
            &[
                ("nominal", r("Nominal")),
                ("dim", r("Dim")),
                ("angle", i()),
                ("conservative", b()),
                ("exact", b()),
                ("authored", s()),
            ],
            &[
                ("tol", r("Affine")),
                ("drawn", r("Exact")),
                ("reason", s()),
                ("unit_assumed", s()),
            ],
        ),
    ));
    d.push((
        "Provenance",
        json!({
            "description": "How a member came to exist.",
            "oneOf": [
                {"type": "string", "enum": ["authored", "derived"]},
                obj("Read from a CAD tool.", &[("imported", obj(
                    "The import source.",
                    &[("adapter", s()), ("file", s()), ("native_name", s())],
                    &[],
                ))], &[]),
            ],
        }),
    ));
    d.push((
        "ComponentClass",
        en(
            "The class of a component.",
            &["ots", "model", "assembly", "process"],
        ),
    ));
    d.push((
        "MemberKind",
        en(
            "What a member declares.",
            &["param", "let", "bound", "datum", "place"],
        ),
    ));
    d.push((
        // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
        "CitationType",
        en(
            "The declared type of a `cite` item.",
            &[
                "standard",
                "datasheet",
                "drawing",
                "catalog",
                "book",
                "article",
                "web",
                "quote",
                "document",
            ],
        ),
    ));
    d.push((
        "CiteBuiltin",
        en(
            "A built-in citation form.",
            &["note", "measured", "assumed", "estimate"],
        ),
    ));
    d.push((
        "Citation",
        obj(
            "A `cite` item reached by the design; `fields` maps names to strings (`author` to a list).",
            &[
                ("path", s()),
                ("type", r("CitationType")),
                ("fields", map(json!({ "type": ["string", "array"], "items": { "type": "string" } }))),
                ("digest", r("Digest")),
            ],
            &[("file_digest", r("Digest"))],
        ),
    ));
    d.push((
        "CiteRef",
        obj(
            "A use-site citation: exactly one of `cite` (an item path) and `builtin`.",
            &[],
            &[
                ("cite", s()),
                ("builtin", r("CiteBuiltin")),
                ("text", s()),
                ("locator", s()),
            ],
        ),
    ));
    d.push((
        "Member",
        obj(
            "A parameter, let, bound or generated member; `priv` is the private flag.",
            &[
                ("path", r("Path")),
                ("kind", r("MemberKind")),
                ("analysis", b()),
                ("priv", b()),
                ("deps", arr(r("Path"))),
                ("was", arr(s())),
                ("attrs", map(s())),
                ("provenance", r("Provenance")),
            ],
            &[
                ("value", r("Value")),
                ("bound", r("Bound")),
                ("expr", s()),
                ("role", s()),
                ("citation", r("CiteRef")),
                ("doc", s()),
                ("span", r("SpanRef")),
                ("variants", map(r("Value"))),
                ("digest", r("Digest")),
                ("closure", r("Digest")),
            ],
        ),
    ));
    d.push((
        "Part",
        obj(
            "A part slot in a component.",
            &[
                ("path", r("Path")),
                ("type", s()),
                ("primary", s()),
                ("alternates", arr(s())),
                ("args", map(s())),
            ],
            &[("count", i())],
        ),
    ));
    d.push((
        "Fit",
        obj(
            "A fit between a hole and a shaft member.",
            &[
                ("path", r("Path")),
                ("hole", r("Path")),
                ("shaft", r("Path")),
                ("clearance", r("Value")),
                ("type", s()),
            ],
            &[("required", r("Bound"))],
        ),
    ));
    d.push(("Axis", en("A coordinate axis.", &["x", "y", "z"])));
    d.push((
        "Op",
        json!({
            "description": "A frame operation shared by Datum and Placement.",
            "oneOf": [
                obj(
                    "Offset by member-referenced axes.",
                    &[(
                        "offset",
                        obj(
                            "Offset arguments.",
                            &[],
                            &[("x", r("Path")), ("y", r("Path")), ("z", r("Path"))],
                        ),
                    )],
                    &[],
                ),
                obj(
                    "Rotate about an axis.",
                    &[(
                        "rotate",
                        obj(
                            "Rotate arguments.",
                            &[("axis", r("Axis")), ("angle", r("Path"))],
                            &[],
                        ),
                    )],
                    &[],
                ),
                obj(
                    "Flip about an axis (x or y only, ir.md 3).",
                    &[("flip", en("A flip axis.", &["x", "y"]))],
                    &[],
                ),
            ],
        }),
    ));
    d.push((
        "Datum",
        obj(
            "A frame built from its parent by an op chain.",
            &[
                ("path", r("Path")),
                ("parent", r("Path")),
                ("ops", arr(r("Op"))),
            ],
            &[
                ("variants", map(arr(r("Op")))),
                ("digest", r("Digest")),
                ("closure", r("Digest")),
            ],
        ),
    ));
    d.push((
        "Point",
        obj(
            "A named point on a datum.",
            &[
                ("path", r("Path")),
                ("datum", r("Path")),
                ("coords", arr(r("Value"))),
            ],
            &[("position", r("Exact"))],
        ),
    ));
    d.push((
        "Placement",
        obj(
            "A placement edge of the part tree.",
            &[
                ("path", r("Path")),
                ("part", r("Path")),
                ("datum", r("Path")),
                ("at", r("Path")),
                ("ops", arr(r("Op"))),
            ],
            &[
                ("variants", map(arr(r("Op")))),
                ("digest", r("Digest")),
                ("closure", r("Digest")),
            ],
        ),
    ));
    d.push((
        "Component",
        obj(
            "A component definition.",
            &[
                ("path", r("Path")),
                ("class", r("ComponentClass")),
                ("implements", arr(s())),
                ("attrs", map(s())),
                ("members", arr(r("Member"))),
                ("parts", arr(r("Part"))),
                ("fits", arr(r("Fit"))),
            ],
            &[
                ("datums", arr(r("Datum"))),
                ("points", arr(r("Point"))),
                ("placements", arr(r("Placement"))),
                ("params_sig", s()),
                ("citation", r("CiteRef")),
                ("span", r("SpanRef")),
                ("digest", r("Digest")),
            ],
        ),
    ));
    d.push((
        "VerdictState",
        en("A check verdict.", &["pass", "fail", "unknown"]),
    ));
    d.push((
        "Verdicts",
        obj(
            "Primary and per-alternate verdicts, keyed `SLOT=ALT`.",
            &[("per_alternate", map(r("VerdictState")))],
            &[("primary", r("VerdictState"))],
        ),
    ));
    d.push((
        "Check",
        obj(
            "A check and its verdicts.",
            &[
                ("path", r("Path")),
                ("expr", s()),
                ("verdicts", r("Verdicts")),
                ("conservative", b()),
                ("waived_for", arr(s())),
            ],
            &[],
        ),
    ));
    d.push((
        "Require",
        obj(
            "A require obligation and its verdicts; never suppressed.",
            &[
                ("path", r("Path")),
                ("expr", s()),
                ("verdicts", r("Verdicts")),
                ("conservative", b()),
            ],
            &[("message", s()), ("citation", r("CiteRef"))],
        ),
    ));
    d.push((
        "Solver",
        obj(
            "A solver script with its declared inputs and outputs.",
            &[
                ("path", r("Path")),
                ("script", s()),
                ("inputs", map(s())),
                ("outputs", map(s())),
                ("deps", arr(r("Path"))),
                ("stale", b()),
            ],
            &[("result_digest", r("Digest")), ("citation", r("CiteRef"))],
        ),
    ));
    d.push((
        "LocatorKind",
        en(
            "A native locator kind.",
            &["global", "dimension", "property", "frame", "instance"],
        ),
    ));
    d.push((
        "LocatorMode",
        en(
            "Whether a locator holds the value or links to it.",
            &["value", "ref"],
        ),
    ));
    // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
    d.push((
        "Implicit",
        en(
            "The native fact that implied a locator no bindings file lists.",
            &["global", "link", "frame", "reference"],
        ),
    ));
    d.push((
        "TolerancePolicy",
        en(
            "Whether a dimension locator's design tolerance is written (adapters.md 3.6).",
            &["drive", "ignore"],
        ),
    ));
    d.push((
        "ObservedFrame",
        obj(
            "A frame seen in a native document (observed flavour).",
            &[
                ("name", s()),
                ("document", s()),
                ("transform", transform34()),
            ],
            &[],
        ),
    ));
    d.push((
        "ObservedInstance",
        obj(
            "A component instance seen in a native document (observed flavour).",
            &[
                ("component", s()),
                ("document", s()),
                ("transform", transform34()),
            ],
            &[("reference", s())],
        ),
    ));
    d.push(("Locator", obj(
        "A native locator; `for` selects a slot binding's alternate.",
        &[("locator", s()), ("kind", r("LocatorKind")), ("mode", r("LocatorMode")), ("native_unit", s())],
        &[
            ("ref_to", s()), ("for", s()), ("config", s()),
            ("sign", json!({"description": "1, or -1 to negate a coordinate.", "type": "integer", "enum": [1, -1]})),
            ("component", s()), ("implicit", r("Implicit")),
            ("tolerance", r("TolerancePolicy")),
        ],
    )));
    d.push((
        "Binding",
        obj(
            "The adapter locators of one member.",
            &[
                ("path", r("Path")),
                ("adapter", s()),
                ("locators", arr(r("Locator"))),
            ],
            &[],
        ),
    ));
    d.push((
        "PackageId",
        obj(
            "A package name and version.",
            &[("name", s()), ("version", s())],
            &[],
        ),
    ));
    d.push((
        "Dependency",
        obj(
            "A resolved dependency.",
            &[
                ("name", s()),
                ("version", s()),
                ("source", s()),
                ("digest", s()),
            ],
            &[],
        ),
    ));
    d.push((
        "InterfaceMember",
        obj(
            "An interface member requirement.",
            &[("name", s()), ("type", s()), ("optional", b())],
            &[],
        ),
    ));
    d.push((
        "InterfaceBound",
        obj(
            "An interface bound requirement.",
            &[("name", s()), ("bound", r("Bound"))],
            &[],
        ),
    ));
    d.push((
        "Interface",
        obj(
            "An interface definition.",
            &[
                ("path", r("ComponentPath")),
                ("members", arr(r("InterfaceMember"))),
                ("bounds", arr(r("InterfaceBound"))),
                ("datums", arr(s())),
            ],
            &[],
        ),
    ));
    d.push((
        "TableCol",
        obj(
            "A table key or column.",
            &[("name", s()), ("type", s())],
            &[],
        ),
    ));
    d.push((
        "Cell",
        json!({
            "description": "A quantity, or an enum variant name.",
            "oneOf": [r("Value"), obj("An enum variant.", &[("variant", s())], &[])],
        }),
    ));
    d.push((
        "TableRow",
        obj(
            "A table row; a key-only row has no cells.",
            &[("key", r("Cell")), ("cells", arr(r("Cell")))],
            &[],
        ),
    ));
    d.push((
        "Table",
        obj(
            "A table of constants.",
            &[
                ("path", r("ComponentPath")),
                ("key", r("TableCol")),
                ("cols", arr(r("TableCol"))),
                ("rows", arr(r("TableRow"))),
            ],
            &[("citation", r("CiteRef"))],
        ),
    ));
    d.push((
        "Waiver",
        obj(
            "A suppression of one code on a path glob.",
            &[
                ("code", s()),
                ("glob", s()),
                ("because", s()),
                ("targets_closure", map(r("Digest"))),
            ],
            &[("expires", s()), ("for_alt", r("VariantName"))],
        ),
    ));
    d.push((
        "ProcessRule",
        obj(
            "One compensation rule of a process.",
            &[("role", s()), ("expr", s())],
            &[],
        ),
    ));
    d.push((
        "Process",
        obj(
            "A manufacturing process.",
            &[
                ("path", r("ComponentPath")),
                ("params", arr(r("Member"))),
                ("rules", arr(r("ProcessRule"))),
            ],
            &[],
        ),
    ));
    d.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

/// The `cadetta.ir/1` schema document: `Project` at the root, every entity under `$defs`.
pub fn schema_value() -> J {
    let mut root = obj(
        "A `cadetta.ir/1` document.",
        &[
            (
                "schema",
                json!({"description": "Always `cadetta.ir/1`.", "const": SCHEMA}),
            ),
            ("flavour", r("Flavour")),
            ("package", r("PackageId")),
            ("modules", arr(s())),
            ("dependencies", arr(r("Dependency"))),
            ("components", arr(r("Component"))),
            ("checks", arr(r("Check"))),
            ("solvers", arr(r("Solver"))),
            ("bindings", arr(r("Binding"))),
            ("citations", arr(r("Citation"))),
        ],
        &[
            ("frames", arr(r("ObservedFrame"))),
            ("instances", arr(r("ObservedInstance"))),
            ("interfaces", arr(r("Interface"))),
            ("tables", arr(r("Table"))),
            ("requires", arr(r("Require"))),
            ("waivers", arr(r("Waiver"))),
            ("digest", r("Digest")),
        ],
    );
    let o = root.as_object_mut().expect("obj builds an object");
    o.insert(
        "$schema".into(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    o.insert(
        "$id".into(),
        json!(format!("https://cadetta.dev/schemas/{SCHEMA}")),
    );
    o.insert("title".into(), json!("Project"));
    o.insert("$defs".into(), J::Object(defs()));
    debug!(
        defs = o["$defs"].as_object().map_or(0, Map::len),
        "ir schema: built"
    );
    root
}

/// The schema as committed: pretty-printed, sorted keys, trailing newline.
pub fn schema_json() -> String {
    let mut text = serde_json::to_string_pretty(&schema_value()).expect("a schema serializes");
    text.push('\n');
    text
}

/// Classify the change from `old` to `new` per ir.md 5.
pub fn classify(old: &J, new: &J) -> Change {
    let c = diff(old, new);
    info!(?c, "ir schema: classified");
    c
}

/// Recursive structural diff; the worst difference wins.
fn diff(old: &J, new: &J) -> Change {
    if old == new {
        return Change::None;
    }
    let (J::Object(a), J::Object(b)) = (old, new) else {
        return Change::Breaking;
    };
    let has_props = a.contains_key("properties") && b.contains_key("properties");
    let mut worst = if has_props {
        diff_props(a, b)
    } else {
        Change::None
    };
    for k in a.keys().chain(b.keys()) {
        if has_props && (k == "properties" || k == "required") {
            continue;
        }
        let c = match (a.get(k), b.get(k)) {
            (Some(J::Object(x)), Some(J::Object(y))) if k == "$defs" => diff_defs(x, y),
            (Some(x), Some(y)) => diff(x, y),
            _ => Change::Breaking,
        };
        worst = worst.max(c);
    }
    worst
}

/// Diff `$defs`: an added definition is minor, a removed one breaking, a changed one recurses.
fn diff_defs(a: &Map<String, J>, b: &Map<String, J>) -> Change {
    let mut worst = Change::None;
    for (k, x) in a {
        match b.get(k) {
            Some(y) => worst = worst.max(diff(x, y)),
            None => return Change::Breaking,
        }
    }
    if b.keys().any(|k| !a.contains_key(k)) {
        worst = worst.max(Change::Minor);
    }
    worst
}

/// Diff an object's `properties` and `required`: new optional properties are minor.
fn diff_props(a: &Map<String, J>, b: &Map<String, J>) -> Change {
    let (J::Object(pa), J::Object(pb)) = (&a["properties"], &b["properties"]) else {
        return Change::Breaking;
    };
    let req = |m: &Map<String, J>| -> Vec<J> {
        m.get("required")
            .and_then(J::as_array)
            .cloned()
            .unwrap_or_default()
    };
    if req(a) != req(b) {
        return Change::Breaking;
    }
    let mut worst = Change::None;
    for (k, x) in pa {
        match pb.get(k) {
            Some(y) => worst = worst.max(diff(x, y)),
            None => return Change::Breaking,
        }
    }
    if pb.keys().any(|k| !pa.contains_key(k)) {
        worst = worst.max(Change::Minor);
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{
        Cell, Interface, InterfaceBound, InterfaceMember, Process, ProcessRule, Require, Table,
        TableCol, TableRow, Waiver,
    };
    use crate::model::*;
    use crate::path::{Path, PathContext};
    use crate::variant_name::{ComponentPath, VariantName};
    use cadetta_units::{Dim, Magnitude, Rational, UnitTable};
    use std::collections::BTreeMap;

    fn p(t: &str) -> Path {
        Path::parse(t, PathContext::General).unwrap()
    }

    fn ex(n: i64, d: i64, pi: i32) -> Exact {
        Exact(Magnitude::new(Rational::new(n, d).unwrap(), pi))
    }

    fn cp(t: &str) -> ComponentPath {
        ComponentPath::parse(t).unwrap()
    }

    fn full_value() -> Value {
        let mm = UnitTable::builtin().parse("mm").unwrap();
        let mut v = Value::new(Nominal::Exact(ex(127, 5000, 0)), mm.dim, "5.08 mm");
        v.tol = Some(Affine {
            terms: vec![AffineTerm {
                var: p("a.b::C.d"),
                lo: ex(-1, 2000, 0),
                hi: ex(1, 2000, 0),
            }],
        });
        v.drawn = Some(ex(3, 1, 0));
        v.reason = Some("native rounding".into());
        v.unit_assumed = Some("mm".into());
        v
    }

    fn full_member(path: &str) -> Member {
        let mut m = Member::new(p(path), MemberKind::Param);
        m.value = Some(full_value());
        m.bound = Some(Bound {
            op: BoundOp::In,
            lo: Some(ex(0, 1, 0)),
            hi: Some(ex(1, 1, 0)),
        });
        m.expr = Some("1 mm".into());
        m.deps = vec![p("a.b::C.e")];
        m.role = Some("hole".into());
        m.citation = Some(cite());
        m.was = vec!["old".into()];
        m.attrs = BTreeMap::from([("display".to_owned(), "x".to_owned())]);
        m.provenance = Provenance::Imported {
            adapter: "freecad".into(),
            file: "a.FCStd".into(),
            native_name: "d".into(),
        };
        m.doc = Some("doc".into());
        m.span = Some(SpanRef {
            file: "a.cdt".into(),
            start: 1,
            end: 2,
        });
        m.variants = Some(BTreeMap::from([(
            "a.b::C/w=a.b::D".to_owned(),
            full_value(),
        )]));
        m.digest = Some(Digest("00ff".into()));
        m.closure = Some(Digest("11ee".into()));
        m
    }

    fn cite() -> CiteRef {
        CiteRef::item("acme.rover::refs::n", Some("p1".into()))
    }

    fn citation() -> Citation {
        let mut f = BTreeMap::new();
        f.insert(
            "author".to_owned(),
            CiteField::List(vec!["A".into(), "B".into()]),
        );
        f.insert("title".to_owned(), CiteField::Text("T".into()));
        Citation::new(
            "acme.rover::refs::n",
            CitationType::Book,
            f,
            Some(Digest("ab".into())),
        )
    }

    /// A project with every optional field and every enum arm exercised.
    fn full_project() -> Project {
        let mut pr = Project::new(
            Flavour::Observed,
            PackageId {
                name: "a.b".into(),
                version: "0.1.0".into(),
            },
        );
        pr.set_modules(vec!["a.b".into()]);
        pr.set_dependencies(vec![Dependency {
            name: "std".into(),
            version: "0.1".into(),
            source: "path".into(),
            digest: "ab".into(),
        }]);
        let mut inexact = full_member("a.b::C.f");
        inexact.value = Some(Value::new(
            Nominal::Inexact(Inexact {
                f64: 1.5,
                root: p("a.b::C.f"),
            }),
            Dim::default(),
            "sqrt(2)",
        ));
        inexact.provenance = Provenance::Derived;
        let mut plain = Member::new(p("a.b::C.g"), MemberKind::Let);
        plain.provenance = Provenance::Authored;
        pr.components.push(Component {
            path: p("a.b::C.d"),
            class: ComponentClass::Assembly,
            params_sig: Some("(d: Length)".into()),
            implements: vec!["a.b::I".into()],
            citation: Some(cite()),
            attrs: BTreeMap::new(),
            members: vec![full_member("a.b::C.d"), inexact, plain],
            parts: vec![Part {
                path: p("a.b::C/w.x"),
                type_name: "W".into(),
                count: Some(2),
                primary: "a.b::W1".into(),
                alternates: vec!["a.b::W2".into()],
                args: BTreeMap::from([("d".to_owned(), "1 mm".to_owned())]),
            }],
            fits: vec![Fit {
                path: p("a.b::C.fit"),
                hole: p("a.b::C.h"),
                shaft: p("a.b::C.s"),
                clearance: full_value(),
                fit_type: "clearance".into(),
                required: Some(Bound {
                    op: BoundOp::Ge,
                    lo: Some(ex(0, 1, 0)),
                    hi: None,
                }),
            }],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: Some(SpanRef {
                file: "a.cdt".into(),
                start: 0,
                end: 9,
            }),
            digest: Some(Digest("cd".into())),
        });
        pr.checks.push(Check {
            path: p("a.b::C.chk"),
            expr: "x <= 1".into(),
            verdicts: Verdicts {
                primary: Some(VerdictState::Pass),
                per_alternate: BTreeMap::from([(
                    "a.b::C/w=a.b::W2".to_owned(),
                    VerdictState::Unknown,
                )]),
            },
            conservative: true,
            waived_for: vec!["a.b::C/w=a.b::W2".into()],
        });
        pr.solvers.push(Solver {
            path: p("a.b::C.sol"),
            script: "s.py".into(),
            citation: Some(CiteRef::item("a.b::method", Some("p.3".into()))),
            inputs: BTreeMap::from([("i".to_owned(), "a.b::C.d".to_owned())]),
            outputs: BTreeMap::from([("o".to_owned(), "a.b::C.e".to_owned())]),
            deps: vec![p("a.b::C.d")],
            result_digest: Some(Digest("ef".into())),
            stale: true,
        });
        let loc = |kind, mode, sign| Locator {
            locator: "D1@Sketch1".into(),
            kind,
            mode,
            native_unit: "mm".into(),
            ref_to: Some("x".into()),
            for_alternate: Some("W2".into()),
            config: Some("Default".into()),
            sign: Some(sign),
            component: Some("wheel<2>".into()),
            implicit: Some(crate::model::Implicit::Link),
            tolerance: Some(crate::model::TolerancePolicy::Drive),
        };
        pr.bindings.push(Binding {
            path: p("a.b::C.d"),
            adapter: "solidworks".into(),
            locators: vec![
                loc(LocatorKind::Global, LocatorMode::Value, Sign::Plus),
                loc(LocatorKind::Dimension, LocatorMode::Ref, Sign::Minus),
                loc(LocatorKind::Property, LocatorMode::Value, Sign::Plus),
                loc(LocatorKind::Frame, LocatorMode::Value, Sign::Plus),
            ],
        });
        pr.citations = vec![citation()];
        pr.digest = Some(Digest("0123".into()));
        pr
    }

    fn resolve<'a>(root: &'a J, node: &'a J) -> &'a J {
        match node.get("$ref").and_then(J::as_str) {
            Some(rf) => &root["$defs"][rf.trim_start_matches("#/$defs/")],
            None => node,
        }
    }

    /// A minimal validator for the keyword subset this module emits.
    fn check(root: &J, node: &J, v: &J, at: &str, errs: &mut Vec<String>) {
        let node = resolve(root, node);
        if let Some(one) = node.get("oneOf").and_then(J::as_array) {
            let ok = one.iter().any(|n| {
                let mut e = vec![];
                check(root, n, v, at, &mut e);
                e.is_empty()
            });
            if !ok {
                errs.push(format!("{at}: matches no oneOf arm"));
            }
            return;
        }
        if let Some(c) = node.get("const") {
            if c != v {
                errs.push(format!("{at}: const mismatch"));
            }
            return;
        }
        if let Some(t) = node.get("type").and_then(J::as_str) {
            let ok = match t {
                "string" => v.is_string(),
                "integer" => v.is_i64() || v.is_u64(),
                "number" => v.is_number(),
                "boolean" => v.is_boolean(),
                "array" => v.is_array(),
                "object" => v.is_object(),
                _ => false,
            };
            if !ok {
                errs.push(format!("{at}: not a {t}"));
                return;
            }
        }
        if let Some(e) = node.get("enum").and_then(J::as_array)
            && !e.contains(v)
        {
            errs.push(format!("{at}: {v} not in enum"));
        }
        if let (Some(items), Some(a)) = (node.get("items"), v.as_array()) {
            for (n, x) in a.iter().enumerate() {
                check(root, items, x, &format!("{at}[{n}]"), errs);
            }
        }
        if let Some(o) = v.as_object() {
            let props = node.get("properties").and_then(J::as_object);
            for req in node
                .get("required")
                .and_then(J::as_array)
                .into_iter()
                .flatten()
            {
                let k = req.as_str().unwrap();
                if !o.contains_key(k) {
                    errs.push(format!("{at}: missing required `{k}`"));
                }
            }
            for (k, x) in o {
                let sub = props
                    .and_then(|p| p.get(k))
                    .or_else(|| node.get("additionalProperties").filter(|a| a.is_object()));
                match sub {
                    Some(sub) => check(root, sub, x, &format!("{at}.{k}"), errs),
                    None => errs.push(format!("{at}: unexpected property `{k}`")),
                }
            }
        }
    }

    fn validate(def: Option<&str>, v: &J) -> Vec<String> {
        let root = schema_value();
        let node = match def {
            Some(d) => r(d),
            None => root.clone(),
        };
        let mut errs = vec![];
        check(&root, &node, v, "$", &mut errs);
        errs
    }

    // frob:tests crates/cadetta-ir/src/schema.rs::schema_json kind=unit
    #[test]
    fn schema_file_matches_types() {
        let on_disk = include_str!("../../../docs/schemas/cadetta.ir.1.json");
        assert_eq!(
            on_disk,
            schema_json(),
            "docs/schemas/cadetta.ir.1.json is stale; replace it with schema_json()"
        );
    }

    // frob:tests crates/cadetta-ir/src/schema.rs::schema_value kind=unit
    #[test]
    fn flip_op_schema_rejects_z_accepts_x_and_y() {
        for (axis, ok) in [("x", true), ("y", true), ("z", false)] {
            let op = serde_json::json!({ "flip": axis });
            assert_eq!(validate(Some("Op"), &op).is_empty(), ok, "flip {axis}");
        }
    }

    // frob:tests crates/cadetta-ir/src/schema.rs::schema_value kind=unit
    #[test]
    fn full_project_validates_against_schema() {
        let j = serde_json::to_value(full_project()).unwrap();
        let errs = validate(None, &j);
        assert!(errs.is_empty(), "{errs:#?}");
        let bare = serde_json::to_value(Project::new(
            Flavour::Design,
            PackageId {
                name: "a".into(),
                version: "1".into(),
            },
        ))
        .unwrap();
        assert!(validate(None, &bare).is_empty());
    }

    // frob:tests crates/cadetta-ir/src/schema.rs::schema_value kind=unit
    #[test]
    fn project_accepts_interfaces_tables_and_waivers() {
        let mut j = serde_json::to_value(full_project()).unwrap();
        j["interfaces"] = json!([{"path": "a.b::I", "members": [], "bounds": [], "datums": []}]);
        j["waivers"] =
            json!([{"code": "R001", "glob": "a::*", "because": "x", "targets_closure": {}}]);
        j["tables"] = json!([{"bogus": 1}]);
        let errs = validate(None, &j);
        assert!(
            errs.iter().all(|e| e.contains("tables")),
            "interfaces and waivers must validate: {errs:#?}"
        );
        assert!(!errs.is_empty(), "a malformed table must be rejected");
        j["tables"] = json!([]);
        assert_eq!(validate(None, &j), Vec::<String>::new());
    }

    // frob:tests crates/cadetta-ir/src/json.rs::serialize kind=unit
    // frob:ticket 01M4YKQSGPM7JTKPRTZXSHCDWP
    #[test]
    fn project_carries_and_serializes_item_collections() {
        let mut pr = full_project();
        let bare = serde_json::to_value(&pr).unwrap();
        for k in ["interfaces", "tables", "waivers"] {
            assert!(bare.get(k).is_none(), "empty {k} must be omitted");
        }
        pr.interfaces.push(Interface {
            path: cp("a.b::I"),
            members: vec![],
            bounds: vec![],
            datums: vec![],
        });
        pr.tables.push(Table {
            path: cp("a.b::T"),
            key: TableCol {
                name: "size".into(),
                type_name: "Size".into(),
            },
            cols: vec![],
            rows: vec![],
            citation: None,
        });
        pr.waivers.push(Waiver::new("R001", "a.b::*", "because"));
        let j = serde_json::to_value(&pr).unwrap();
        assert_eq!(j["interfaces"][0]["path"], "a.b::I");
        assert_eq!(j["tables"][0]["path"], "a.b::T");
        assert_eq!(j["waivers"][0]["code"], "R001");
        assert_eq!(validate(None, &j), Vec::<String>::new());
    }

    // frob:tests crates/cadetta-ir/src/schema.rs::schema_value kind=unit
    // frob:ticket 01M4YKTX2YMSXVE4CK0JAVMC41
    #[test]
    fn project_carries_and_validates_requires() {
        let mut pr = full_project();
        let bare = serde_json::to_value(&pr).unwrap();
        assert!(bare.get("requires").is_none(), "empty requires omitted");
        let mut r = Require::new(p("a.b::C.budget"), "mass <= 6kg");
        r.message = Some("under budget".into());
        r.citation = Some(CiteRef::item("a.b::spec", Some("REQ-12".into())));
        r.verdicts.primary = Some(VerdictState::Pass);
        pr.requires.push(r.clone());
        let j = serde_json::to_value(&pr).unwrap();
        assert_eq!(j["requires"][0]["path"], "a.b::C.budget");
        assert_eq!(j["requires"][0]["message"], "under budget");
        assert!(j["requires"][0].get("waived_for").is_none());
        assert_eq!(validate(None, &j), Vec::<String>::new());
        let back = crate::decode::decode_project(&j).unwrap();
        assert_eq!(back.requires, vec![r]);
    }

    // frob:tests crates/cadetta-ir/src/schema.rs::schema_value kind=unit
    #[test]
    fn schema_rejects_unknown_and_missing_fields() {
        let mut j = serde_json::to_value(full_project()).unwrap();
        j["extra"] = json!(1);
        assert!(
            validate(None, &j)
                .iter()
                .any(|e| e.contains("unexpected property `extra`"))
        );
        let mut j = serde_json::to_value(full_project()).unwrap();
        j.as_object_mut().unwrap().remove("flavour");
        assert!(
            validate(None, &j)
                .iter()
                .any(|e| e.contains("missing required `flavour`"))
        );
    }

    // frob:tests crates/cadetta-ir/src/schema.rs::schema_value kind=unit
    #[test]
    fn item_entities_validate_against_defs() {
        let iface = Interface {
            path: cp("a.b::I"),
            members: vec![InterfaceMember {
                name: "d".into(),
                type_name: "Length".into(),
                optional: true,
            }],
            bounds: vec![InterfaceBound {
                name: "d".into(),
                bound: Bound {
                    op: BoundOp::Le,
                    lo: None,
                    hi: Some(ex(1, 1, 0)),
                },
            }],
            datums: vec!["origin".into()],
        };
        assert_eq!(
            validate(Some("Interface"), &serde_json::to_value(&iface).unwrap()),
            Vec::<String>::new()
        );
        let table = Table {
            path: cp("a.b::T"),
            key: TableCol {
                name: "size".into(),
                type_name: "Size".into(),
            },
            cols: vec![TableCol {
                name: "d".into(),
                type_name: "Length".into(),
            }],
            rows: vec![
                TableRow {
                    key: Cell::Variant("M5".into()),
                    cells: vec![Cell::Value(Box::new(full_value()))],
                },
                TableRow {
                    key: Cell::Variant("M6".into()),
                    cells: vec![],
                },
            ],
            citation: Some(cite()),
        };
        assert_eq!(
            validate(Some("Table"), &serde_json::to_value(&table).unwrap()),
            Vec::<String>::new()
        );
        let mut w = Waiver::new("R001", "a.b::*", "because");
        w.expires = Some("2027-01-01".into());
        w.for_alt = Some(VariantName::parse("a.b::C/w=a.b::W2").unwrap());
        w.targets_closure = BTreeMap::from([("a.b::C.d".to_owned(), Digest("ab".into()))]);
        assert_eq!(
            validate(Some("Waiver"), &serde_json::to_value(&w).unwrap()),
            Vec::<String>::new()
        );
        let proc = Process {
            path: cp("a.b::P"),
            params: vec![full_member("a.b::P.k")],
            rules: vec![ProcessRule {
                role: "hole".into(),
                expr: "x + 1 mm".into(),
            }],
        };
        assert_eq!(
            validate(Some("Process"), &serde_json::to_value(&proc).unwrap()),
            Vec::<String>::new()
        );
        let ops = vec![
            crate::geometry::Op::Offset(Box::new(crate::geometry::OffsetArgs {
                x: Some(p("a.b::C.d.offset.x")),
                ..Default::default()
            })),
            crate::geometry::Op::Rotate {
                axis: crate::geometry::Axis::Z,
                angle: p("a.b::C.d.rotate.z"),
            },
            crate::geometry::Op::Flip {
                axis: crate::geometry::Axis::X,
            },
        ];
        let mut datum = crate::geometry::Datum::new(p("a.b::C.d"), p("a.b::C.origin"), ops.clone());
        datum.variants = Some(BTreeMap::from([(
            "a.b::C/w=a.b::W2".to_owned(),
            ops.clone(),
        )]));
        datum.digest = Some(Digest("ab".into()));
        assert_eq!(
            validate(Some("Datum"), &serde_json::to_value(&datum).unwrap()),
            Vec::<String>::new()
        );
        let point = crate::geometry::Point {
            path: p("a.b::C.pt"),
            datum: p("a.b::C.d"),
            coords: vec![full_value()],
            position: Some(ex(1, 10, 0)),
        };
        assert_eq!(
            validate(Some("Point"), &serde_json::to_value(&point).unwrap()),
            Vec::<String>::new()
        );
        let pl = crate::geometry::Placement {
            path: p("a.b::A/br.place"),
            part: p("a.b::A/br.x"),
            datum: p("a.b::B.base"),
            at: p("a.b::C.d"),
            ops,
            variants: None,
            digest: None,
            closure: None,
        };
        assert_eq!(
            validate(Some("Placement"), &serde_json::to_value(&pl).unwrap()),
            Vec::<String>::new()
        );
        let dim = serde_json::to_value(crate::dim::DimWire::default()).unwrap();
        assert_eq!(validate(Some("Dim"), &dim), Vec::<String>::new());
    }

    // frob:tests crates/cadetta-ir/src/schema.rs::classify kind=unit
    #[test]
    fn adding_optional_field_is_minor() {
        let old = schema_value();
        let mut new = old.clone();
        new["$defs"]["Member"]["properties"]["note"] = json!({"type": "string"});
        assert_eq!(classify(&old, &old), Change::None);
        assert_eq!(classify(&old, &new), Change::Minor);
        let mut with_def = old.clone();
        with_def["$defs"]["Extra"] = json!({"type": "string"});
        assert_eq!(classify(&old, &with_def), Change::Minor);
    }

    // frob:tests crates/cadetta-ir/src/schema.rs::classify kind=unit
    #[test]
    fn anything_else_is_breaking() {
        let old = schema_value();
        let mut req = old.clone();
        req["$defs"]["Member"]["properties"]["note"] = json!({"type": "string"});
        req["$defs"]["Member"]["required"]
            .as_array_mut()
            .unwrap()
            .push(json!("note"));
        assert_eq!(classify(&old, &req), Change::Breaking);
        let mut gone = old.clone();
        gone["$defs"]["Member"]["properties"]
            .as_object_mut()
            .unwrap()
            .remove("doc");
        assert_eq!(classify(&old, &gone), Change::Breaking);
        let mut retyped = old.clone();
        retyped["$defs"]["Member"]["properties"]["doc"] = json!({"type": "integer"});
        assert_eq!(classify(&old, &retyped), Change::Breaking);
        let mut removed_def = old.clone();
        removed_def["$defs"]
            .as_object_mut()
            .unwrap()
            .remove("Binding");
        assert_eq!(classify(&old, &removed_def), Change::Breaking);
        let mut enum_widened = old.clone();
        enum_widened["$defs"]["BoundOp"]["enum"]
            .as_array_mut()
            .unwrap()
            .push(json!("=="));
        assert_eq!(classify(&old, &enum_widened), Change::Breaking);
    }
}
