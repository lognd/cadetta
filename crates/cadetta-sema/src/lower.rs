// frob:ticket 01M4SPWQH44JWFXZDNTGEBMMNX
//! Lowering of an evaluated program into the design IR (`docs/spec/ir.md` 3).
//!
//! The lowerer lives in `cadetta-sema`, not `cadetta-ir`, because sema already depends on the
//! IR crate (dependency direction ir <- sema). It reads the evaluator's output and the source
//! text and builds a `cadetta_ir::model::Project` of the design flavour.
//!
//! Decisions made here (written into ir.md 3):
//! - `Value.nominal` is the designed value; `Value.drawn` is the process-compensated emitted
//!   value and exists only under a process rule.
//! - A tolerance becomes one [`AffineTerm`] per source variable. A term's bounds are deviations
//!   from the nominal; the offset of the form's centre from the nominal is carried by the
//!   first term, so the term bounds sum to the form's range. A variable owned by a parameter is
//!   named by that parameter's path; an anonymous variable is named `<member path>.tol<N>`.
//! - A member is `Derived` when its definition reads other members, else `Authored`.
//! - Variant evaluations are supplied by the caller keyed `"<slot>=<alternate>"`; a member
//!   stores only the values that differ from the primary.
//! - Solvers and waivers lower in `lower_solver.rs`; the recorded-result state of a solver comes
//!   from the caller (`LowerInput::solver_states`).
//! - Tables lower in `lower_table.rs`.
//! - Not lowered yet: interfaces, `require` obligations and bindings; parameterized components lower with
//!   valueless members. Datums, points and placements lower in `lower_geom.rs`.

use std::collections::{BTreeMap, BTreeSet};

use cadetta_ir::model::{
    self as ir, AffineTerm, Check, Component, ComponentClass, Exact, Fit, Flavour, Inexact, Member,
    MemberKind, Nominal, PackageId, Part, Project, Provenance, SpanRef, Value, VerdictState,
    Verdicts,
};
use cadetta_ir::path::{Path, PathContext, Seg};
use cadetta_syntax::ast;
use cadetta_syntax::lexer::Span;
use cadetta_units::compare::{Note, Verdict};
use cadetta_units::{Magnitude, Toleranced, VarId};
use tracing::{debug, trace, warn};

use crate::cite::{CiteIndex, effective};
use crate::deps::DepGraph;
use crate::eval::{Evaluated, Evaluation, Quant, Val};
use crate::fits::FitResult;
use crate::process::ProcessOutcome;
use crate::resolve::{CanonPath, MemberSeg, PackageSrc, PartSeg};

/// The text and name of one source file, for spans and expression text.
#[derive(Clone, Copy, Debug)]
pub struct SourceText<'a> {
    /// The file name recorded in spans.
    pub file: &'a str,
    /// The file's text.
    pub text: &'a str,
}

/// An analysed fit and where it lives, supplied by the caller (fits are not in `Evaluation`).
#[derive(Clone, Debug)]
pub struct FitInput {
    /// The fit's canonical path (`PKG::Item.name`).
    pub path: CanonPath,
    /// The hole member.
    pub hole: CanonPath,
    /// The shaft member.
    pub shaft: CanonPath,
    /// The analysis result.
    pub result: FitResult,
    /// The declared required clearance bound, if any.
    pub required: Option<ir::Bound>,
}

/// Everything the lowerer reads.
pub struct LowerInput<'a> {
    /// The package name to lower.
    pub name: &'a str,
    /// The package version.
    pub version: &'a str,
    /// All packages; only the one named `name` is lowered.
    pub packages: &'a [PackageSrc<'a>],
    /// Source text by module path.
    pub sources: &'a BTreeMap<Vec<String>, SourceText<'a>>,
    /// The package directory, for hashing cited files.
    pub package_dir: Option<&'a std::path::Path>,
    /// The primary evaluation.
    pub evaluation: &'a Evaluation,
    /// Process compensation, when the program has processes.
    pub compensation: Option<&'a ProcessOutcome>,
    /// The member dependency graph.
    pub deps: &'a DepGraph,
    /// Variant evaluations keyed `"<slot>=<alternate>"`.
    pub variants: BTreeMap<String, &'a Evaluation>,
    /// Analysed fits.
    pub fits: &'a [FitInput],
    /// Recorded primary result state per solver (`PKG::Comp.solver`); absent means unsolved.
    // frob:ticket 01M4ZJAYRJMG47610S0R0ANADA
    pub solver_states: BTreeMap<CanonPath, SolverState>,
}

/// Converts a canonical path to an IR path; `None` (logged) when it is not well formed.
fn ir_path(p: &CanonPath) -> Option<Path> {
    match Path::parse(&p.to_string(), PathContext::General) {
        Ok(path) => Some(path),
        Err(e) => {
            warn!(path = %p, error = %e, "lower: path is not a valid IR path");
            None
        }
    }
}

/// The member path `comp.name`.
fn member_path(comp: &CanonPath, name: &str) -> CanonPath {
    let mut p = comp.clone();
    p.members.push(MemberSeg::Name(name.to_owned()));
    p
}

/// The source slice of `span` with runs of whitespace collapsed to one space.
fn text_of(src: Option<&SourceText<'_>>, span: Span) -> String {
    let Some(s) = src else {
        return String::new();
    };
    let raw = s.text.get(span.start..span.end).unwrap_or("");
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn span_ref(src: Option<&SourceText<'_>>, span: Span) -> Option<SpanRef> {
    let s = src?;
    Some(SpanRef {
        file: s.file.to_owned(),
        start: u32::try_from(span.start).unwrap_or(u32::MAX),
        end: u32::try_from(span.end).unwrap_or(u32::MAX),
    })
}

/// True when `m` is below zero (sign test only; no float is stored).
fn is_negative(m: &Magnitude) -> bool {
    m.to_f64() < 0.0
}

/// Maps every source variable to the parameter that owns it.
fn var_owners(ev: &Evaluation) -> BTreeMap<u32, CanonPath> {
    let mut owners = BTreeMap::new();
    for (path, e) in &ev.values {
        if let (Some(v), None) = (e.var, &e.alias_of) {
            owners.entry(v.0).or_insert_with(|| path.clone());
        }
        if e.owns_discs && e.alias_of.is_none() {
            disc_owners(path, &e.value, &mut owners);
        }
    }
    owners
}

/// Records the `<point>.position` stem that owns each disc variable of a `[position d]` value.
// frob:ticket 01M50C0NEZM1R9KYFTRJCJY1Z8
fn disc_owners(path: &CanonPath, value: &Val, owners: &mut BTreeMap<u32, CanonPath>) {
    let mut stem_of = |point: CanonPath, cs: &[Val]| {
        let mut stem = point;
        stem.members.push(MemberSeg::Name("position".to_owned()));
        for c in cs {
            if let Val::Quantity(q) = c {
                for v in q.form.disc_vars() {
                    owners.entry(v.0).or_insert_with(|| stem.clone());
                }
            }
        }
    };
    match value {
        Val::Point(cs) => stem_of(path.clone(), cs),
        Val::Array(items) => {
            for (i, item) in (0u64..).zip(items) {
                if let Val::Point(cs) = item {
                    let mut p = path.clone();
                    p.members
                        .push(MemberSeg::Index(cadetta_ir::canon::Index::At(i)));
                    stem_of(p, cs);
                }
            }
        }
        _ => {}
    }
}

/// Builds the IR [`Value`] of a quantity (ir.md 3 `Value`).
fn quant_value(
    ev: &Evaluation,
    owners: &BTreeMap<u32, CanonPath>,
    member: &CanonPath,
    q: &Quant,
    authored: &str,
) -> Value {
    let mut v = Value::new(
        Nominal::Exact(Exact(q.nominal.clone())),
        q.dim().clone(),
        authored,
    );
    v.conservative = q.form.is_conservative();
    let offset = match q.form.center().sub(&q.nominal) {
        cadetta_units::Sum::Exact(m) => m,
        cadetta_units::Sum::Inexact(_) => {
            warn!(path = %member, "lower: centre offset mixes pi exponents; dropped");
            Magnitude::new(cadetta_units::Rational::from_int(0), 0)
        }
    };
    let mut terms = Vec::new();
    for i in 0..ev.var_count {
        let Some(c) = q.form.coefficient(VarId(i)) else {
            continue;
        };
        if c.is_zero() {
            continue;
        }
        let r = if is_negative(c) { c.neg() } else { c.clone() };
        let (lo, hi) = if terms.is_empty() {
            (offset.sub(&r), offset.add(&r))
        } else {
            (
                cadetta_units::Sum::Exact(r.neg()),
                cadetta_units::Sum::Exact(r.clone()),
            )
        };
        let (cadetta_units::Sum::Exact(lo), cadetta_units::Sum::Exact(hi)) = (lo, hi) else {
            warn!(path = %member, "lower: term bounds mix pi exponents; term dropped");
            continue;
        };
        let var = match owners.get(&i) {
            Some(o) => ir_path(o),
            None => ir_path(member).map(|mut p| {
                p.members.push(Seg::Name(format!("tol{}", terms.len())));
                p
            }),
        };
        let Some(var) = var else { continue };
        trace!(path = %member, var = %i, "lower: tolerance term");
        terms.push(AffineTerm {
            var,
            lo: Exact(lo),
            hi: Exact(hi),
        });
    }
    disc_terms(&mut terms, owners, member, q);
    if !terms.is_empty() {
        v.tol = Some(ir::Affine { terms });
    }
    v
}

/// Appends the paired `.u`/`.v` terms of every disc variable of `q` (ir.md 3, geometry.md 3.1).
///
/// Both terms of a pair carry the same symmetric bounds `+-(|a| + |b|)`, which contain the disc
/// radius `sqrt(a^2 + b^2)`; the pair names the shared disc, so the two coordinates of one point
/// correlate through the common stem.
// frob:ticket 01M50C0NEZM1R9KYFTRJCJY1Z8
fn disc_terms(
    terms: &mut Vec<AffineTerm>,
    owners: &BTreeMap<u32, CanonPath>,
    member: &CanonPath,
    q: &Quant,
) {
    for id in q.form.disc_vars() {
        let Some((a, b)) = q.form.disc_coefficients(id) else {
            continue;
        };
        let abs = |m: &Magnitude| if is_negative(m) { m.neg() } else { m.clone() };
        let cadetta_units::Sum::Exact(r) = abs(a).add(&abs(b)) else {
            warn!(path = %member, "lower: disc radius mixes pi exponents; disc dropped");
            continue;
        };
        let stem = match owners.get(&id.0) {
            Some(o) => ir_path(o),
            None => ir_path(member).map(|mut p| {
                p.members.push(Seg::Name(format!("position{}", id.0)));
                p
            }),
        };
        let Some(stem) = stem else { continue };
        for axis in ["u", "v"] {
            let mut var = stem.clone();
            var.members.push(Seg::Name(axis.to_owned()));
            trace!(path = %member, var = %var, "lower: disc term");
            terms.push(AffineTerm {
                var,
                lo: Exact(r.neg()),
                hi: Exact(r.clone()),
            });
        }
    }
}

/// The IR value of an evaluated member, if it is numeric.
fn evaluated_value(
    ev: &Evaluation,
    owners: &BTreeMap<u32, CanonPath>,
    path: &CanonPath,
    e: &Evaluated,
    authored: &str,
) -> Option<Value> {
    match &e.value {
        Val::Quantity(q) => Some(quant_value(ev, owners, path, q, authored)),
        Val::Inexact(i) if i.exact_nominal.is_some() => {
            let m = i.exact_nominal.clone()?;
            let mut v = Value::new(Nominal::Exact(Exact(m)), i.dim.clone(), authored);
            v.conservative = true;
            Some(v)
        }
        Val::Inexact(i) => {
            let root = ir_path(path)?;
            let mut v = Value::new(
                Nominal::Inexact(Inexact {
                    f64: i.nominal,
                    root,
                }),
                i.dim.clone(),
                authored,
            );
            v.conservative = true;
            Some(v)
        }
        other => {
            trace!(path = %path, kind = ?std::mem::discriminant(other), "lower: non-numeric value");
            None
        }
    }
}

#[path = "lower_array.rs"]
mod lower_array;
#[path = "lower_geom.rs"]
mod lower_geom;
#[path = "lower_solver.rs"]
mod lower_solver;
#[path = "lower_table.rs"]
mod lower_table;
pub use lower_solver::{SolverState, lower_waiver};

/// Per-lowering shared state.
struct Cx<'a> {
    input: &'a LowerInput<'a>,
    owners: BTreeMap<u32, CanonPath>,
    variant_owners: BTreeMap<String, BTreeMap<u32, CanonPath>>,
    resolver: crate::resolve::Resolver<'a>,
}

impl Cx<'_> {
    /// A dependency on a solver node, widened to the solver's recorded output members; any other node as is.
    // frob:ticket 01M4YRH3G6F8EGQ5X5CK4JKH27
    fn solver_output_nodes(&self, d: &CanonPath) -> Vec<CanonPath> {
        let outputs: Vec<CanonPath> = self
            .input
            .evaluation
            .values
            .keys()
            .filter(|k| {
                k.parts.is_empty()
                    && d.parts.is_empty()
                    && k.item_only() == d.item_only()
                    && k.members.len() == 2
                    && d.members.len() == 1
                    && k.members[0] == d.members[0]
            })
            .cloned()
            .collect();
        if outputs.is_empty() {
            vec![d.clone()]
        } else {
            trace!(solver = %d, outputs = outputs.len(), "lower: solver dependency widened to its outputs");
            outputs
        }
    }

    /// A dependency path with part-slot instances mapped to the part type's own member.
    // frob:ticket 01M4Z7SF16P01AQQ1XXE2NDJ5J
    fn dep_node(&self, p: CanonPath) -> Option<CanonPath> {
        if p.parts.is_empty() {
            if self.is_underived_fit(&p) {
                debug!(path = %p, "lower: dependency on a fit with component-argument operands dropped");
                return None;
            }
            return Some(self.slot_node(p));
        }
        let d = crate::eval::geom::definition_path(&self.resolver, &p);
        match d {
            None => {
                debug!(path = %p, "lower: dependency through an unresolved part slot dropped");
                None
            }
            // A dependency package's component is not lowered, so its member is no node:
            // the read depends on the slot (and so on the arguments) that instantiates it.
            Some(d) if d.package != self.input.name => {
                let mut slot = p.item_only();
                slot.parts.push(PartSeg {
                    name: p.parts[0].name.clone(),
                    index: None,
                });
                debug!(path = %p, slot = %slot, "lower: dependency on a foreign part member is the slot");
                Some(slot)
            }
            d => d,
        }
    }

    /// True when `p` names a `fit` member that was not derived (its operands read component arguments).
    // frob:ticket 01M4Z7SEXW62CXBGNNHTC5DJ1M
    fn is_underived_fit(&self, p: &CanonPath) -> bool {
        let [MemberSeg::Name(name)] = p.members.as_slice() else {
            return false;
        };
        let is_fit = self.resolver.component(&p.item_only()).is_some_and(|c| {
            c.members
                .iter()
                .any(|m| matches!(&m.kind, ast::MemberKind::Fit(f) if f.name.name == *name))
        });
        // A fit derived inside a parameterized instance carries a per-instance path
        // (`Unit/housing.bore_fit`); it is derived when it maps back to this definition path.
        is_fit
            && !self.input.fits.iter().any(|f| {
                f.path == *p
                    || (!f.path.parts.is_empty()
                        && crate::eval::geom::definition_path(&self.resolver, &f.path).as_ref()
                            == Some(p))
            })
    }

    /// A read of a bare part slot (`Unit.bearing`) is the slot's own `Unit/bearing` node.
    fn slot_node(&self, p: CanonPath) -> CanonPath {
        let [MemberSeg::Name(name)] = p.members.as_slice() else {
            return p;
        };
        let is_slot = self.resolver.component(&p.item_only()).is_some_and(|c| {
            c.members
                .iter()
                .any(|m| matches!(&m.kind, ast::MemberKind::Part(x) if x.name.name == *name))
        });
        if !is_slot {
            return p;
        }
        let mut slot = p.item_only();
        slot.parts.push(PartSeg {
            name: name.clone(),
            index: None,
        });
        trace!(slot = %slot, "lower: dependency on a part slot");
        slot
    }

    fn lower_member(
        &self,
        comp: &CanonPath,
        src: Option<&SourceText<'_>>,
        idx: &CiteIndex<'_>,
        file: &ast::File,
        c: &ast::Component,
        m: &ast::Member,
    ) -> Option<Member> {
        let (name, kind, expr, citation, was) = match &m.kind {
            ast::MemberKind::Param(p) => (
                &p.name,
                MemberKind::Param,
                &p.value,
                p.citation.as_ref(),
                p.was.as_slice(),
            ),
            ast::MemberKind::Let(l) => (&l.name, MemberKind::Let, &l.value, None, &[][..]),
            other => {
                trace!(kind = ?std::mem::discriminant(other), "lower: member kind not lowered here");
                return None;
            }
        };
        let canon = member_path(comp, &name.name);
        let mut out = Member::new(ir_path(&canon)?, kind);
        out.private = m.private;
        let text = text_of(src, expr.span);
        out.expr = Some(text.clone());
        out.doc = (!m.docs.is_empty()).then(|| {
            m.docs
                .iter()
                .map(|d| d.text.trim())
                .collect::<Vec<_>>()
                .join("\n")
        });
        out.span = span_ref(src, m.span);
        out.was = was
            .iter()
            .map(|p| {
                p.segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".")
            })
            .collect();
        out.attrs = crate::attrs::resolve_attrs(
            src.map_or("", |s| s.text),
            &file.inner_attrs,
            &c.inner_attrs,
            &m.attrs,
            &m.trailing,
        );
        out.role = role_from_attrs(&out.attrs);
        out.citation = effective(citation, c.citation.as_ref()).and_then(|cc| {
            match idx.lower_ref(&comp.package, &comp.modules, cc) {
                Ok(r) => Some(r),
                Err(e) => {
                    debug!(path = %canon, error = %e.message, "lower: citation unresolved");
                    None
                }
            }
        });
        let deps: Vec<Path> = self
            .input
            .deps
            .deps_of(&canon)
            .flat_map(|d| self.solver_output_nodes(d))
            .filter_map(|d| self.dep_node(d))
            .filter_map(|d| ir_path(&d))
            .collect();
        out.provenance = if deps.is_empty() {
            Provenance::Authored
        } else {
            Provenance::Derived
        };
        out.deps = deps;

        let ev = self.input.evaluation;
        if let Some(e) = ev.get(&canon) {
            out.analysis = e.analysis || kind == MemberKind::Let;
            let comp_out = self
                .input
                .compensation
                .and_then(|o| o.compensated.get(&canon));
            out.value = match comp_out {
                Some(co) => {
                    let mut v = quant_value(ev, &self.owners, &canon, &co.designed, &text);
                    v.drawn = Some(Exact(co.drawn.nominal.clone()));
                    out.role = Some(co.role.clone());
                    debug!(path = %canon, process = %co.process, "lower: compensated member");
                    Some(v)
                }
                None => evaluated_value(ev, &self.owners, &canon, e, &text),
            };
        } else {
            trace!(path = %canon, "lower: member has no evaluated value");
        }

        out.variants = self.differing_variants(&canon, &text, out.value.as_ref());
        Some(out)
    }

    /// The numeric members of the parameterized instances declared in `comp` (`Box/j.depth`,
    /// `Box/arm[1].depth` for one element of a part array).
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    fn lower_instance_members(&self, comp: &CanonPath) -> Vec<Member> {
        let ev = self.input.evaluation;
        let mut out = Vec::new();
        for (k, e) in &ev.values {
            let is_instance_member =
                k.item_only() == *comp && k.parts.len() == 1 && k.members.len() == 1;
            if !is_instance_member {
                continue;
            }
            let Some(path) = ir_path(k) else { continue };
            let Some(value) = evaluated_value(ev, &self.owners, k, e, "") else {
                continue;
            };
            let mut m = Member::new(path, MemberKind::Param);
            m.private = ev.instance_private.contains(k);
            m.analysis = e.analysis;
            m.value = Some(value);
            m.provenance = Provenance::Derived;
            out.push(m);
        }
        trace!(component = %comp, members = out.len(), "lower: instance members");
        out
    }

    /// The recorded outputs of the solver members of `c` as derived analysis members.
    // frob:ticket 01M4YRH3G6F8EGQ5X5CK4JKH27
    fn lower_solver_outputs(&self, comp: &CanonPath, c: &ast::Component) -> Vec<Member> {
        let ev = self.input.evaluation;
        let solvers: BTreeSet<&str> = c
            .members
            .iter()
            .filter_map(|m| match &m.kind {
                ast::MemberKind::Solver(s) => Some(s.name.name.as_str()),
                _ => None,
            })
            .collect();
        let mut out = Vec::new();
        for (k, e) in &ev.values {
            let is_output = k.item_only() == *comp
                && k.parts.is_empty()
                && matches!(k.members.as_slice(),
                    [MemberSeg::Name(s), MemberSeg::Name(_)] if solvers.contains(s.as_str()));
            if !is_output {
                continue;
            }
            let Some(path) = ir_path(k) else { continue };
            let Some(value) = evaluated_value(ev, &self.owners, k, e, "") else {
                continue;
            };
            let mut m = Member::new(path, MemberKind::Param);
            m.analysis = true;
            m.value = Some(value);
            m.provenance = Provenance::Derived;
            out.push(m);
        }
        trace!(component = %comp, members = out.len(), "lower: solver outputs");
        out
    }

    /// The variant values of `canon` that differ from the primary value.
    fn differing_variants(
        &self,
        canon: &CanonPath,
        text: &str,
        primary: Option<&Value>,
    ) -> Option<BTreeMap<String, Value>> {
        let mut variants = BTreeMap::new();
        for (key, vev) in &self.input.variants {
            let Some(e) = vev.get(canon) else { continue };
            let owners = self.variant_owners.get(key).unwrap_or(&self.owners);
            let v = evaluated_value(vev, owners, canon, e, text);
            if let Some(v) = v
                && Some(&v) != primary
            {
                debug!(path = %canon, variant = %key, "lower: variant value differs");
                variants.insert(key.clone(), v);
            }
        }
        (!variants.is_empty()).then_some(variants)
    }

    fn lower_part(
        &self,
        comp: &CanonPath,
        src: Option<&SourceText<'_>>,
        p: &ast::Part,
    ) -> Option<Part> {
        // A slot is a `/part` segment (`pkg::Assembly/wheel`), not a `.member`.
        let mut path = comp.item_only();
        path.parts.push(PartSeg {
            name: p.name.name.clone(),
            index: None,
        });
        debug!(slot = %path, "lower: part slot path");
        let ctor_text = |c: &ast::Ctor| {
            c.path
                .segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join("::")
        };
        let mut args = BTreeMap::new();
        for (i, a) in p.args.iter().flatten().enumerate() {
            let key = a
                .name
                .as_ref()
                .map_or_else(|| i.to_string(), |n| n.name.clone());
            args.insert(key, text_of(src, a.value.span));
        }
        Some(Part {
            path: ir_path(&path)?,
            type_name: text_of(src, p.ty.span),
            count: p
                .count
                .as_ref()
                .and_then(|c| text_of(src, c.span).parse().ok()),
            primary: p.primary.as_ref().map(ctor_text).unwrap_or_default(),
            alternates: p.alts.iter().map(ctor_text).collect(),
            args,
        })
    }

    fn lower_fit(&self, f: &FitInput) -> Option<Fit> {
        let t: &Toleranced = &f.result.clearance;
        let nominal = t.nominal().clone();
        let mut v = Value::new(
            Nominal::Exact(Exact(nominal)),
            t.dim().clone(),
            String::new(),
        );
        let mut var = ir_path(&f.path)?;
        var.members.push(Seg::Name("clearance".to_owned()));
        v.tol = Some(ir::Affine {
            terms: vec![AffineTerm {
                var,
                lo: Exact(t.lower().clone()),
                hi: Exact(t.upper().clone()),
            }],
        });
        Some(Fit {
            path: ir_path(&f.path)?,
            hole: ir_path(&f.hole)?,
            shaft: ir_path(&f.shaft)?,
            clearance: v,
            fit_type: f.result.fit_type.name().to_owned(),
            required: f.required.clone(),
        })
    }
}

/// The tolerance role a member plays: bare `#[hole]`/`#[shaft]` or `#[role(hole)]`/`#[role(shaft)]`.
fn role_from_attrs(attrs: &BTreeMap<String, String>) -> Option<String> {
    let role = ["hole", "shaft"]
        .iter()
        .find(|r| attrs.contains_key(**r))
        .map(|r| (*r).to_owned())
        .or_else(|| {
            attrs
                .get("role")
                .map(|v| v.trim())
                .filter(|v| matches!(*v, "hole" | "shaft"))
                .map(str::to_owned)
        });
    trace!(role = ?role, "lower: member role resolved");
    role
}

fn class_of(c: ast::ComponentClass) -> ComponentClass {
    match c {
        ast::ComponentClass::Ots => ComponentClass::Ots,
        ast::ComponentClass::Model => ComponentClass::Model,
        ast::ComponentClass::Assembly => ComponentClass::Assembly,
    }
}

/// Folds the instance verdicts of one check into one state (any fail fails).
fn fold_verdicts(results: &[&crate::eval::checks::CheckResult]) -> VerdictState {
    let mut state = VerdictState::Pass;
    for r in results {
        match r.verdict {
            Verdict::Violates => return VerdictState::Fail,
            Verdict::Undecided => state = VerdictState::Unknown,
            Verdict::Holds => {}
        }
    }
    state
}

fn group_checks(ev: &Evaluation) -> BTreeMap<CanonPath, Vec<&crate::eval::checks::CheckResult>> {
    let mut m: BTreeMap<CanonPath, Vec<&crate::eval::checks::CheckResult>> = BTreeMap::new();
    for r in ev.checks.iter().filter(|r| !r.obligation) {
        m.entry(r.path.clone()).or_default().push(r);
    }
    m
}

/// Lowers the evaluated program of one package into a design-flavour IR project.
pub fn lower(input: &LowerInput<'_>) -> Project {
    let mut project = Project::new(
        Flavour::Design,
        PackageId {
            name: input.name.to_owned(),
            version: input.version.to_owned(),
        },
    );
    let cx = Cx {
        input,
        resolver: crate::resolve::Resolver::new(input.packages.to_vec()).0,
        owners: var_owners(input.evaluation),
        variant_owners: input
            .variants
            .iter()
            .map(|(k, e)| (k.clone(), var_owners(e)))
            .collect(),
    };
    let mut modules = BTreeSet::new();
    let mut cited: BTreeSet<String> = BTreeSet::new();
    let idx = CiteIndex::new(input.packages);
    for pkg in input.packages.iter().filter(|p| p.name == input.name) {
        for module in &pkg.modules {
            if !module.path.is_empty() {
                modules.insert(module.path.join("::"));
            }
            let src = input.sources.get(&module.path);
            for c in module.file.components() {
                let canon = CanonPath::item(&pkg.name, &module.path, &c.name.name);
                let Some(path) = ir_path_item(&canon) else {
                    continue;
                };
                let mut comp = Component {
                    path,
                    class: class_of(c.class),
                    params_sig: (!c.params.is_empty()).then(|| {
                        c.params
                            .iter()
                            .map(|p| p.name.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }),
                    implements: c
                        .implements
                        .iter()
                        .map(|p| {
                            p.segments
                                .iter()
                                .map(|s| s.name.as_str())
                                .collect::<Vec<_>>()
                                .join("::")
                        })
                        .collect(),
                    citation: c
                        .citation
                        .as_ref()
                        .and_then(|cc| idx.lower_ref(&pkg.name, &module.path, cc).ok()),
                    attrs: BTreeMap::new(),
                    members: Vec::new(),
                    parts: Vec::new(),
                    fits: Vec::new(),
                    datums: Vec::new(),
                    points: Vec::new(),
                    placements: Vec::new(),
                    span: None,
                    digest: None,
                };
                for m in &c.members {
                    if let Some(mem) = cx.lower_member(&canon, src, &idx, module.file, c, m) {
                        cited.extend(mem.citation.iter().filter_map(|r| r.cite.clone()));
                        comp.members.push(mem);
                    } else if let ast::MemberKind::Part(p) = &m.kind {
                        comp.parts.extend(cx.lower_part(&canon, src, p));
                    }
                }
                // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
                comp.members.extend(cx.lower_instance_members(&canon));
                // frob:ticket 01M4VYZS8VGT1RASS407Y8K809
                comp.members.extend(cx.lower_instance_geometry(&canon));
                // frob:ticket 01M4Z92C25BYMP5JYSX9WDH9XE
                comp.members
                    .extend(cx.lower_array_members(&canon, &pkg.name, &module.path, c));
                // frob:ticket 01M4YRH3G6F8EGQ5X5CK4JKH27
                comp.members.extend(cx.lower_solver_outputs(&canon, c));
                cx.lower_geometry(&canon, src, c, &mut comp);
                // frob:ticket 01M4ZJAYRJMG47610S0R0ANADA
                let solvers = cx.lower_solvers(&canon, src, &idx, (&pkg.name, &module.path), c);
                cited.extend(
                    solvers
                        .iter()
                        .filter_map(|s| s.citation.as_ref()?.cite.clone()),
                );
                project.solvers.extend(solvers);
                comp.fits = input
                    .fits
                    .iter()
                    .filter(|f| f.path.item_only() == canon)
                    .filter_map(|f| cx.lower_fit(f))
                    .collect();
                cited.extend(comp.citation.iter().filter_map(|r| r.cite.clone()));
                debug!(component = %canon, members = comp.members.len(), "lower: component");
                project.components.push(comp);
            }
            // frob:ticket 01M5187T6Y9B3SMBS9DMPG9VFX
            for item in module.file.items() {
                if let ast::ItemKind::Table(t) = &item.kind {
                    let canon = CanonPath::item(&pkg.name, &module.path, &t.name.name);
                    let table = cx.lower_table(&canon, src, &idx, (&pkg.name, &module.path), t);
                    if let Some(table) = table {
                        cited.extend(table.citation.iter().filter_map(|r| r.cite.clone()));
                        project.tables.push(table);
                    }
                }
            }
            // frob:ticket 01M4ZJAYRJMG47610S0R0ANADA
            for item in module.file.items() {
                if let ast::ItemKind::Waive(w) = &item.kind {
                    match lower_waiver(w) {
                        Ok(w) => project.waivers.push(w),
                        Err(e) => lower_solver::skip_waiver(w, &e),
                    }
                }
            }
        }
    }
    let primary = group_checks(input.evaluation);
    let variant_checks: BTreeMap<&String, _> = input
        .variants
        .iter()
        .map(|(k, e)| (k, group_checks(e)))
        .collect();
    for (path, results) in &primary {
        let Some(p) = ir_path(path) else { continue };
        let src = input.sources.get(&path.modules);
        let mut verdicts = Verdicts {
            primary: Some(fold_verdicts(results)),
            ..Verdicts::default()
        };
        for (k, vc) in &variant_checks {
            if let Some(r) = vc.get(path) {
                verdicts
                    .per_alternate
                    .insert((*k).clone(), fold_verdicts(r));
            }
        }
        let conservative = results
            .iter()
            .any(|r| r.notes.contains(&Note::Conservative));
        project.checks.push(Check {
            path: p,
            expr: text_of(src, results[0].span),
            verdicts,
            conservative,
            waived_for: Vec::new(),
        });
    }
    project.set_modules(modules.into_iter().collect());
    project.citations = cited
        .iter()
        .filter_map(|p| idx.citation(p, input.name, input.package_dir))
        .collect();
    debug!(
        components = project.components.len(),
        checks = project.checks.len(),
        "lower: project built"
    );
    project
}

/// The IR path of a component (an item path has no member tail, so its tail is dropped).
fn ir_path_item(p: &CanonPath) -> Option<Path> {
    let mut s = format!("{}::", p.package);
    for m in &p.modules {
        s.push_str(m);
        s.push_str("::");
    }
    s.push_str(&p.item);
    s.push_str(".x");
    match Path::parse(&s, PathContext::General) {
        Ok(mut path) => {
            path.members.clear();
            Some(path)
        }
        Err(e) => {
            warn!(path = %s, error = %e, "lower: component path is not a valid IR path");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deps::Renames;
    use crate::eval::Evaluator;
    use crate::fits::analyze;
    use crate::process::Processes;
    use crate::resolve::{ModuleSrc, Resolver};
    use cadetta_ir::geometry::Op;
    use cadetta_ir::model::Digest;
    use cadetta_syntax::ast::{FitSpec, Ident};
    use cadetta_syntax::parser::parse;
    use cadetta_units::{Rational, UnitTable};

    const PROCESS_SRC: &str = "process Fdm @measured:\"cal\" {\n  \
        hole_comp = 0.2mm [+-0.05mm]\n  \
        rule hole(d: Length) -> Length = d + hole_comp\n}\n\
        model Case {\n  #![process(Fdm)]\n  insert_hole = 3.4mm [+-0.1mm] #[hole]\n}\n";

    const SRC: &str = "model Axle {\n  /// the seat\n  seat = 20mm [+0.021mm -0]\n  \
        wall = 2.5mm [+-0.1mm]\n  od = seat + 2 * wall\n  let slack = od - seat\n  let stack = seat + 2 * wall\n  \
        plain = 7mm\n  check wide: od > 24mm\n}\n";

    struct Built {
        project: Project,
        ev: Evaluation,
    }

    fn evaluate(src: &str) -> (Evaluation, Option<ProcessOutcome>, DepGraph) {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        let pkgs = vec![PackageSrc {
            defaults: Default::default(),
            name: "p".to_owned(),
            deps: Vec::new(),
            modules: vec![ModuleSrc {
                path: Vec::new(),
                file: &p.file,
                enums: &[],
            }],
        }];
        let (resolver, errs) = Resolver::new(pkgs.clone());
        assert!(errs.is_empty());
        let units = UnitTable::builtin();
        let mut ev = Evaluator::new(&resolver, &units, &pkgs);
        let procs = Processes::compile(&mut ev, &pkgs);
        ev.set_processes(procs.clone());
        let evaluation = ev.evaluate();
        let outcome = procs.apply(&evaluation);
        let renames = Renames::collect(&resolver, &pkgs);
        let graph = DepGraph::build(&resolver, &renames, &pkgs);
        (evaluation, Some(outcome), graph)
    }

    fn build(src: &str, variants: &[(&str, &str)], fits: &[FitInput]) -> Built {
        build_with(src, variants, fits, BTreeMap::new())
    }

    fn build_with(
        src: &str,
        variants: &[(&str, &str)],
        fits: &[FitInput],
        solver_states: BTreeMap<CanonPath, SolverState>,
    ) -> Built {
        let (evaluation, outcome, graph) = evaluate(src);
        let vevals: Vec<(String, Evaluation)> = variants
            .iter()
            .map(|(k, s)| ((*k).to_owned(), evaluate(s).0))
            .collect();
        let p = parse(src);
        let pkgs = vec![PackageSrc {
            defaults: Default::default(),
            name: "p".to_owned(),
            deps: Vec::new(),
            modules: vec![ModuleSrc {
                path: Vec::new(),
                file: &p.file,
                enums: &[],
            }],
        }];
        let mut sources = BTreeMap::new();
        sources.insert(
            Vec::new(),
            SourceText {
                file: "lib.cdt",
                text: src,
            },
        );
        let input = LowerInput {
            name: "p",
            version: "0.1.0",
            packages: &pkgs,
            sources: &sources,
            package_dir: None,
            evaluation: &evaluation,
            compensation: outcome.as_ref(),
            deps: &graph,
            variants: vevals.iter().map(|(k, e)| (k.clone(), e)).collect(),
            fits,
            solver_states,
        };
        let project = lower(&input);
        Built {
            project,
            ev: evaluation,
        }
    }

    fn member<'a>(b: &'a Built, comp: &str, name: &str) -> &'a Member {
        let c = b
            .project
            .components
            .iter()
            .find(|c| c.path.item == comp)
            .expect("component");
        c.members
            .iter()
            .find(|m| m.path.members == vec![Seg::Name(name.to_owned())])
            .expect("member")
    }

    const SOLVER_SRC: &str = "model Frame { span = 2m\n  \
        solver beam {\n    script \"beam.sh\"\n    in len = span\n    out defl: Length\n  }\n}\n\
        waive W001 on Frame.* { because \"known\" expires 2030-01-01 }\n\
        waive W002 on Frame.span for alt p::Frame/s=p::Alt { because \"alt\" }\n\
        waive W003 on Frame.span for alt a=b { because \"bad alt\" }\n";

    // frob:ticket 01M4ZJAYRJMG47610S0R0ANADA
    // frob:tests crates/cadetta-sema/src/lower_solver.rs::lower_solvers kind=unit
    #[test]
    fn a_solver_lowers_with_script_inputs_outputs_and_deps() {
        let b = build(SOLVER_SRC, &[], &[]);
        assert_eq!(b.project.solvers.len(), 1);
        let s = &b.project.solvers[0];
        assert_eq!(s.path.to_string(), "p::Frame.beam");
        assert_eq!(s.script, "beam.sh");
        assert_eq!(s.inputs.get("len").map(String::as_str), Some("span"));
        assert_eq!(
            s.outputs.get("defl").map(String::as_str),
            Some("p::Frame.beam.defl")
        );
        assert_eq!(
            s.deps.iter().map(|d| d.to_string()).collect::<Vec<_>>(),
            ["p::Frame.span"]
        );
        assert!(s.result_digest.is_none() && !s.stale);
    }

    // frob:ticket 01M4ZJAYRJMG47610S0R0ANADA
    // frob:tests crates/cadetta-sema/src/lower_solver.rs::lower_solvers kind=unit
    #[test]
    fn a_solver_takes_its_recorded_result_state_from_the_caller() {
        let key = member_path(&CanonPath::item("p", &[], "Frame"), "beam");
        let state = SolverState {
            result_digest: Some(Digest("ab".to_owned())),
            stale: true,
        };
        let b = build_with(SOLVER_SRC, &[], &[], BTreeMap::from([(key, state)]));
        let s = &b.project.solvers[0];
        assert_eq!(s.result_digest, Some(Digest("ab".to_owned())));
        assert!(s.stale);
    }

    // frob:ticket 01M4VNC6QGY8T20VW77QCK9BKX
    // frob:tests crates/cadetta-sema/src/lower_solver.rs::lower_solvers kind=unit
    #[test]
    fn a_solver_citation_lowers_to_a_cite_ref_and_is_not_an_input() {
        let src = "cite fea: web { title = \"FEA\" }\n\
            model Frame { span = 2m\n  \
            solver beam @fea:\"p.4\" {\n    script \"beam.sh\"\n    in len = span\n    out defl: Length\n  }\n}\n";
        let b = build(src, &[], &[]);
        let s = &b.project.solvers[0];
        let c = s.citation.as_ref().expect("solver citation");
        assert_eq!(c.cite.as_deref(), Some("p::fea"));
        assert_eq!(c.locator.as_deref(), Some("p.4"));
        assert_eq!(s.inputs.len(), 1, "the citation is not an input");
        assert!(s.deps.iter().all(|d| d.to_string() != "p::fea"));
        assert!(b.project.citations.iter().any(|c| c.path == "p::fea"));
        let plain = build(SOLVER_SRC, &[], &[]);
        assert!(plain.project.solvers[0].citation.is_none());
    }

    const TABLE_SRC: &str = "cite iso: web { title = \"ISO\" }\n\
        enum Th { A, B }\n\
        table T @iso:\"t.1\" { key t: Th\n cols major: Length, fine: Ratio\n \
        Th::A => 5mm, 0.5\n Th::B => 6mm, 0.25 }\n\
        table D { key d: Length\n 3mm\n 4mm }\n";

    // frob:ticket 01M5187T6Y9B3SMBS9DMPG9VFX
    // frob:tests crates/cadetta-sema/src/lower_table.rs::lower_table kind=unit
    #[test]
    fn a_table_lowers_with_its_columns_rows_and_cell_values() {
        let b = build(TABLE_SRC, &[], &[]);
        assert_eq!(b.project.tables.len(), 2);
        let t = &b.project.tables[0];
        assert_eq!(t.path.to_string(), "p::T");
        assert_eq!((t.key.name.as_str(), t.key.type_name.as_str()), ("t", "Th"));
        let cols: Vec<_> = t
            .cols
            .iter()
            .map(|c| (c.name.as_str(), c.type_name.as_str()))
            .collect();
        assert_eq!(cols, [("major", "Length"), ("fine", "Ratio")]);
        assert_eq!(t.rows.len(), 2);
        assert_eq!(
            t.rows[1].key,
            cadetta_ir::entities::Cell::Variant("B".to_owned())
        );
        let cadetta_ir::entities::Cell::Value(v) = &t.rows[1].cells[0] else {
            panic!("a quantity cell");
        };
        assert_eq!(v.authored, "6mm");
        assert_eq!(
            v.nominal,
            Nominal::Exact(Exact(Magnitude::new(Rational::new(6, 1000).unwrap(), 0)))
        );
    }

    // frob:ticket 01M5187T6Y9B3SMBS9DMPG9VFX
    // frob:tests crates/cadetta-sema/src/lower_table.rs::lower_table kind=unit
    #[test]
    fn a_key_only_table_has_quantity_keys_and_no_cells() {
        let b = build(TABLE_SRC, &[], &[]);
        let d = &b.project.tables[1];
        assert!(d.cols.is_empty() && d.citation.is_none());
        assert_eq!(d.rows.len(), 2);
        assert!(d.rows.iter().all(|r| r.cells.is_empty()));
        assert!(matches!(
            d.rows[0].key,
            cadetta_ir::entities::Cell::Value(_)
        ));
    }

    // frob:ticket 01M5187T6Y9B3SMBS9DMPG9VFX
    // frob:tests crates/cadetta-sema/src/lower_table.rs::lower_table kind=unit
    #[test]
    fn a_table_citation_lowers_to_a_cite_ref() {
        let b = build(TABLE_SRC, &[], &[]);
        let c = b.project.tables[0]
            .citation
            .as_ref()
            .expect("table citation");
        assert_eq!(c.cite.as_deref(), Some("p::iso"));
        assert_eq!(c.locator.as_deref(), Some("t.1"));
        assert!(b.project.citations.iter().any(|c| c.path == "p::iso"));
    }

    // frob:ticket 01M4ZJAYRJMG47610S0R0ANADA
    // frob:tests crates/cadetta-sema/src/lower_solver.rs::lower_waiver kind=unit
    #[test]
    fn waivers_lower_with_expiry_and_alternate() {
        let b = build(SOLVER_SRC, &[], &[]);
        let w = &b.project.waivers;
        assert_eq!(w.len(), 2, "the waiver with a bad `for alt` is skipped");
        assert_eq!(
            (w[0].code.as_str(), w[0].glob.as_str()),
            ("W001", "Frame.*")
        );
        assert_eq!(w[0].because, "known");
        assert_eq!(w[0].expires.as_deref(), Some("2030-01-01"));
        assert!(w[0].for_alt.is_none());
        assert_eq!(
            w[1].for_alt.as_ref().map(|v| v.to_string()),
            Some("p::Frame/s=p::Alt".to_owned())
        );
    }

    // frob:ticket 01M4W47VTBXFMC70Q2R137W4EE
    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    #[test]
    fn part_slot_lowers_with_a_slash_segment() {
        let src = "pub interface Caster {\n  diameter: Length\n}\n\
            pub ots Good: Caster {\n  diameter = 5in\n}\n\
            pub ots Bad: Caster {\n  diameter = 5in\n}\n\
            pub assembly Rover {\n  part wheel[4]: Caster = Good alt [Bad]\n}\n";
        let b = build(src, &[], &[]);
        let rover = b
            .project
            .components
            .iter()
            .find(|c| c.path.item == "Rover")
            .expect("Rover");
        let paths: Vec<String> = rover.parts.iter().map(|p| p.path.to_string()).collect();
        assert_eq!(paths, ["p::Rover/wheel"]);
    }

    // frob:ticket 01M4Z7SF16P01AQQ1XXE2NDJ5J
    // frob:tests crates/cadetta-sema/src/lower.rs::slot_node kind=unit
    #[test]
    fn a_read_of_a_part_slot_depends_on_the_slot_node() {
        let src = "pub interface Caster {\n  diameter: Length\n}\n\
            pub ots Good: Caster {\n  diameter = 5in\n}\n\
            pub assembly Rover {\n  part wheel: Caster = Good\n  \
            reach = wheel.diameter\n}\n";
        let b = build(src, &[], &[]);
        let reach = member(&b, "Rover", "reach");
        let deps: Vec<String> = reach.deps.iter().map(|d| d.to_string()).collect();
        assert!(deps.iter().all(|d| !d.ends_with(".wheel")), "{deps:?}");
    }

    // frob:ticket 01M4Z7SEXW62CXBGNNHTC5DJ1M
    // frob:tests crates/cadetta-sema/src/lower.rs::is_underived_fit kind=unit
    #[test]
    fn fit_on_component_argument_operand_is_not_a_dependency() {
        let src = "pub interface Motor {\n  pilot_dia: Length\n}\n\
            pub ots M1: Motor {\n  pilot_dia = 12mm [-0.006mm -0.017mm]\n}\n\
            pub model Plate(motor: Motor) {\n  fit pilot = H7 on motor.pilot_dia\n  \
            pilot_bore = pilot.hole\n}\n";
        let b = build(src, &[], &[]);
        let plate = b
            .project
            .components
            .iter()
            .find(|c| c.path.item == "Plate")
            .expect("Plate");
        for m in &plate.members {
            assert!(
                m.deps.iter().all(|d| !d.to_string().ends_with(".pilot")),
                "{:?}",
                m.deps
            );
        }
    }

    // frob:ticket 01M50XZ0F7YNSYEDEJRVWWVNS4
    // frob:tests crates/cadetta-sema/src/lower.rs::is_underived_fit kind=unit
    #[test]
    fn fit_derived_in_an_instance_is_a_dependency_of_the_definition() {
        let src = "pub interface Motor {\n  pilot_dia: Length\n}\n\
            pub ots M1: Motor {\n  pilot_dia = 12mm [-0.006mm -0.017mm]\n}\n\
            pub model Plate(motor: Motor) {\n  fit pilot = H7 on motor.pilot_dia\n  \
            pilot_bore = pilot.hole\n}\n\
            pub assembly Rig {\n  part m: Motor = M1\n  part plate: Plate(motor = m)\n}\n";
        let mut path = CanonPath::item("p", &[], "Rig");
        path.parts.push(PartSeg {
            name: "plate".to_owned(),
            index: None,
        });
        path.members.push(MemberSeg::Name("pilot".to_owned()));
        let spec = FitSpec::Class(Ident {
            name: "H7".into(),
            span: Span::default(),
        });
        let dim =
            cadetta_units::Dim::new(cadetta_units::Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0);
        let nominal = Toleranced::symmetric(
            dim.clone(),
            Magnitude::new(Rational::new(12, 1000).unwrap(), 0),
            (dim, Magnitude::new(Rational::from_int(0), 0)),
        )
        .unwrap();
        let fit = FitInput {
            path: path.clone(),
            hole: path.clone(),
            shaft: path,
            result: analyze(&spec, &nominal, None, None).expect("fit"),
            required: None,
        };
        let b = build(src, &[], &[fit]);
        let bore = member(&b, "Plate", "pilot_bore");
        assert!(
            bore.deps.iter().any(|d| d.to_string().ends_with(".pilot")),
            "{:?}",
            bore.deps
        );
    }

    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    #[test]
    fn derived_member_has_derived_provenance_and_deps() {
        let b = build(SRC, &[], &[]);
        assert!(b.ev.diagnostics.is_empty(), "{:?}", b.ev.diagnostics);
        let od = member(&b, "Axle", "od");
        assert_eq!(od.provenance, Provenance::Derived);
        let deps: Vec<String> = od.deps.iter().map(|d| d.to_string()).collect();
        assert!(deps.iter().any(|d| d.ends_with(".seat")), "{deps:?}");
        assert!(deps.iter().any(|d| d.ends_with(".wall")), "{deps:?}");
        assert_eq!(od.expr.as_deref(), Some("seat + 2 * wall"));
        assert_eq!(member(&b, "Axle", "seat").provenance, Provenance::Authored);
    }

    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    #[test]
    fn example_lowers_components_fits_and_checks() {
        let f = |n: &str| {
            let mut c = CanonPath::item("p", &[], "Axle");
            c.members.push(MemberSeg::Name(n.to_owned()));
            c
        };
        let first = Toleranced::symmetric(
            cadetta_units::Dim::new(cadetta_units::Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0),
            Magnitude::new(Rational::new(20, 1000).unwrap(), 0),
            (
                cadetta_units::Dim::new(
                    cadetta_units::Dimension::from_si([1, 0, 0, 0, 0, 0, 0]),
                    0,
                ),
                Magnitude::new(Rational::from_int(0), 0),
            ),
        )
        .unwrap();
        let spec = FitSpec::Pair(
            Ident {
                name: "H7".into(),
                span: Span::default(),
            },
            Ident {
                name: "p6".into(),
                span: Span::default(),
            },
        );
        let fit = FitInput {
            path: f("seat_fit"),
            hole: f("seat_fit.hole"),
            shaft: f("seat_fit.shaft"),
            result: analyze(&spec, &first, None, None).expect("fit"),
            required: None,
        };
        let b = build(SRC, &[], &[fit]);
        let comp = &b.project.components[0];
        assert_eq!(comp.class, ComponentClass::Model);
        assert_eq!(comp.fits.len(), 1);
        assert_eq!(comp.fits[0].fit_type, "interference");
        assert_eq!(b.project.checks.len(), 1);
        assert_eq!(
            b.project.checks[0].verdicts.primary,
            Some(VerdictState::Pass)
        );
        assert!(b.project.checks[0].expr.contains("od > 24mm"));
        assert_eq!(b.project.flavour, Flavour::Design);
    }

    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    // frob:ticket 01M4Y772CY1967MXZ9T5T8FKPM
    #[test]
    fn file_scope_fit_lowers_into_its_owner_component() {
        let src = "model Bore { d = 22mm [+0.021mm -0] }\n\
                   model Pin { d = 22mm [+0.0mm -0.009mm] }\n\
                   fit press = Bore.d / Pin.d\n";
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        let pkgs = vec![PackageSrc {
            defaults: Default::default(),
            name: "p".to_owned(),
            deps: Vec::new(),
            modules: vec![ModuleSrc {
                path: Vec::new(),
                file: &p.file,
                enums: &[],
            }],
        }];
        let (resolver, errs) = Resolver::new(pkgs.clone());
        assert!(errs.is_empty());
        let units = UnitTable::builtin();
        let mut ev = Evaluator::new(&resolver, &units, &pkgs);
        let ranges = crate::fitderive::eval_ranges(&mut ev, &pkgs);
        let evaluation = ev.evaluate();
        let fits = crate::fitderive::derive_fits(&resolver, &evaluation, &ranges, &pkgs);
        let b = build(src, &[], &fits);
        let bore = b.project.components.iter().find(|c| c.path.item == "Bore");
        let pin = b.project.components.iter().find(|c| c.path.item == "Pin");
        assert_eq!(bore.expect("Bore").fits.len(), 1);
        assert_eq!(bore.expect("Bore").fits[0].path.members.len(), 1);
        assert!(pin.expect("Pin").fits.is_empty());
    }

    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    #[test]
    fn let_is_analysis() {
        let b = build(SRC, &[], &[]);
        let l = member(&b, "Axle", "slack");
        assert!(l.analysis);
        assert_eq!(l.kind, MemberKind::Let);
        assert!(!member(&b, "Axle", "od").analysis);
    }

    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    #[test]
    fn variants_store_only_differing_values() {
        let alt = SRC.replace("wall = 2.5mm", "wall = 3mm");
        let b = build(SRC, &[("Axle/wall=Thick", alt.as_str())], &[]);
        let wall = member(&b, "Axle", "wall");
        let v = wall.variants.as_ref().expect("wall differs");
        assert!(v.contains_key("Axle/wall=Thick"));
        assert!(member(&b, "Axle", "plain").variants.is_none());
        assert!(member(&b, "Axle", "od").variants.is_some());
        assert!(
            b.project.checks[0]
                .verdicts
                .per_alternate
                .contains_key("Axle/wall=Thick")
        );
    }

    // frob:tests crates/cadetta-sema/src/lower.rs::role_from_attrs kind=unit
    #[test]
    fn role_attribute_names_hole_or_shaft() {
        let mk = |k: &str, v: &str| BTreeMap::from([(k.to_owned(), v.to_owned())]);
        assert_eq!(role_from_attrs(&mk("hole", "")).as_deref(), Some("hole"));
        assert_eq!(
            role_from_attrs(&mk("role", "hole")).as_deref(),
            Some("hole")
        );
        assert_eq!(
            role_from_attrs(&mk("role", "shaft")).as_deref(),
            Some("shaft")
        );
        assert_eq!(role_from_attrs(&mk("role", "other")), None);
        assert_eq!(role_from_attrs(&BTreeMap::new()), None);
    }

    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    #[test]
    fn compensated_member_keeps_designed_nominal_and_drawn() {
        let b = build(PROCESS_SRC, &[], &[]);
        let m = member(&b, "Case", "insert_hole");
        let v = m.value.as_ref().expect("value");
        let mm = |n: i64, d: i64| Magnitude::new(Rational::new(n, d).unwrap(), 0);
        assert_eq!(v.nominal, Nominal::Exact(Exact(mm(34, 10_000))));
        assert_eq!(v.drawn, Some(Exact(mm(36, 10_000))));
        assert_eq!(m.role.as_deref(), Some("hole"));
        assert!(member(&b, "Case", "insert_hole").value.is_some());
    }

    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    #[test]
    fn derived_tolerance_is_an_affine_of_source_variables() {
        let b = build(SRC, &[], &[]);
        let seat = member(&b, "Axle", "seat").value.as_ref().expect("seat");
        let st = seat.tol.as_ref().expect("seat tol");
        assert_eq!(st.terms.len(), 1);
        let mm = |n: i64, d: i64| Magnitude::new(Rational::new(n, d).unwrap(), 0);
        // +0.021/-0: centre offset 0.0105mm, radius 0.0105mm.
        assert_eq!(st.terms[0].lo, Exact(mm(0, 1)));
        assert_eq!(st.terms[0].hi, Exact(mm(21, 1_000_000)));
        // Derived parameters keep their nominal only; a `let` keeps the analysis form.
        let od = member(&b, "Axle", "stack").value.as_ref().expect("stack");
        let ot = od.tol.as_ref().expect("stack tol");
        let vars: Vec<String> = ot.terms.iter().map(|t| t.var.to_string()).collect();
        assert!(vars.iter().any(|d| d.ends_with(".seat")), "{vars:?}");
        assert!(vars.iter().any(|d| d.ends_with(".wall")), "{vars:?}");
        assert!(
            member(&b, "Axle", "plain")
                .value
                .as_ref()
                .unwrap()
                .tol
                .is_none()
        );
    }

    const CITED: &str = "cite zeta: web { url = \"https://x.example/z\" }\n\
        cite alpha: web { title = \"A\" }\n\
        pub ots Part @zeta {\n  a = 1mm @alpha:\"p.1\"\n  b = 2mm\n  c = 3mm @\"note\"\n}\n";

    // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    #[test]
    fn citations_are_sorted_and_member_citation_replaces_the_components() {
        let b = build(CITED, &[], &[]);
        let paths: Vec<&str> = b
            .project
            .citations
            .iter()
            .map(|c| c.path.as_str())
            .collect();
        assert_eq!(paths, ["p::alpha", "p::zeta"]);
        let comp = &b.project.components[0];
        assert_eq!(
            comp.citation.as_ref().unwrap().cite.as_deref(),
            Some("p::zeta")
        );
        let a = member(&b, "Part", "a").citation.as_ref().unwrap();
        assert_eq!(a.cite.as_deref(), Some("p::alpha"));
        assert_eq!(a.locator.as_deref(), Some("p.1"));
        // an uncited member inherits the component's citation
        let inherited = member(&b, "Part", "b").citation.as_ref().unwrap();
        assert_eq!(inherited.cite.as_deref(), Some("p::zeta"));
        let c = member(&b, "Part", "c").citation.as_ref().unwrap();
        assert_eq!(c.text.as_deref(), Some("note"));
        assert!(c.cite.is_none());
    }

    // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    #[test]
    fn editing_a_cite_changes_its_digest_but_not_the_citing_nodes() {
        use cadetta_ir::digest::{component_digest, member_digest};
        let one = build(CITED, &[], &[]);
        let two = build(&CITED.replace("\"A\"", "\"B\""), &[], &[]);
        let d = |b: &Built| b.project.citations[0].digest.clone();
        assert_ne!(d(&one), d(&two));
        assert_eq!(
            member_digest(member(&one, "Part", "a")),
            member_digest(member(&two, "Part", "a"))
        );
        assert_eq!(
            component_digest(&one.project.components[0]),
            component_digest(&two.project.components[0])
        );
    }

    const GEOM_SRC: &str = "model Rail {\n  height = 30mm\n  datum top = origin.offset(z = height)\n}\n\
        model Bracket {\n  holes: [Point2 on origin; 2] = [Point2(-25mm, 0mm), Point2(25mm, 0mm)]\n}\n\
        assembly Mount {\n  part rail: Rail\n  part bracket: Bracket\n  \
        place rail.origin at origin\n  place bracket.origin at origin.offset(x = 10mm)\n  \
        hx = (bracket.holes[1] in origin).x\n}\n";

    // frob:ticket 01M50C0NEZM1R9KYFTRJCJY1Z8
    // frob:tests crates/cadetta-sema/src/lower.rs::lower kind=unit
    // frob:tests crates/cadetta-sema/src/lower_geom.rs::lower_geometry kind=unit
    #[test]
    fn position_point2_lowers_to_paired_uv_terms() {
        let src = "model Plate {\n  holes: [Point2 on origin; 2] = \
            [Point2(-25mm, 0mm), Point2(25mm, 0mm)] [position 0.2mm]\n  \
            lone: Point2 on origin = Point2(1mm, 2mm)\n}\n";
        let b = build(src, &[], &[]);
        assert!(b.ev.diagnostics.is_empty(), "{:?}", b.ev.diagnostics);
        let plate = &b.project.components[0];
        let mm = |n: i64, d: i64| Magnitude::new(Rational::new(n, d).unwrap(), 0);
        // Every toleranced point has its own zone diameter and its own `.u/.v` stem.
        let mut stems = Vec::new();
        for pt in plate
            .points
            .iter()
            .filter(|p| p.path.to_string().contains("holes"))
        {
            assert_eq!(pt.position, Some(Exact(mm(2, 10_000))), "{}", pt.path);
            let tol_of = |c: usize| {
                pt.coords[c]
                    .tol
                    .as_ref()
                    .expect("coordinate tol")
                    .terms
                    .clone()
            };
            let (xt, yt) = (tol_of(0), tol_of(1));
            let names: Vec<String> = xt.iter().map(|t| t.var.to_string()).collect();
            assert_eq!(names.len(), 2, "{names:?}");
            assert!(names[0].ends_with(".position.u"), "{names:?}");
            assert!(names[1].ends_with(".position.v"), "{names:?}");
            // The x and y coordinates share the disc, and the pair has equal symmetric bounds.
            assert_eq!(
                xt.iter().map(|t| t.var.to_string()).collect::<Vec<_>>(),
                yt.iter().map(|t| t.var.to_string()).collect::<Vec<_>>()
            );
            assert_eq!(xt[0].lo, xt[1].lo);
            assert_eq!(xt[0].hi, xt[1].hi);
            assert_eq!(xt[0].hi, Exact(mm(1, 10_000)));
            assert_eq!(xt[0].lo, Exact(mm(-1, 10_000)));
            stems.push(names[0].clone());
        }
        assert_eq!(stems.len(), 2);
        assert_ne!(stems[0], stems[1], "one disc per point");
        let lone = plate
            .points
            .iter()
            .find(|p| p.path.to_string().ends_with("lone"))
            .expect("lone");
        assert_eq!(lone.position, None);
        assert!(lone.coords.iter().all(|c| c.tol.is_none()));
    }

    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
    // frob:tests crates/cadetta-sema/src/lower_geom.rs::lower_geometry kind=unit
    #[test]
    fn datums_placements_and_points_are_lowered() {
        let b = build(GEOM_SRC, &[], &[]);
        assert!(b.ev.diagnostics.is_empty(), "{:?}", b.ev.diagnostics);
        let rail = b
            .project
            .components
            .iter()
            .find(|c| c.path.item == "Rail")
            .expect("Rail");
        assert!(
            rail.datums
                .iter()
                .any(|d| d.path.to_string().ends_with("Rail.top"))
        );
        let mount = b
            .project
            .components
            .iter()
            .find(|c| c.path.item == "Mount")
            .expect("Mount");
        let places: Vec<String> = mount
            .placements
            .iter()
            .map(|p| p.path.to_string())
            .collect();
        assert!(
            places.iter().any(|p| p.ends_with("Mount/bracket.place")),
            "{places:?}"
        );
        let br = mount
            .placements
            .iter()
            .find(|p| p.path.to_string().ends_with("Mount/bracket.place"))
            .expect("bracket placement");
        assert_eq!(br.ops.len(), 1);
        let Op::Offset(o) = &br.ops[0] else {
            panic!("expected an offset op, got {:?}", br.ops[0]);
        };
        assert!(o.x.is_some() && o.y.is_none());
    }

    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
    // frob:tests crates/cadetta-sema/src/geom.rs::reexpress kind=unit
    #[test]
    fn re_expression_composes_the_placement() {
        let b = build(GEOM_SRC, &[], &[]);
        let hx = member(&b, "Mount", "hx");
        let v = hx.value.as_ref().expect("hx value");
        // 25 mm + the 10 mm placement offset = 35 mm, exact.
        let Nominal::Exact(Exact(m)) = &v.nominal else {
            panic!("expected an exact nominal, got {v:?}");
        };
        assert_eq!(m.rational().to_string(), "7/200");
    }

    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
    // frob:tests crates/cadetta-sema/src/geom.rs::reexpress kind=unit
    #[test]
    fn re_expression_out_of_an_unplaced_part_is_g003() {
        let src = "model P {\n  datum face = origin.offset(z = 5mm)\n}\n\
            assembly A {\n  part a: P\n  part b: P\n  place a.origin at origin\n  \
            dz = (b.face in origin).z\n  dy = (a.face in face).z\n}\n";
        let (ev, _, _) = evaluate(src);
        let codes: Vec<&str> = ev.diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(codes, vec!["G003", "G003"], "{:?}", ev.diagnostics);
    }

    const LEGS_SRC: &str = "model Leg {\n  length = 700mm\n  datum top = origin.offset(z = length)\n}\n\
        assembly Frame {\n  part leg[4]: Leg\n  \
        corners: [Point2 on origin; 4] = [Point2(0mm, 0mm), Point2(1000mm, 0mm), Point2(0mm, 500mm), Point2(1000mm, 500mm)]\n  \
        place leg.origin at origin.offset(corners)\n  \
        gx = (leg[1].top in origin).x - (leg[0].top in origin).x\n  \
        gy = (leg[2].top in origin).y\n  hz = (leg[3].top in origin).z\n  \
        reach = leg[2].length\n}\n";

    fn exact_text(b: &Built, item: &str, name: &str) -> String {
        let v = member(b, item, name).value.as_ref().expect("value");
        let Nominal::Exact(Exact(m)) = &v.nominal else {
            panic!("expected an exact nominal, got {v:?}");
        };
        m.rational().to_string()
    }

    // frob:ticket 01M4VYZRQKXYE9WEFMDM3797GP
    // frob:tests crates/cadetta-sema/src/geom.rs::reexpress kind=unit
    #[test]
    fn indexed_part_instances_re_express_at_their_own_placement() {
        let b = build(LEGS_SRC, &[], &[]);
        assert!(b.ev.diagnostics.is_empty(), "{:?}", b.ev.diagnostics);
        assert_eq!(exact_text(&b, "Frame", "gx"), "1");
        assert_eq!(exact_text(&b, "Frame", "gy"), "1/2");
        assert_eq!(exact_text(&b, "Frame", "hz"), "7/10");
    }

    // frob:ticket 01M4VYZRQKXYE9WEFMDM3797GP
    // frob:tests crates/cadetta-sema/src/geom.rs::through_parts kind=unit
    #[test]
    fn a_member_read_through_an_indexed_slot_is_the_part_types_member() {
        let b = build(LEGS_SRC, &[], &[]);
        assert_eq!(exact_text(&b, "Frame", "reach"), "7/10");
    }

    // frob:ticket 01M4VYZRQKXYE9WEFMDM3797GP
    // frob:tests crates/cadetta-sema/src/geom.rs::reexpress kind=unit
    #[test]
    fn an_out_of_range_or_cross_instance_re_expression_is_an_error() {
        let oob = LEGS_SRC.replace("leg[3].top", "leg[9].top");
        let (ev, _, _) = evaluate(&oob);
        assert!(
            ev.diagnostics
                .iter()
                .any(|d| d.message.contains("out of range")),
            "{:?}",
            ev.diagnostics
        );
        let cross = LEGS_SRC.replace("(leg[2].top in origin).y", "(leg[2].top in leg[1].top).y");
        let (ev, _, _) = evaluate(&cross);
        assert!(
            ev.diagnostics
                .iter()
                .any(|d| d.message.contains("between instances")),
            "{:?}",
            ev.diagnostics
        );
    }

    // frob:ticket 01M4Z92C25BYMP5JYSX9WDH9XE
    // frob:tests crates/cadetta-sema/src/lower_array.rs::lower_array_members kind=unit
    #[test]
    fn part_array_instance_members_lower_per_index() {
        let src = "model Axle {\n  len = 10mm\n}\n\
            model Rover {\n  part axle[2]: Axle\n}\n";
        let b = build(src, &[], &[]);
        let rover = b
            .project
            .components
            .iter()
            .find(|c| c.path.item == "Rover")
            .expect("Rover");
        let mut paths: Vec<String> = rover.members.iter().map(|m| m.path.to_string()).collect();
        paths.sort();
        assert_eq!(paths, ["p::Rover/axle[0].len", "p::Rover/axle[1].len"]);
        assert!(rover.members.iter().all(|m| m.value.is_some()));
    }

    // frob:ticket 01M4ZAD0AW4C1R33F43WTVTR3V
    // frob:tests crates/cadetta-sema/src/instances.rs::expand_arrays kind=unit
    #[test]
    fn parameterized_part_array_lowers_per_instance_values() {
        let src = "model Arm(len: Length, gap: Length = 1mm) {\n  reach = len + gap\n}\n\
            model Rover {\n  part arm[2]: Arm(len = 10mm)\n}\n";
        let b = build(src, &[], &[]);
        let rover = b
            .project
            .components
            .iter()
            .find(|c| c.path.item == "Rover")
            .expect("Rover");
        let reach: Vec<_> = rover
            .members
            .iter()
            .filter(|m| m.path.to_string().ends_with(".reach"))
            .collect();
        let mut paths: Vec<String> = reach.iter().map(|m| m.path.to_string()).collect();
        paths.sort();
        assert_eq!(paths, ["p::Rover/arm[0].reach", "p::Rover/arm[1].reach"]);
        for m in reach {
            let v = m.value.as_ref().expect("a value");
            assert!(format!("{v:?}").contains("Rational(11/1000)"), "{v:?}");
        }
    }

    const INST_GEOM_SRC: &str = "model Leg {\n  len = 10mm\n}\n\
        model Box(depth: Length, lift: Length = 3mm) {\n  \
        datum mount = origin.offset(z = depth + lift)\n}\n\
        assembly Frame(w: Length) {\n  part leg: Leg\n  \
        place leg.origin at origin.offset(x = w)\n}\n\
        assembly Top {\n  part j: Box(depth = 7mm)\n  part f: Frame(w = 4mm)\n}\n";

    fn top_member<'a>(b: &'a Built, path: &str) -> Option<&'a Member> {
        let top = b
            .project
            .components
            .iter()
            .find(|c| c.path.item == "Top")
            .expect("Top");
        top.members.iter().find(|m| m.path.to_string() == path)
    }

    // frob:ticket 01M4VYZS8VGT1RASS407Y8K809
    // frob:tests crates/cadetta-sema/src/lower_geom.rs::lower_instance_geometry kind=unit
    #[test]
    fn instance_datums_are_valued_from_args_and_defaults() {
        let b = build(INST_GEOM_SRC, &[], &[]);
        let m = top_member(&b, "p::Top/j.mount.offset.z").expect("datum member");
        assert_eq!(m.kind, MemberKind::Datum);
        let v = m.value.as_ref().expect("a value");
        assert!(
            format!("{v:?}").contains("Rational(1/100)"),
            "7mm + 3mm default: {v:?}"
        );
    }

    // frob:ticket 01M4VYZS8VGT1RASS407Y8K809
    // frob:tests crates/cadetta-sema/src/lower_geom.rs::lower_instance_geometry kind=unit
    #[test]
    fn instance_placements_are_valued_from_args() {
        let b = build(INST_GEOM_SRC, &[], &[]);
        let m = top_member(&b, "p::Top/f/leg.place.offset.x").expect("placement member");
        assert_eq!(m.kind, MemberKind::Place);
        let v = m.value.as_ref().expect("a value");
        assert!(
            format!("{v:?}").contains("Rational(1/250)"),
            "w = 4mm: {v:?}"
        );
    }
}
