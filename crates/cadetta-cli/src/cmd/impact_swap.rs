//! frob:ticket 01M4ST8F91FHYFC01TN6018BAM
//! `cadetta impact --swap PART NEW`: the variant evaluation of `PART=NEW` diffed against the
//! primary evaluation (obligations.md 5.1, sections 1 to 4).

use std::io::Write;

use cadetta_ir::model::{Check, Part, Project as Ir, VerdictState};
use serde_json::{Value as Json, json};

use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::{PackageDir, Project};
use crate::universe::Universe;
use crate::{EXIT_OK, EXIT_USAGE, Globals};
use cadetta_adapter::bindings::BindingSet;
use cadetta_obligations::lock::{Lock, Record};

/// Schema tag of the JSON report; the schema is `docs/schemas/swap.json`.
pub const SCHEMA: &str = "cadetta.swap/1";

#[path = "impact_swap_report.rs"]
pub mod report;

const USAGE: &str = "usage: cadetta impact --swap PART NEW";

/// Section 1: the overall outcome of the swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing fails and nothing is undecided.
    Ok,
    /// Something is undecided (unknown) under the swap.
    OkWithWarnings,
    /// A check or require is violated under the swap.
    Blocked,
}

impl Verdict {
    /// The words used in text and JSON output.
    pub fn word(self) -> &'static str {
        match self {
            Verdict::Ok => "ok",
            Verdict::OkWithWarnings => "ok with warnings",
            Verdict::Blocked => "blocked",
        }
    }
}

/// One value that differs between the primary and the swapped evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueChange {
    /// Canonical member path.
    pub path: String,
    /// Value under the primary, as authored; `None` when the member had none.
    pub old: Option<String>,
    /// Value under the swap, as authored.
    pub new: String,
    /// True for an interface member of the swapped part, false for a derived value.
    pub direct: bool,
}

/// One check whose verdict differs between the primary and the swap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flip {
    /// Canonical check path.
    pub path: String,
    /// Verdict word under the primary (`pass`, `fail`, `unknown`, `not evaluated`).
    pub old: &'static str,
    /// Verdict word under the swap.
    pub new: &'static str,
}

/// The preview of one swap.
#[derive(Debug, Clone, PartialEq)]
pub struct Swap {
    /// Canonical path of the slot.
    pub slot: String,
    /// The replacement component.
    pub new: String,
    /// True when `new` is in the slot's `alt` list.
    pub approved: bool,
    /// Section 1.
    pub verdict: Verdict,
    /// Section 2: direct values first, then derived, each sorted by path.
    pub values: Vec<ValueChange>,
    /// Section 3: flipped checks, sorted by path.
    pub flips: Vec<Flip>,
    /// Section 4: the new `part` line.
    pub suggested_edit: String,
    /// Sections 5 to 7: CAD re-sync, attestations invalidated, commercial deltas.
    pub report: report::Report,
}

/// Why a swap cannot be previewed.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SwapError {
    /// `PART` names no part slot.
    #[error("`{0}` is not a part slot of this design")]
    NoSlot(String),
    /// `NEW` is not a component of this design.
    #[error("`{0}` is not a component of this design")]
    NoComponent(String),
    /// `NEW` does not implement the slot's interface.
    #[error("`{new}` does not implement `{iface}`; NEW must implement the part's interface")]
    NotImplementer {
        /// The replacement.
        new: String,
        /// The slot's interface.
        iface: String,
    },
}

/// The component identity of path text: everything before the first `.` or `/` of the tail.
fn component_of(text: &str) -> &str {
    let tail_at = text.rfind("::").map_or(0, |i| i + 2);
    let end = text[tail_at..]
        .find(['.', '/'])
        .map_or(text.len(), |i| tail_at + i);
    &text[..end]
}

/// True when `name` denotes the component or interface `want`; either side may be a full path or a
/// bare item name, and the match is symmetric.
fn names(name: &str, want: &str) -> bool {
    let (name, want) = (component_of(name), component_of(want));
    let tail = |s: &str| s.rsplit("::").next().map(str::to_owned);
    name == want
        || (!want.contains("::") && tail(name).as_deref() == Some(want))
        || (!name.contains("::") && tail(want).as_deref() == Some(name))
}

fn verdict_word(v: Option<VerdictState>) -> &'static str {
    match v {
        Some(VerdictState::Pass) => "pass",
        Some(VerdictState::Fail) => "fail",
        Some(VerdictState::Unknown) => "unknown",
        None => "not evaluated",
    }
}

/// The verdict of `check` for the variant `key`, falling back to the primary when the
/// variant evaluation kept no differing entry.
fn swapped(check: &Check, key: &str) -> Option<VerdictState> {
    check
        .verdicts
        .per_alternate
        .get(key)
        .copied()
        .or(check.verdicts.primary)
}

/// Find the slot `part` names: a full canonical path, or a suffix of one.
fn find_slot<'a>(ir: &'a Ir, part: &str) -> Option<&'a Part> {
    let slots = || ir.components.iter().flat_map(|c| &c.parts);
    slots().find(|p| p.path.to_string() == part).or_else(|| {
        slots().find(|p| {
            let full = p.path.to_string();
            full.ends_with(part) && full[..full.len() - part.len()].ends_with(['.', '/', ':'])
        })
    })
}

/// The new `part` line: `new` as primary, the rest of `alt` kept minus `new`, old primary last.
fn suggested_edit(part: &Part, new: &str) -> String {
    let full = part.path.to_string();
    let name = full.rsplit(['.', '/']).next().unwrap_or(&full);
    let count = part.count.map_or(String::new(), |n| format!("[{n}]"));
    let mut alts: Vec<&str> = part
        .alternates
        .iter()
        .map(String::as_str)
        .filter(|a| !names(a, new))
        .collect();
    alts.push(&part.primary);
    format!(
        "part {name}{count}: {} = {new} alt [{}]",
        part.type_name,
        alts.join(", ")
    )
}

/// Preview swapping slot `part` to `new` using the variant data carried by `ir`, without
/// attestation scopes.
pub fn preview(ir: &Ir, part: &str, new: &str) -> Result<Swap, SwapError> {
    preview_with(ir, part, new, &[])
}

/// Preview the swap, judging attestations against `scopes` (frob:ticket 01M4ST8FAX8HDAXH8NBC8VKPXW).
pub fn preview_with(
    ir: &Ir,
    part: &str,
    new: &str,
    scopes: &[report::AttestScope],
) -> Result<Swap, SwapError> {
    let slot = find_slot(ir, part).ok_or_else(|| SwapError::NoSlot(part.to_owned()))?;
    let slot_path = slot.path.to_string();
    let approved = slot.alternates.iter().any(|a| names(a, new));
    let component = ir
        .components
        .iter()
        .find(|c| names(&c.path.to_string(), new))
        .ok_or_else(|| SwapError::NoComponent(new.to_owned()))?;
    if !approved
        && !component
            .implements
            .iter()
            .any(|i| names(i, &slot.type_name))
    {
        tracing::warn!(new, iface = %slot.type_name, "impact --swap: not an implementer");
        return Err(SwapError::NotImplementer {
            new: new.to_owned(),
            iface: slot.type_name.clone(),
        });
    }
    // Variant keys use whatever spelling the evaluator stored; match on the slot and alternate.
    let matches_key = |key: &str| {
        key.split_once('=')
            .is_some_and(|(s, a)| names(&slot_path, s) && names(a, new))
    };

    let mut values = Vec::new();
    for m in ir.components.iter().flat_map(|c| &c.members) {
        let Some(variants) = &m.variants else {
            continue;
        };
        let Some((_, v)) = variants.iter().find(|(k, _)| matches_key(k)) else {
            continue;
        };
        let old = m.value.as_ref().map(|o| o.authored.clone());
        if old.as_deref() == Some(v.authored.as_str()) {
            continue;
        }
        let path = m.path.to_string();
        let owner = component_of(&path);
        values.push(ValueChange {
            direct: names(owner, new) || names(owner, &slot.primary),
            path,
            old,
            new: v.authored.clone(),
        });
    }
    // Direct (interface members of the swapped part) first, then derived; each by path.
    values.sort_by(|a, b| (!a.direct, &a.path).cmp(&(!b.direct, &b.path)));

    let mut flips = Vec::new();
    let mut verdict = Verdict::Ok;
    for c in &ir.checks {
        let key = c
            .verdicts
            .per_alternate
            .keys()
            .find(|k| matches_key(k))
            .map(String::as_str);
        let after = key.map_or(c.verdicts.primary, |k| swapped(c, k));
        match after {
            // A violation blocks; a waiver never lifts it, since a violated require cannot
            // be waived and the IR does not mark which checks are requires (obligations.md 4).
            Some(VerdictState::Fail) => verdict = Verdict::Blocked,
            Some(VerdictState::Unknown) if verdict == Verdict::Ok => {
                verdict = Verdict::OkWithWarnings;
            }
            _ => {}
        }
        if after != c.verdicts.primary {
            flips.push(Flip {
                path: c.path.to_string(),
                old: verdict_word(c.verdicts.primary),
                new: verdict_word(after),
            });
        }
    }
    flips.sort_by(|a, b| a.path.cmp(&b.path));
    let mut moved: std::collections::BTreeSet<String> =
        values.iter().map(|v| v.path.clone()).collect();
    moved.extend(flips.iter().map(|f| f.path.clone()));
    moved.insert(slot_path.clone());
    let report = report::build(ir, slot, new, &moved, scopes);
    tracing::debug!(
        slot = %slot_path, new, approved,
        values = values.len(), flips = flips.len(), verdict = verdict.word(),
        "impact --swap: previewed"
    );
    Ok(Swap {
        slot: slot_path,
        new: new.to_owned(),
        approved,
        verdict,
        values,
        flips,
        suggested_edit: suggested_edit(slot, new),
        report,
    })
}

/// Render the swap as text or the `cadetta.swap/1` JSON document.
pub fn render(swap: &Swap, format: Format, out: &mut dyn Write) {
    if format == Format::Json {
        let doc = json!({
            "schema": SCHEMA,
            "swap": {"slot": swap.slot, "alternate": swap.new, "approved": swap.approved},
            "verdict": swap.verdict.word(),
            "values_changed": swap.values.iter().map(|v| json!({
                "path": v.path, "old": v.old, "new": v.new,
                "class": if v.direct { "direct" } else { "derived" },
            })).collect::<Vec<Json>>(),
            "verdicts_flipped": swap.flips.iter().map(|f| json!({
                "path": f.path, "old": f.old, "new": f.new,
            })).collect::<Vec<Json>>(),
            "suggested_edit": swap.suggested_edit,
        });
        let mut doc = doc;
        if let Some(obj) = doc.as_object_mut() {
            for (k, v) in report::to_json(&swap.report) {
                obj.insert(k.to_owned(), v);
            }
        }
        let _ = writeln!(out, "{doc}");
        return;
    }
    let _ = writeln!(out, "swap {} -> {}", swap.slot, swap.new);
    if !swap.approved {
        let _ = writeln!(
            out,
            "  {} is not yet approved (not in the alt list); it must still implement the interface",
            swap.new
        );
    }
    let _ = writeln!(out, "1. verdict: {}", swap.verdict.word());
    let _ = writeln!(out, "2. values that change: {}", swap.values.len());
    for v in &swap.values {
        let class = if v.direct { "direct" } else { "derived" };
        let _ = writeln!(
            out,
            "  {} [{class}]: {} -> {}",
            v.path,
            v.old.as_deref().unwrap_or("-"),
            v.new
        );
    }
    let _ = writeln!(out, "3. verdicts that flip: {}", swap.flips.len());
    for f in &swap.flips {
        let _ = writeln!(out, "  {}: {} -> {}", f.path, f.old, f.new);
    }
    let _ = writeln!(out, "4. suggested edit:");
    let _ = writeln!(out, "  {}", swap.suggested_edit);
    report::write_text(&swap.report, out);
}

/// The attestation scopes of `ir`: every lock attestation on a check, with its covered keys.
fn attest_scopes(ir: &Ir, lock: &Lock) -> Vec<report::AttestScope> {
    let universe = match Universe::of(ir) {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(error = %e, "impact --swap: no obligation universe; skipping attestations");
            return Vec::new();
        }
    };
    let mut scopes = Vec::new();
    for rec in lock.records() {
        let Record::Attest(a) = rec else { continue };
        let Some(entry) = universe.entry(&a.id) else {
            continue;
        };
        scopes.push(report::AttestScope {
            id: a.id.to_string(),
            role: a.role.clone(),
            covered: universe.covered_keys(&entry.key),
        });
    }
    scopes
}

/// Load every adapter's bindings file of `pkg` into `ir.bindings` so section 5 can see them.
// frob:ticket 01M4VQXMQ16RXW2ACNQHSDKA6V
fn load_bindings(ir: &mut Ir, pkg: &PackageDir) {
    for adapter in &pkg.config.adapters {
        let file = pkg.dir.join(adapter.bindings_path());
        let Ok(text) = std::fs::read_to_string(&file) else {
            tracing::debug!(file = %file.display(), "impact --swap: no bindings file");
            continue;
        };
        match BindingSet::load_str(adapter.id(), &text) {
            Ok(set) => {
                tracing::debug!(adapter = %adapter.id(), n = set.entries().len(), "impact --swap: bindings loaded");
                ir.bindings.extend(set.file_entries());
            }
            Err(e) => {
                tracing::warn!(file = %file.display(), error = %e, "impact --swap: bindings unreadable")
            }
        }
    }
}

/// Run `cadetta impact --swap PART NEW`; `args` are `PART NEW`. Returns the exit code.
pub fn run_swap(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "impact --swap failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let [part, new] = args else {
        return fail(err, USAGE);
    };
    if part.starts_with("--") || new.starts_with("--") {
        return fail(err, &format!("unknown flag; {USAGE}"));
    }
    let (project, members) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref())
    {
        Ok(o) => o,
        Err(e) => return fail(err, &e.to_string()),
    };
    let mut analysis = analyze(&project, &members);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    let lock = Lock::load(&project.root.join("cadetta.lock")).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "impact --swap: lock unreadable; attestations skipped");
        Lock::new()
    });
    let mut last = SwapError::NoSlot(part.clone());
    for package in &mut analysis.packages {
        if let Some(pkg) = project.packages.iter().find(|p| {
            p.config.package.as_ref().and_then(|c| c.name.as_deref()) == Some(package.name.as_str())
        }) {
            load_bindings(&mut package.ir, pkg);
        }
        let scopes = attest_scopes(&package.ir, &lock);
        match preview_with(&package.ir, part, new, &scopes) {
            Ok(swap) => {
                render(&swap, globals.format, out);
                return EXIT_OK;
            }
            Err(SwapError::NoSlot(_)) => {}
            Err(e) => last = e,
        }
    }
    fail(err, &last.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{
        Component, ComponentClass, Exact, Flavour, Member, MemberKind, Nominal, PackageId, Value,
        Verdicts,
    };
    use cadetta_ir::path::{Path, PathContext};
    use std::collections::BTreeMap;

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn val(text: &str) -> Value {
        let n: i64 = text.parse().unwrap();
        Value::new(
            Nominal::Exact(Exact(cadetta_units::Magnitude::new(
                cadetta_units::Rational::new(n, 1).unwrap(),
                0,
            ))),
            cadetta_units::Dim::default(),
            text.to_owned(),
        )
    }

    fn member(path: &str, old: &str, new: &str) -> Member {
        let mut m = Member::new(p(path), MemberKind::Param);
        m.value = Some(val(old));
        m.variants = Some(BTreeMap::from([(
            "pkg::Rover.wheel=pkg::Colson".to_owned(),
            val(new),
        )]));
        m
    }

    fn comp(path: &str, implements: &[&str]) -> Component {
        Component {
            path: p(path),
            class: ComponentClass::Model,
            params_sig: None,
            implements: implements.iter().map(|s| (*s).to_owned()).collect(),
            citation: None,
            attrs: Default::default(),
            members: vec![],
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        }
    }

    fn check(path: &str, primary: VerdictState, swap: Option<VerdictState>) -> Check {
        let mut v = Verdicts {
            primary: Some(primary),
            per_alternate: BTreeMap::new(),
        };
        if let Some(s) = swap {
            v.per_alternate
                .insert("pkg::Rover.wheel=pkg::Colson".into(), s);
        }
        Check {
            path: p(path),
            expr: "x".into(),
            verdicts: v,
            conservative: false,
            waived_for: vec![],
        }
    }

    fn ir(alts: &[&str]) -> Ir {
        let mut ir = Ir::new(
            Flavour::Design,
            PackageId {
                name: "pkg".into(),
                version: "0.1.0".into(),
            },
        );
        let mut rover = comp("pkg::Rover.x", &[]);
        rover.parts.push(Part {
            path: p("pkg::Rover.wheel"),
            type_name: "pkg::Caster".into(),
            count: Some(4),
            primary: "pkg::Vendor".into(),
            alternates: alts.iter().map(|s| (*s).to_owned()).collect(),
            args: BTreeMap::new(),
        });
        rover.members = vec![
            member("pkg::Rover.total", "10", "12"),
            member("pkg::Colson.bore", "5", "6"),
            member("pkg::Rover.same", "1", "1"),
        ];
        ir.components.push(rover);
        ir.components.push(comp("pkg::Vendor.x", &["pkg::Caster"]));
        ir.components.push(comp("pkg::Colson.x", &["pkg::Caster"]));
        ir.components.push(comp("pkg::Plain.x", &[]));
        ir.checks = vec![
            check(
                "pkg::Rover.b",
                VerdictState::Pass,
                Some(VerdictState::Unknown),
            ),
            check("pkg::Rover.a", VerdictState::Pass, Some(VerdictState::Fail)),
            check("pkg::Rover.c", VerdictState::Pass, None),
        ];
        ir
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap.rs::preview
    #[test]
    fn diffs_variant_against_primary_and_blocks_on_failure() {
        let s = preview(&ir(&["pkg::Colson"]), "wheel", "pkg::Colson").unwrap();
        assert!(s.approved);
        assert_eq!(s.verdict, Verdict::Blocked);
        let got: Vec<_> = s.values.iter().map(|v| v.path.as_str()).collect();
        assert_eq!(got, ["pkg::Colson.bore", "pkg::Rover.total"]);
        assert_eq!(s.values[0].old.as_deref(), Some("5"));
        assert_eq!(s.values[0].new, "6");
    }

    // frob:ticket 01M4W47VTBXFMC70Q2R137W4EE
    // frob:tests crates/cadetta-cli/src/cmd/impact_swap.rs::preview
    #[test]
    fn slash_slot_and_bare_alt_match_a_full_path_new() {
        let mut i = ir(&["Colson"]);
        i.components[0].parts[0].path = p("pkg::Rover/wheel");
        let s = preview(&i, "pkg::Rover/wheel", "pkg::Colson").unwrap();
        assert!(s.approved);
        assert!(names("Colson", "pkg::Colson") && names("pkg::Colson", "Colson"));
        assert!(!names("Other", "pkg::Colson"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap.rs::preview
    #[test]
    fn warnings_without_failure_and_ok_when_nothing_moves() {
        let mut i = ir(&["pkg::Colson"]);
        i.checks.remove(1);
        assert_eq!(
            preview(&i, "wheel", "pkg::Colson").unwrap().verdict,
            Verdict::OkWithWarnings
        );
        i.checks.remove(0);
        assert_eq!(
            preview(&i, "wheel", "pkg::Colson").unwrap().verdict,
            Verdict::Ok
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap.rs::preview
    #[test]
    fn direct_values_precede_derived() {
        let s = preview(&ir(&["pkg::Colson"]), "wheel", "pkg::Colson").unwrap();
        let classes: Vec<_> = s.values.iter().map(|v| v.direct).collect();
        assert_eq!(classes, [true, false]);
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap.rs::preview
    #[test]
    fn flips_sorted_by_canonical_path() {
        let s = preview(&ir(&["pkg::Colson"]), "wheel", "pkg::Colson").unwrap();
        let got: Vec<_> = s.flips.iter().map(|f| (f.path.as_str(), f.new)).collect();
        assert_eq!(got, [("pkg::Rover.a", "fail"), ("pkg::Rover.b", "unknown")]);
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap.rs::render
    #[test]
    fn unapproved_new_says_so_and_must_implement() {
        let s = preview(&ir(&[]), "wheel", "pkg::Colson").unwrap();
        assert!(!s.approved);
        let mut out = Vec::new();
        render(&s, Format::Text, &mut out);
        assert!(String::from_utf8(out).unwrap().contains("not yet approved"));
        assert!(matches!(
            preview(&ir(&[]), "wheel", "pkg::Plain"),
            Err(SwapError::NotImplementer { .. })
        ));
        assert_eq!(
            preview(&ir(&[]), "nope", "pkg::Colson"),
            Err(SwapError::NoSlot("nope".into()))
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap.rs::preview
    #[test]
    fn suggested_edit_removes_new_and_appends_old_primary() {
        let s = preview(&ir(&["pkg::Colson", "pkg::Other"]), "wheel", "pkg::Colson").unwrap();
        assert_eq!(
            s.suggested_edit,
            "part wheel[4]: pkg::Caster = pkg::Colson alt [pkg::Other, pkg::Vendor]"
        );
    }

    // frob:ticket 01M4SWN8G3QTQC5YK07VN1YQYC
    // frob:tests crates/cadetta-cli/src/cmd/impact_swap.rs::render
    #[test]
    fn json_validates_against_swap_schema() {
        let schema: Json =
            serde_json::from_str(include_str!("../../../../docs/schemas/swap.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let s = preview(&ir(&["pkg::Colson"]), "wheel", "pkg::Colson").unwrap();
        let mut out = Vec::new();
        render(&s, Format::Json, &mut out);
        let v: Json = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["schema"], SCHEMA);
        assert!(validator.is_valid(&v), "{v}");
    }
}
