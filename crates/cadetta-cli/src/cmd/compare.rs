//! frob:ticket 01M4ST8FCNN3GZKZNRK2X3Q55S
//! `cadetta compare IFACE`: every implementer of an interface side by side, across every slot
//! typed by it (obligations.md 5, 5.1).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use cadetta_ir::model::{Bound, BoundOp, Check, Component, Exact, Project as Ir, VerdictState};
use serde_json::{Value as Json, json};

use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::Project;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Schema tag of the JSON report; the schema is `docs/schemas/compare.json`.
pub const SCHEMA: &str = "cadetta.compare/1";

const USAGE: &str = "usage: cadetta compare IFACE";

/// The members of `std::commercial::Sourced` (stdlib.md 6): the commercial columns.
pub const COMMERCIAL: [&str; 5] = ["unit_cost", "lead_time", "moq", "supplier", "mass"];

/// One member of an implementer: its value or its bound, as authored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// Member name (last path segment).
    pub name: String,
    /// The authored value, when the member has one.
    pub value: Option<String>,
    /// The bound as text (`<= 3`, `in [1, 2]`), when the member is a bound.
    pub bound: Option<String>,
}

/// A check's verdict for one implementer in one slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckVerdict {
    /// The slot the verdict is for.
    pub slot: String,
    /// The check's canonical path.
    pub check: String,
    /// `pass`, `fail`, `unknown`, or `not evaluated`.
    pub verdict: &'static str,
}

/// One implementer: a column of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// Canonical component path.
    pub component: String,
    /// Interface members and bounds (not commercial), sorted by name.
    pub members: Vec<Cell>,
    /// Commercial members (`std::commercial::Sourced`); absent ones have no cell.
    pub commercial: Vec<Cell>,
    /// The `lifecycle` attribute text, when declared.
    pub lifecycle: Option<String>,
    /// Slots (of those typed by the interface) where this is primary or approved alternate.
    pub slots: Vec<String>,
    /// Verdicts per slot and check, sorted by slot then check.
    pub verdicts: Vec<CheckVerdict>,
}

/// The comparison of one interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// The interface asked about.
    pub iface: String,
    /// Every slot typed by the interface, sorted.
    pub slots: Vec<String>,
    /// One column per implementer, sorted by component path.
    pub columns: Vec<Column>,
}

/// `IFACE` is neither implemented by a component nor the type of a slot.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("`{0}` is not an interface of this design")]
pub struct NotFound(pub String);

/// The component identity of path text: everything before the first `.` or `/` of the tail.
fn component_of(text: &str) -> &str {
    let tail_at = text.rfind("::").map_or(0, |i| i + 2);
    let end = text[tail_at..]
        .find(['.', '/'])
        .map_or(text.len(), |i| tail_at + i);
    &text[..end]
}

/// True when `name` denotes the component or interface `want` (full path or bare item name).
fn names(name: &str, want: &str) -> bool {
    let (name, want) = (component_of(name), component_of(want));
    name == want || name.rsplit("::").next() == Some(want)
}

fn exact_text(e: &Exact) -> String {
    let m = &e.0;
    match m.pi_exp() {
        0 => m.rational().to_string(),
        k => format!("{} pi^{k}", m.rational()),
    }
}

fn bound_text(b: &Bound) -> String {
    let lim = |x: &Option<Exact>| x.as_ref().map(exact_text).unwrap_or_default();
    match b.op {
        BoundOp::Le => format!("<= {}", lim(&b.hi)),
        BoundOp::Ge => format!(">= {}", lim(&b.lo)),
        BoundOp::In => format!("in [{}, {}]", lim(&b.lo), lim(&b.hi)),
    }
}

fn verdict_word(v: Option<VerdictState>) -> &'static str {
    match v {
        Some(VerdictState::Pass) => "pass",
        Some(VerdictState::Fail) => "fail",
        Some(VerdictState::Unknown) => "unknown",
        None => "not evaluated",
    }
}

/// The verdict of `check` for `comp` in `slot`: the primary verdict when `comp` is the slot's
/// primary, else the per-alternate entry `SLOT=ALT`.
fn verdict_for(check: &Check, slot: &str, is_primary: bool, comp: &str) -> &'static str {
    if is_primary {
        return verdict_word(check.verdicts.primary);
    }
    let found = check.verdicts.per_alternate.iter().find(|(k, _)| {
        k.split_once('=')
            .is_some_and(|(s, a)| s == slot && names(comp, a))
    });
    verdict_word(found.map(|(_, v)| *v))
}

fn column(ir: &Ir, c: &Component, slots: &[(&str, &cadetta_ir::model::Part)]) -> Column {
    let comp = component_of(&c.path.to_string()).to_owned();
    let mut members = Vec::new();
    let mut commercial = Vec::new();
    for m in c.members.iter().filter(|m| !m.private && !m.analysis) {
        let mp = m.path.to_string();
        let name = mp.rsplit('.').next().unwrap_or(&mp).to_owned();
        let cell = Cell {
            name: name.clone(),
            value: m.value.as_ref().map(|v| v.authored.clone()),
            bound: m.bound.as_ref().map(bound_text),
        };
        if COMMERCIAL.contains(&name.as_str()) {
            commercial.push(cell);
        } else {
            members.push(cell);
        }
    }
    members.sort_by(|a, b| a.name.cmp(&b.name));
    commercial.sort_by(|a, b| a.name.cmp(&b.name));
    let mut my_slots = Vec::new();
    let mut verdicts = Vec::new();
    for (slot, part) in slots {
        let primary = names(&comp, &part.primary);
        if !primary && !part.alternates.iter().any(|a| names(&comp, a)) {
            continue;
        }
        my_slots.push((*slot).to_owned());
        for k in &ir.checks {
            let v = verdict_for(k, slot, primary, &comp);
            verdicts.push(CheckVerdict {
                slot: (*slot).to_owned(),
                check: k.path.to_string(),
                verdict: v,
            });
        }
    }
    verdicts.sort_by(|a, b| (&a.slot, &a.check).cmp(&(&b.slot, &b.check)));
    tracing::debug!(component = %comp, slots = my_slots.len(), "compare: column");
    Column {
        component: comp,
        members,
        commercial,
        lifecycle: c.attrs.get("lifecycle").cloned(),
        slots: my_slots,
        verdicts,
    }
}

/// Collect every implementer of `iface` and every slot typed by it.
pub fn compare(ir: &Ir, iface: &str) -> Result<Comparison, NotFound> {
    let slot_parts: Vec<(String, &cadetta_ir::model::Part)> = ir
        .components
        .iter()
        .flat_map(|c| &c.parts)
        .filter(|p| names(&p.type_name, iface))
        .map(|p| (p.path.to_string(), p))
        .collect();
    let implementers: Vec<&Component> = ir
        .components
        .iter()
        .filter(|c| c.implements.iter().any(|i| names(i, iface)))
        .collect();
    if slot_parts.is_empty() && implementers.is_empty() {
        tracing::warn!(iface, "compare: no implementer or slot");
        return Err(NotFound(iface.to_owned()));
    }
    let slots: Vec<(&str, &cadetta_ir::model::Part)> =
        slot_parts.iter().map(|(s, p)| (s.as_str(), *p)).collect();
    let mut columns: Vec<Column> = implementers.iter().map(|c| column(ir, c, &slots)).collect();
    columns.sort_by(|a, b| a.component.cmp(&b.component));
    let slot_names: BTreeSet<String> = slot_parts.iter().map(|(s, _)| s.clone()).collect();
    tracing::debug!(
        iface,
        columns = columns.len(),
        slots = slot_names.len(),
        "compare: done"
    );
    Ok(Comparison {
        iface: iface.to_owned(),
        slots: slot_names.into_iter().collect(),
        columns,
    })
}

fn cell_text(c: &Cell) -> String {
    match (&c.value, &c.bound) {
        (Some(v), Some(b)) => format!("{v} ({b})"),
        (Some(v), None) => v.clone(),
        (None, Some(b)) => b.clone(),
        (None, None) => "-".to_owned(),
    }
}

fn cell_json(c: &Cell) -> Json {
    json!({"name": c.name, "value": c.value, "bound": c.bound})
}

/// Render the comparison as text or the `cadetta.compare/1` JSON document.
pub fn render(cmp: &Comparison, format: Format, out: &mut dyn Write) {
    if format == Format::Json {
        let doc = json!({
            "schema": SCHEMA,
            "iface": cmp.iface,
            "slots": cmp.slots,
            "columns": cmp.columns.iter().map(|c| json!({
                "component": c.component,
                "members": c.members.iter().map(cell_json).collect::<Vec<Json>>(),
                "commercial": c.commercial.iter().map(cell_json).collect::<Vec<Json>>(),
                "lifecycle": c.lifecycle,
                "slots": c.slots,
                "verdicts": c.verdicts.iter().map(|v| json!({
                    "slot": v.slot, "check": v.check, "verdict": v.verdict,
                })).collect::<Vec<Json>>(),
            })).collect::<Vec<Json>>(),
        });
        let _ = writeln!(out, "{doc}");
        return;
    }
    let _ = writeln!(
        out,
        "{}: {} implementers, {} slots",
        cmp.iface,
        cmp.columns.len(),
        cmp.slots.len()
    );
    for s in &cmp.slots {
        let _ = writeln!(out, "  slot {s}");
    }
    for c in &cmp.columns {
        let _ = writeln!(out, "{}", c.component);
        let _ = writeln!(
            out,
            "  lifecycle: {}",
            c.lifecycle.as_deref().unwrap_or("-")
        );
        let _ = writeln!(
            out,
            "  slots: {}",
            if c.slots.is_empty() {
                "-".into()
            } else {
                c.slots.join(", ")
            }
        );
        for m in &c.members {
            let _ = writeln!(out, "  {}: {}", m.name, cell_text(m));
        }
        let have: BTreeMap<&str, &Cell> =
            c.commercial.iter().map(|m| (m.name.as_str(), m)).collect();
        for name in COMMERCIAL {
            let text = have.get(name).map_or("-".to_owned(), |m| cell_text(m));
            let _ = writeln!(out, "  {name}: {text}");
        }
        for v in &c.verdicts {
            let _ = writeln!(out, "  check {} @ {}: {}", v.check, v.slot, v.verdict);
        }
    }
}

/// Run `cadetta compare`; `args` are the words after the verb. Returns the exit code.
pub fn run_compare(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "compare failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let [iface] = args else {
        return fail(err, USAGE);
    };
    if iface.starts_with("--") {
        return fail(err, &format!("unknown flag `{iface}`; {USAGE}"));
    }
    let (project, members) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref())
    {
        Ok(o) => o,
        Err(e) => return fail(err, &e.to_string()),
    };
    let analysis = analyze(&project, &members);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    for package in &analysis.packages {
        if let Ok(cmp) = compare(&package.ir, iface) {
            render(&cmp, globals.format, out);
            return EXIT_OK;
        }
    }
    fail(err, &NotFound(iface.clone()).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{
        ComponentClass, Flavour, Member, MemberKind, PackageId, Part, Value, Verdicts,
    };
    use cadetta_ir::path::{Path, PathContext};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn comp(path: &str, iface: &str, members: &[(&str, &str)], life: Option<&str>) -> Component {
        let mut c = Component {
            path: p(path),
            class: ComponentClass::Ots,
            params_sig: None,
            implements: vec![iface.into()],
            citation: None,
            attrs: Default::default(),
            members: vec![],
            parts: vec![],
            fits: vec![],
            span: None,
            digest: None,
        };
        for (n, v) in members {
            let mut m = Member::new(p(&format!("{}.{n}", component_of(path))), MemberKind::Param);
            m.value = Some(Value::new(
                cadetta_ir::model::Nominal::Exact(Exact(cadetta_units::Magnitude::new(
                    cadetta_units::Rational::new(1, 1).unwrap(),
                    0,
                ))),
                cadetta_units::Dim::default(),
                *v,
            ));
            c.members.push(m);
        }
        if let Some(l) = life {
            c.attrs.insert("lifecycle".into(), l.into());
        }
        c
    }

    fn part(path: &str, ty: &str, primary: &str, alts: &[&str]) -> Part {
        Part {
            path: p(path),
            type_name: ty.into(),
            count: None,
            primary: primary.into(),
            alternates: alts.iter().map(|s| (*s).into()).collect(),
            args: Default::default(),
        }
    }

    fn ir() -> Ir {
        let mut ir = Ir::new(
            Flavour::Design,
            PackageId {
                name: "pkg".into(),
                version: "0.1.0".into(),
            },
        );
        ir.components.push(comp(
            "pkg::Colson.x",
            "pkg::Caster",
            &[("bore", "8mm"), ("unit_cost", "12 USD")],
            Some("active, 2030-01"),
        ));
        ir.components.push(comp(
            "pkg::Hamilton.x",
            "pkg::Caster",
            &[("bore", "10mm")],
            None,
        ));
        ir.components
            .push(comp("pkg::Other.x", "pkg::Wheel", &[("bore", "1mm")], None));
        let mut rover = comp("pkg::Rover.x", "pkg::Asm", &[], None);
        rover.parts.push(part(
            "pkg::Rover.front",
            "pkg::Caster",
            "pkg::Colson",
            &["pkg::Hamilton"],
        ));
        rover
            .parts
            .push(part("pkg::Rover.rear", "pkg::Caster", "pkg::Hamilton", &[]));
        ir.components.push(rover);
        let mut v = Verdicts {
            primary: Some(VerdictState::Pass),
            ..Default::default()
        };
        v.per_alternate
            .insert("pkg::Rover.front=Hamilton".into(), VerdictState::Fail);
        ir.checks.push(Check {
            path: p("pkg::Rover.clear"),
            expr: "x".into(),
            verdicts: v,
            conservative: false,
            waived_for: vec![],
        });
        ir
    }

    // frob:tests crates/cadetta-cli/src/cmd/compare.rs::compare
    #[test]
    fn every_implementer_is_a_column_with_members_commercial_lifecycle_and_verdicts() {
        let c = compare(&ir(), "Caster").unwrap();
        let names: Vec<_> = c.columns.iter().map(|x| x.component.as_str()).collect();
        assert_eq!(names, ["pkg::Colson", "pkg::Hamilton"]);
        let colson = &c.columns[0];
        assert_eq!(colson.members.len(), 1);
        assert_eq!(colson.commercial[0].name, "unit_cost");
        assert_eq!(colson.lifecycle.as_deref(), Some("active, 2030-01"));
        let hamilton = &c.columns[1];
        assert_eq!(hamilton.lifecycle, None);
        let verdict = |col: &Column, slot: &str| {
            col.verdicts
                .iter()
                .find(|v| v.slot == slot)
                .unwrap()
                .verdict
        };
        assert_eq!(verdict(colson, "pkg::Rover.front"), "pass");
        assert_eq!(verdict(hamilton, "pkg::Rover.front"), "fail");
        assert_eq!(verdict(hamilton, "pkg::Rover.rear"), "pass");
    }

    // frob:tests crates/cadetta-cli/src/cmd/compare.rs::compare
    #[test]
    fn table_covers_every_slot_typed_by_the_interface() {
        let c = compare(&ir(), "pkg::Caster").unwrap();
        assert_eq!(c.slots, ["pkg::Rover.front", "pkg::Rover.rear"]);
        assert_eq!(c.columns[1].slots, ["pkg::Rover.front", "pkg::Rover.rear"]);
        assert_eq!(c.columns[0].slots, ["pkg::Rover.front"]);
    }

    // frob:tests crates/cadetta-cli/src/cmd/compare.rs::compare
    #[test]
    fn unknown_interface_is_not_found() {
        assert_eq!(compare(&ir(), "Nope"), Err(NotFound("Nope".into())));
    }

    // frob:tests crates/cadetta-cli/src/cmd/compare.rs::render
    #[test]
    fn text_lists_all_commercial_names() {
        let mut out = Vec::new();
        render(&compare(&ir(), "Caster").unwrap(), Format::Text, &mut out);
        let s = String::from_utf8(out).unwrap();
        for n in COMMERCIAL {
            assert!(s.contains(&format!("  {n}: ")), "{s}");
        }
        assert!(s.contains("lifecycle: active, 2030-01"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/compare.rs::render
    #[test]
    fn json_carries_schema_tag() {
        let mut out = Vec::new();
        render(&compare(&ir(), "Caster").unwrap(), Format::Json, &mut out);
        let v: Json = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["schema"], SCHEMA);
        assert_eq!(v["columns"].as_array().unwrap().len(), 2);
    }

    // frob:tests crates/cadetta-cli/src/cmd/compare.rs::run_compare
    #[test]
    fn usage_errors_exit_two() {
        let g = Globals {
            format: Format::Text,
            cwd: None,
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run_compare(&[], &g, &mut out, &mut err), EXIT_USAGE);
        assert_eq!(
            run_compare(&["--x".into()], &g, &mut out, &mut err),
            EXIT_USAGE
        );
        assert!(out.is_empty());
    }
}
