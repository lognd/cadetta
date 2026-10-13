//! frob:ticket 01M4ST8FAX8HDAXH8NBC8VKPXW
//! `cadetta impact --swap`, sections 5 to 7: CAD files to re-sync, attestations invalidated and
//! commercial deltas (obligations.md 5.1). Pure functions over the IR; the parent module owns the
//! evaluation diff and calls [`build`] with the changed paths.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use cadetta_ir::model::{Component, Member, Nominal, Part, Project as Ir, Value};
use cadetta_units::display::{DisplaySpec, format_money, format_percent, format_value};
use cadetta_units::{Dim, Magnitude, Rational, Sum, UnitTable};
use serde_json::{Value as Json, json};

use super::names;

/// True when a variant key `"<slot>=<alternate>"` is the preview of `slot_path` swapped to `new`.
pub fn key_matches(key: &str, slot_path: &str, new: &str) -> bool {
    key.split_once('=')
        .is_some_and(|(s, a)| names(slot_path, s) && names(a, new))
}

/// One attestation and the graph keys its covered closure spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestScope {
    /// The obligation id text (`check:PATH`).
    pub id: String,
    /// The attesting role.
    pub role: String,
    /// The slot-closure: the obligation's own key and every transitive dependency.
    pub covered: BTreeSet<String>,
}

/// A file with the bound paths whose emitted value changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleFile {
    /// The CAD file.
    pub file: String,
    /// Changed bound paths, sorted.
    pub paths: Vec<String>,
}

/// Section 5.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CadResync {
    /// Files to re-sync, sorted by name.
    pub stale: Vec<StaleFile>,
    /// Files bound to paths that do not change, sorted.
    pub current: Vec<String>,
    /// Replace-component actions for assembly files.
    pub replaces: Vec<String>,
}

/// One attestation that the swap invalidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestHit {
    /// The obligation id text.
    pub id: String,
    /// The attesting role.
    pub role: String,
    /// Covered nodes whose value, verdict or slot choice changes.
    pub affected: usize,
}

/// An exact quantity in SI base units (or a currency), with its display dimension.
#[derive(Debug, Clone, PartialEq)]
pub struct Amount {
    /// The exact magnitude.
    pub magnitude: Magnitude,
    /// Its dimension.
    pub dim: Dim,
}

/// A before and after roll-up value.
#[derive(Debug, Clone, PartialEq)]
pub struct Delta {
    /// The roll-up as written, with the currency for money.
    pub name: String,
    /// Value under the primary; `None` when no part supplies it.
    pub old: Option<Amount>,
    /// Value under the swap.
    pub new: Option<Amount>,
}

/// Parts lacking a roll-up member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    /// The member (`unit_cost`, ...).
    pub param: String,
    /// `before` or `after` the swap.
    pub side: &'static str,
    /// Part slots lacking it, sorted.
    pub parts: Vec<String>,
}

/// Section 7.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Commercial {
    /// The assembly the roll-ups are taken at.
    pub assembly: String,
    /// `sum?` per currency, `max?` and `mass?`.
    pub deltas: Vec<Delta>,
    /// `moq` of the swapped part, old then new.
    pub moq: Option<Delta>,
    /// Parts lacking a value.
    pub missing: Vec<Missing>,
}

/// Sections 5 to 7.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// Section 5.
    pub cad: CadResync,
    /// Section 6, sorted by obligation id then role.
    pub attestations: Vec<AttestHit>,
    /// Section 7.
    pub commercial: Commercial,
}

/// The file a locator addresses: the last `@` segment when it names a file, else the adapter.
fn file_of(locator: &str, adapter: &str) -> String {
    match locator.rsplit('@').next() {
        Some(tail) if locator.contains('@') && tail.contains('.') && !tail.contains(' ') => {
            tail.to_owned()
        }
        _ => format!("({adapter} globals)"),
    }
}

/// Section 5: stale and current files, and the component replacements.
fn cad_resync(ir: &Ir, slot_path: &str, new: &str, changed: &BTreeSet<String>) -> CadResync {
    let slot_name = slot_path.rsplit(['.', '/']).next().unwrap_or(slot_path);
    let mut stale: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut all: BTreeSet<String> = BTreeSet::new();
    let mut new_file: Option<String> = None;
    let mut instances: BTreeSet<(String, String)> = BTreeSet::new();
    for b in &ir.bindings {
        let path = b.path.to_string();
        let moves = changed.contains(&path);
        for l in &b.locators {
            let file = file_of(&l.locator, &b.adapter);
            all.insert(file.clone());
            let for_new = l.for_alternate.as_deref().is_some_and(|a| names(a, new));
            let active = l.for_alternate.is_none() || for_new;
            if active && (moves || for_new) {
                stale.entry(file.clone()).or_default().insert(path.clone());
            }
            if for_new && l.component.is_none() && new_file.is_none() {
                new_file = Some(file.clone());
            }
            if let Some(c) = &l.component {
                instances.insert((c.clone(), file));
            }
        }
    }
    let replaces = new_file
        .map(|nf| {
            instances
                .iter()
                .filter(|(c, _)| c.split('<').next() == Some(slot_name))
                .map(|(c, f)| format!("replace {c} with {nf} in {f}"))
                .collect()
        })
        .unwrap_or_default();
    let current = all
        .iter()
        .filter(|f| !stale.contains_key(*f))
        .cloned()
        .collect();
    tracing::debug!(stale = stale.len(), "impact --swap: cad resync");
    CadResync {
        stale: stale
            .into_iter()
            .map(|(file, paths)| StaleFile {
                file,
                paths: paths.into_iter().collect(),
            })
            .collect(),
        current,
        replaces,
    }
}

/// Section 6: attestations whose covered closure meets the changed set.
fn attestations(scopes: &[AttestScope], changed: &BTreeSet<String>) -> Vec<AttestHit> {
    let mut hits: Vec<AttestHit> = scopes
        .iter()
        .filter_map(|s| {
            let affected = s.covered.intersection(changed).count();
            (affected > 0).then(|| AttestHit {
                id: s.id.clone(),
                role: s.role.clone(),
                affected,
            })
        })
        .collect();
    hits.sort_by(|a, b| (&a.id, &a.role).cmp(&(&b.id, &b.role)));
    hits
}

/// One part instance's contribution to a roll-up.
struct Item {
    slot: String,
    count: u32,
    value: Option<(Magnitude, Dim)>,
}

/// The value of `m` under the swap (when active) or the primary.
fn value_of<'a>(m: &'a Member, swap: Option<(&str, &str)>) -> Option<&'a Value> {
    let variant = swap.and_then(|(slot, new)| {
        m.variants
            .as_ref()?
            .iter()
            .find(|(k, _)| key_matches(k, slot, new))
            .map(|(_, v)| v)
    });
    variant.or(m.value.as_ref())
}

/// The member `param` of `c`.
fn member_of<'a>(c: &'a Component, param: &str) -> Option<&'a Member> {
    c.members.iter().find(|m| {
        let p = m.path.to_string();
        p.rsplit('.').next() == Some(param)
    })
}

/// Every part instance below `assembly`, recursively, with the swapped slot replaced by `new`.
/// A component carrying the member itself stops the descent; one without it that has parts
/// contributes through those parts (language.md 3.5).
fn walk(
    ir: &Ir,
    assembly: &Component,
    swap: Option<(&str, &str)>,
    param: &str,
    mult: u32,
    out: &mut Vec<Item>,
) {
    for part in &assembly.parts {
        let slot = part.path.to_string();
        let chosen = match swap {
            Some((s, new)) if s == slot => new,
            _ => part.primary.as_str(),
        };
        let count = part.count.unwrap_or(1) * mult;
        let comp = ir
            .components
            .iter()
            .find(|c| names(&c.path.to_string(), chosen));
        let value = comp
            .and_then(|c| member_of(c, param))
            .and_then(|m| value_of(m, swap))
            .and_then(|v| match &v.nominal {
                Nominal::Exact(e) => Some((e.0.clone(), v.dim.clone())),
                Nominal::Inexact(_) => None,
            });
        match comp {
            Some(c) if value.is_none() && !c.parts.is_empty() => {
                walk(ir, c, swap, param, count, out);
            }
            _ => out.push(Item { slot, count, value }),
        }
    }
}

/// The currency code of a money dimension.
fn currency_of(dim: &Dim) -> Option<String> {
    dim.dimension
        .currencies()
        .iter()
        .find(|(_, e)| **e == 1)
        .map(|(c, _)| c.clone())
}

fn times(m: &Magnitude, n: u32) -> Magnitude {
    m.mul(&Magnitude::new(Rational::from_int(n), 0))
}

/// Sum the items, grouped by currency (`""` for non-money); inexact terms are skipped.
fn sum_items(items: &[Item]) -> BTreeMap<String, Amount> {
    let mut out: BTreeMap<String, Amount> = BTreeMap::new();
    for it in items {
        let Some((m, dim)) = &it.value else { continue };
        let term = times(m, it.count);
        let key = currency_of(dim).unwrap_or_default();
        match out.get_mut(&key) {
            None => {
                out.insert(
                    key,
                    Amount {
                        magnitude: term,
                        dim: dim.clone(),
                    },
                );
            }
            Some(a) => match a.magnitude.add(&term) {
                Sum::Exact(s) => a.magnitude = s,
                Sum::Inexact(_) => {
                    tracing::warn!(slot = %it.slot, "impact --swap: inexact term skipped in roll-up");
                }
            },
        }
    }
    out
}

/// True when `a` is greater than `b` (same dimension, pi exponent 0 assumed for ordering).
fn greater(a: &Magnitude, b: &Magnitude) -> bool {
    let (x, y) = (a.rational(), b.rational());
    (x.numer() * y.denom()) > (y.numer() * x.denom())
}

fn max_items(items: &[Item]) -> Option<Amount> {
    items.iter().filter_map(|i| i.value.as_ref()).fold(
        None,
        |acc: Option<Amount>, (m, d)| match acc {
            Some(a) if !greater(m, &a.magnitude) => Some(a),
            _ => Some(Amount {
                magnitude: m.clone(),
                dim: d.clone(),
            }),
        },
    )
}

fn missing_of(items: &[Item]) -> Vec<String> {
    let set: BTreeSet<String> = items
        .iter()
        .filter(|i| i.value.is_none())
        .map(|i| i.slot.clone())
        .collect();
    set.into_iter().collect()
}

/// The component named by the slot's prefix: the one that declares the slot.
fn assembly_of<'a>(ir: &'a Ir, slot: &Part) -> Option<&'a Component> {
    ir.components
        .iter()
        .find(|c| c.parts.iter().any(|p| p.path == slot.path))
}

/// Section 7.
fn commercial(ir: &Ir, slot: &Part, new: &str) -> Commercial {
    let slot_path = slot.path.to_string();
    let Some(assembly) = assembly_of(ir, slot) else {
        tracing::warn!(slot = %slot_path, "impact --swap: no assembly declares the slot");
        return Commercial::default();
    };
    let swap = Some((slot_path.as_str(), new));
    let collect = |s, param: &str| {
        let mut v = Vec::new();
        walk(ir, assembly, s, param, 1, &mut v);
        v
    };
    let mut c = Commercial {
        assembly: assembly.path.to_string(),
        ..Commercial::default()
    };
    let mut note = |param: &str, before: &[Item], after: &[Item]| {
        for (side, items) in [("before", before), ("after", after)] {
            let parts = missing_of(items);
            if !parts.is_empty() {
                c.missing.push(Missing {
                    param: param.to_owned(),
                    side,
                    parts,
                });
            }
        }
    };
    let (cost_a, cost_b) = (collect(None, "unit_cost"), collect(swap, "unit_cost"));
    note("unit_cost", &cost_a, &cost_b);
    let (old, new_sum) = (sum_items(&cost_a), sum_items(&cost_b));
    let codes: BTreeSet<&String> = old.keys().chain(new_sum.keys()).collect();
    for code in codes {
        c.deltas.push(Delta {
            name: format!("sum?(self, unit_cost) {code}")
                .trim_end()
                .to_owned(),
            old: old.get(code).cloned(),
            new: new_sum.get(code).cloned(),
        });
    }
    let (lt_a, lt_b) = (collect(None, "lead_time"), collect(swap, "lead_time"));
    note("lead_time", &lt_a, &lt_b);
    c.deltas.push(Delta {
        name: "max?(self, lead_time)".into(),
        old: max_items(&lt_a),
        new: max_items(&lt_b),
    });
    let (ms_a, ms_b) = (collect(None, "mass"), collect(swap, "mass"));
    note("mass", &ms_a, &ms_b);
    let total = |items: &[Item]| sum_items(items).into_values().next();
    c.deltas.push(Delta {
        name: "mass?(self)".into(),
        old: total(&ms_a),
        new: total(&ms_b),
    });
    let moq_of = |chosen: &str| {
        ir.components
            .iter()
            .find(|k| names(&k.path.to_string(), chosen))
            .and_then(|k| member_of(k, "moq"))
            .and_then(|m| m.value.as_ref())
            .and_then(|v| match &v.nominal {
                Nominal::Exact(e) => Some(Amount {
                    magnitude: e.0.clone(),
                    dim: v.dim.clone(),
                }),
                Nominal::Inexact(_) => None,
            })
    };
    c.moq = Some(Delta {
        name: format!("moq {slot_path}"),
        old: moq_of(&slot.primary),
        new: moq_of(new),
    });
    tracing::debug!(
        deltas = c.deltas.len(),
        missing = c.missing.len(),
        "impact --swap: commercial"
    );
    c
}

/// Build sections 5 to 7 for the swap of `slot` to `new`; `changed` holds every path whose value
/// or verdict differs, plus the slot itself.
pub fn build(
    ir: &Ir,
    slot: &Part,
    new: &str,
    changed: &BTreeSet<String>,
    scopes: &[AttestScope],
) -> Report {
    let slot_path = slot.path.to_string();
    let values: BTreeSet<String> = changed
        .iter()
        .filter(|p| **p != slot_path)
        .cloned()
        .collect();
    Report {
        cad: cad_resync(ir, &slot_path, new, &values),
        attestations: attestations(scopes, changed),
        commercial: commercial(ir, slot, new),
    }
}

/// The unit symbol a quantity of `dim` prints in, if it has one.
fn unit_for(dim: &Dim) -> Option<String> {
    if let Some(code) = currency_of(dim) {
        return Some(code);
    }
    let si = dim.dimension.si();
    match si {
        [0, 1, 0, 0, 0, 0, 0] => Some("kg".into()),
        [0, 0, 1, 0, 0, 0, 0] => Some("s".into()),
        _ => None,
    }
}

/// Source literal of an amount in storage units, e.g. `25/2 USD`; exact, never rounded.
fn literal(a: &Amount) -> String {
    let m = &a.magnitude;
    let num = match m.pi_exp() {
        0 => m.rational().to_string(),
        k => format!("{} pi^{k}", m.rational()),
    };
    match unit_for(&a.dim) {
        Some(u) => format!("{num} {u}"),
        None => num,
    }
}

/// Display text of an amount: money at the minor unit, mass in kg, time in days.
fn display(a: &Amount) -> String {
    if let Some(code) = currency_of(&a.dim)
        && a.magnitude.pi_exp() == 0
    {
        return format_money(a.magnitude.rational(), &code);
    }
    let table = UnitTable::builtin();
    let spec = match unit_for(&a.dim).as_deref() {
        Some("kg") => Some("kg"),
        Some("s") => Some("d"),
        _ => None,
    };
    spec.and_then(|u| format_value(&table, &a.magnitude, &a.dim, &DisplaySpec::Unit(u.into())).ok())
        .unwrap_or_else(|| literal(a))
}

/// `(+25.0%)` for a move from `old` to `new`, one decimal half away from zero; none when old is 0.
fn percent(old: &Amount, new: &Amount) -> Option<String> {
    if old.magnitude.is_zero() || old.dim != new.dim || old.magnitude.pi_exp() != 0 {
        return None;
    }
    let diff = new.magnitude.rational().sub(old.magnitude.rational());
    let ratio = diff.div(old.magnitude.rational()).ok()?;
    let text = format_percent(&ratio);
    Some(if text.starts_with('-') || text.starts_with("0.0") {
        text
    } else {
        format!("+{text}")
    })
}

fn delta_text(d: &Delta) -> String {
    let side = |a: &Option<Amount>| a.as_ref().map_or_else(|| "n/a".to_owned(), display);
    let pct = match (&d.old, &d.new) {
        (Some(o), Some(n)) => percent(o, n).map(|p| format!(" ({p})")).unwrap_or_default(),
        _ => String::new(),
    };
    format!("{}: {} -> {}{pct}", d.name, side(&d.old), side(&d.new))
}

fn amount_json(a: &Option<Amount>) -> Json {
    a.as_ref().map_or(Json::Null, |a| json!(literal(a)))
}

fn delta_json(d: &Delta) -> Json {
    json!({"name": d.name, "old": amount_json(&d.old), "new": amount_json(&d.new)})
}

/// The `cad_resync`, `attestations_invalidated` and `commercial_deltas` keys of `swap.json`.
pub fn to_json(r: &Report) -> [(&'static str, Json); 3] {
    [
        (
            "cad_resync",
            json!({
                "stale": r.cad.stale.iter()
                    .map(|f| json!({"file": f.file, "paths": f.paths}))
                    .collect::<Vec<Json>>(),
                "not_stale": r.cad.current,
                "replace_component": r.cad.replaces,
            }),
        ),
        (
            "attestations_invalidated",
            json!(r.attestations.iter()
                .map(|a| json!({"id": a.id, "role": a.role, "covered_nodes_affected": a.affected}))
                .collect::<Vec<Json>>()),
        ),
        (
            "commercial_deltas",
            json!({
                "assembly": r.commercial.assembly,
                "deltas": r.commercial.deltas.iter().map(delta_json).collect::<Vec<Json>>(),
                "moq": r.commercial.moq.as_ref().map_or(Json::Null, delta_json),
                "missing": r.commercial.missing.iter()
                    .map(|m| json!({"param": m.param, "side": m.side, "parts": m.parts}))
                    .collect::<Vec<Json>>(),
            }),
        ),
    ]
}

/// Print sections 5 to 7 as text.
pub fn write_text(r: &Report, out: &mut dyn Write) {
    let _ = writeln!(out, "5. CAD files to re-sync: {}", r.cad.stale.len());
    for f in &r.cad.stale {
        let _ = writeln!(out, "  {}: {}", f.file, f.paths.join(", "));
    }
    for f in &r.cad.current {
        let _ = writeln!(out, "  {f}: not stale");
    }
    for a in &r.cad.replaces {
        let _ = writeln!(out, "  {a}");
    }
    let _ = writeln!(out, "6. attestations invalidated: {}", r.attestations.len());
    for a in &r.attestations {
        let _ = writeln!(
            out,
            "  {} ({}): {} covered node(s) affected",
            a.id, a.role, a.affected
        );
    }
    let _ = writeln!(out, "7. commercial deltas at {}:", r.commercial.assembly);
    for d in &r.commercial.deltas {
        let _ = writeln!(out, "  {}", delta_text(d));
    }
    if let Some(m) = &r.commercial.moq {
        let _ = writeln!(out, "  {}", delta_text(m));
    }
    for m in &r.commercial.missing {
        let _ = writeln!(
            out,
            "  missing {} {} the swap: {}",
            m.param,
            m.side,
            m.parts.join(", ")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{
        Binding, ComponentClass, Exact, Flavour, Locator, LocatorKind, LocatorMode, MemberKind,
        PackageId,
    };
    use cadetta_ir::path::{Path, PathContext};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn amount(text: &str, dim: Dim) -> Value {
        let r = Rational::parse_decimal(text).unwrap();
        Value::new(
            Nominal::Exact(Exact(Magnitude::new(r, 0))),
            dim,
            text.to_owned(),
        )
    }

    fn usd() -> Dim {
        Dim::new(cadetta_units::Dimension::currency("USD"), 0)
    }

    fn mass() -> Dim {
        Dim::new(cadetta_units::Dimension::from_si([0, 1, 0, 0, 0, 0, 0]), 0)
    }

    fn comp(path: &str, members: &[(&str, Value)]) -> Component {
        Component {
            path: p(path),
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members: members
                .iter()
                .map(|(n, v)| {
                    let mut m = Member::new(p(&format!("{path}.{n}")), MemberKind::Param);
                    m.value = Some(v.clone());
                    m
                })
                .collect(),
            parts: vec![],
            fits: vec![],
            span: None,
            digest: None,
        }
    }

    fn slot() -> Part {
        Part {
            path: p("pkg::Rover.wheel"),
            type_name: "pkg::Caster".into(),
            count: Some(4),
            primary: "pkg::Vendor".into(),
            alternates: vec!["pkg::Colson".into()],
            args: Default::default(),
        }
    }

    fn locator(text: &str, for_alt: Option<&str>, component: Option<&str>) -> Locator {
        Locator {
            locator: text.into(),
            kind: LocatorKind::Dimension,
            mode: LocatorMode::Value,
            native_unit: "mm".into(),
            ref_to: None,
            for_alternate: for_alt.map(Into::into),
            config: None,
            sign: None,
            component: component.map(Into::into),
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
        let mut rover = comp("pkg::Rover.x", &[]);
        rover.parts.push(slot());
        let mut spare = slot();
        spare.path = p("pkg::Rover.spare");
        spare.primary = "pkg::Bare".into();
        spare.count = None;
        spare.alternates = vec![];
        rover.parts.push(spare);
        ir.components.push(rover);
        ir.components.push(comp(
            "pkg::Vendor.x",
            &[
                ("unit_cost", amount("10.005", usd())),
                ("mass", amount("2", mass())),
                ("moq", amount("5", Dim::default())),
            ],
        ));
        ir.components.push(comp(
            "pkg::Colson.x",
            &[
                ("unit_cost", amount("12.5", usd())),
                ("mass", amount("3", mass())),
            ],
        ));
        ir.components.push(comp("pkg::Bare.x", &[]));
        ir.bindings = vec![
            Binding {
                path: p("pkg::Rover.total"),
                adapter: "sw".into(),
                locators: vec![locator("D1@S@Rover.SLDPRT", None, None)],
            },
            Binding {
                path: p("pkg::Rover.same"),
                adapter: "sw".into(),
                locators: vec![locator("D1@S@Other.SLDPRT", None, None)],
            },
            Binding {
                path: p("pkg::Rover.wheel"),
                adapter: "sw".into(),
                locators: vec![
                    locator(
                        "Vendor.SLDPRT@Rover.SLDASM",
                        Some("pkg::Vendor"),
                        Some("wheel<2>"),
                    )
                    .clone(),
                    locator(
                        "Colson.SLDPRT@Colson2Series.SLDPRT",
                        Some("pkg::Colson"),
                        None,
                    ),
                    locator("Vendor.SLDPRT@Rover.SLDASM", None, Some("wheel<2>")),
                ],
            },
        ];
        ir
    }

    fn changed(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|s| (*s).to_owned()).collect()
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap_report.rs::build
    #[test]
    fn changed_bound_paths_list_files_and_unchanged_are_not_stale() {
        let r = build(
            &ir(),
            &slot(),
            "pkg::Colson",
            &changed(&["pkg::Rover.total"]),
            &[],
        );
        let files: Vec<_> = r.cad.stale.iter().map(|f| f.file.as_str()).collect();
        assert!(files.contains(&"Rover.SLDPRT"));
        assert_eq!(
            r.cad.stale[files.iter().position(|f| *f == "Rover.SLDPRT").unwrap()].paths,
            ["pkg::Rover.total"]
        );
        assert!(r.cad.current.contains(&"Other.SLDPRT".to_owned()));
        assert!(!r.cad.current.contains(&"Rover.SLDPRT".to_owned()));
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap_report.rs::build
    #[test]
    fn assembly_replace_action_and_alternate_locators() {
        let r = build(&ir(), &slot(), "pkg::Colson", &changed(&[]), &[]);
        assert_eq!(
            r.cad.replaces,
            ["replace wheel<2> with Colson2Series.SLDPRT in Rover.SLDASM"]
        );
        assert!(r.cad.stale.iter().any(|f| f.file == "Colson2Series.SLDPRT"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap_report.rs::build
    #[test]
    fn attestations_count_affected_covered_nodes() {
        let scopes = [
            AttestScope {
                id: "check:pkg::Rover.b".into(),
                role: "lead".into(),
                covered: changed(&["pkg::Rover.b", "pkg::Rover.total", "pkg::Rover.x"]),
            },
            AttestScope {
                id: "check:pkg::Rover.c".into(),
                role: "lead".into(),
                covered: changed(&["pkg::Rover.x"]),
            },
        ];
        let r = build(
            &ir(),
            &slot(),
            "pkg::Colson",
            &changed(&["pkg::Rover.total", "pkg::Rover.b"]),
            &scopes,
        );
        assert_eq!(
            r.attestations,
            [AttestHit {
                id: "check:pkg::Rover.b".into(),
                role: "lead".into(),
                affected: 2
            }]
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap_report.rs::build
    #[test]
    fn commercial_deltas_count_every_instance_and_list_missing_parts() {
        let r = build(&ir(), &slot(), "pkg::Colson", &changed(&[]), &[]);
        let cost = &r.commercial.deltas[0];
        assert_eq!(cost.name, "sum?(self, unit_cost) USD");
        // 4 x 10.005 -> 4 x 12.5
        assert_eq!(
            delta_text(cost),
            "sum?(self, unit_cost) USD: 40.02USD -> 50.00USD (+24.9%)"
        );
        assert_eq!(literal(cost.new.as_ref().unwrap()), "50 USD");
        let mass = r.commercial.deltas.last().unwrap();
        assert_eq!(delta_text(mass), "mass?(self): 8kg -> 12kg (+50.0%)");
        assert_eq!(r.commercial.missing[0].parts, ["pkg::Rover.spare"]);
        let moq = r.commercial.moq.as_ref().unwrap();
        assert_eq!(delta_text(moq), "moq pkg::Rover.wheel: 5 -> n/a");
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap_report.rs::write_text
    #[test]
    fn text_prints_sections_five_to_seven() {
        let r = build(
            &ir(),
            &slot(),
            "pkg::Colson",
            &changed(&["pkg::Rover.total"]),
            &[],
        );
        let mut out = Vec::new();
        write_text(&r, &mut out);
        let t = String::from_utf8(out).unwrap();
        assert!(t.contains("5. CAD files to re-sync"));
        assert!(t.contains("Other.SLDPRT: not stale"));
        assert!(t.contains("6. attestations invalidated: 0"));
        assert!(t.contains("7. commercial deltas at pkg::Rover.x"));
        assert!(t.contains("missing unit_cost before the swap: pkg::Rover.spare"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_swap_report.rs::display
    #[test]
    fn money_uses_minor_unit_and_percent_rounds_half_away() {
        let jpy = Amount {
            magnitude: Magnitude::new(Rational::parse_decimal("1234.5").unwrap(), 0),
            dim: Dim::new(cadetta_units::Dimension::currency("JPY"), 0),
        };
        assert_eq!(display(&jpy), "1235JPY");
        let a = Amount {
            magnitude: Magnitude::new(Rational::from_int(8), 0),
            dim: mass(),
        };
        let b = Amount {
            magnitude: Magnitude::new(Rational::from_int(9), 0),
            dim: mass(),
        };
        // +12.5% exactly: rounds to 12.5; and 1/16 = 6.25% -> 6.3% (half away from zero).
        assert_eq!(percent(&a, &b).as_deref(), Some("+12.5%"));
        let c = Amount {
            magnitude: Magnitude::new(Rational::from_int(17), 0),
            dim: mass(),
        };
        let d = Amount {
            magnitude: Magnitude::new(Rational::from_int(16), 0),
            dim: mass(),
        };
        assert_eq!(percent(&d, &c).as_deref(), Some("+6.3%"));
    }
}
