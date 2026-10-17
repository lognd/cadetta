// frob:ticket 01M4TA3C37X4GWR2YBJQ1ZBN9Y
//! Derives the [`FitInput`]s lowering needs from the evaluated program (language.md 3.7).
//!
//! `fits::analyze` is pure; this module finds every `fit` member of every component, reads the
//! evaluated tolerance of the path(s) it names, runs `analyze` and packages the result with
//! the required range as an IR [`Bound`]. A fit whose operands are not evaluated values (a
//! component argument of the definition, an unevaluated member) is skipped with a log line; a
//! fit of a parameterized component is derived again per instance (`Parent/part.fit`), reading
//! its argument operands from the instance's evaluated members. A file-scope `fit` (no
//! enclosing component) is derived once and lowered into the component that owns its first
//! operand (the hole path of `a / b`, else the `on` path), under the path `Owner.fit_name`.

use std::collections::BTreeMap;

use cadetta_ir::model::{Bound, BoundOp, Exact};
use cadetta_syntax::ast::{self, Component, Fit, FitSpec, ItemKind, MemberKind};
use cadetta_syntax::span::Span;
use cadetta_units::{Magnitude, Role, ToleranceError, Toleranced};
use tracing::{debug, trace, warn};

use crate::eval::instances::{InstanceSite, instance_sites};
use crate::eval::{Evaluation, Evaluator, Quant, Val};
use crate::fits::{FitFailure, analyze};
use crate::lower::FitInput;
use crate::resolve::{CanonPath, Ctx, MemberSeg, PackageSrc, Resolved, Resolver};

/// The evaluated `[lo .. hi]` of each fit with a required range, by the fit's path.
#[derive(Default, Debug)]
pub struct FitRanges(BTreeMap<CanonPath, (Quant, Quant)>);

/// The name scope of a fit: inside component `c`, or file scope when `c` is `None`.
fn ctx_of(package: &str, module: &[String], c: Option<&Component>) -> Ctx {
    Ctx {
        package: package.to_owned(),
        module: module.to_vec(),
        component: c.map(|c| c.name.name.clone()),
        locals: Vec::new(),
    }
}

/// The key a fit's required range is stored under: `Component.fit`, or `module::fit` at file scope.
fn fit_path(package: &str, module: &[String], c: Option<&Component>, f: &Fit) -> CanonPath {
    let owner = c.map_or(f.name.name.as_str(), |c| c.name.name.as_str());
    let mut p = CanonPath::item(package, module, owner);
    if c.is_some() {
        p.members.push(MemberSeg::Name(f.name.name.clone()));
    }
    p
}

/// One fit with where it lives: package, module path, component (`None` at file scope), fit and span.
type FitSite<'a> = (&'a str, &'a [String], Option<&'a Component>, &'a Fit, Span);

/// The path a fit is derived under: its range key, or `Owner.fit` for a file-scope fit.
///
/// A file-scope fit is owned by the component its first operand names; `None` when that operand
/// is not a member of a component.
fn definition_path(resolver: &Resolver, site: &FitSite) -> Option<CanonPath> {
    let (pkg, module, c, f, _) = *site;
    if c.is_some() {
        return Some(fit_path(pkg, module, c, f));
    }
    let first = match &f.spec {
        FitSpec::Paths(a, _) => a,
        _ => &f.on,
    };
    match resolver.resolve_path(&ctx_of(pkg, module, None), first, None) {
        Ok(Resolved::Member { path, .. }) => {
            let mut p = path.item_only();
            p.members.push(MemberSeg::Name(f.name.name.clone()));
            debug!(fit = %p, "fits: file-scope fit owned by its first operand's component");
            Some(p)
        }
        _ => {
            debug!(fit = %f.name.name, "fits: file-scope fit operand is not a component member; fit skipped");
            None
        }
    }
}

/// An F001, F002 or F003 finding about one fit, for the F lift (lints.md F).
// frob:ticket 01M4TRKDAC6FR44KEW64H7PTZX
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FitFinding {
    /// `F001`, `F002` or `F003`.
    pub code: &'static str,
    /// One sentence, no trailing period.
    pub message: String,
    /// The fit member the finding is about.
    pub path: CanonPath,
    /// The fit member's span, in the module of `path`.
    pub span: Span,
}

/// Every fit: each component's `fit` members, then each file-scope `fit` item, with locations.
fn each_fit<'a>(packages: &'a [PackageSrc<'a>]) -> impl Iterator<Item = FitSite<'a>> {
    packages.iter().flat_map(|p| {
        p.modules.iter().flat_map(move |m| {
            let in_components = m.file.components().flat_map(move |c| {
                c.members.iter().filter_map(move |mem| match &mem.kind {
                    MemberKind::Fit(f) => {
                        Some((p.name.as_str(), m.path.as_slice(), Some(c), f, mem.span))
                    }
                    _ => None,
                })
            });
            let file_scope = m.file.items().filter_map(move |i| match &i.kind {
                ItemKind::Fit(f) => Some((p.name.as_str(), m.path.as_slice(), None, f, i.span)),
                _ => None,
            });
            in_components.chain(file_scope)
        })
    })
}

/// The tolerance role each fit implies for the member it names (quantities.md 5.6).
///
/// `hole / shaft` paths give the first the hole role and the second the shaft role; a single
/// class fit gives its `on` member the mate of the class's role. A class pair names one member
/// on both sides, so it implies nothing. The first fit to name a member wins.
// frob:ticket 01M4YNBWZ43RJ19ECMQ4280WEP
pub(crate) fn implied_roles(
    resolver: &Resolver,
    packages: &[PackageSrc],
) -> BTreeMap<CanonPath, Role> {
    let mut out = BTreeMap::new();
    for (pkg, module, c, f, _) in each_fit(packages) {
        let ctx = ctx_of(pkg, module, c);
        let named: Vec<(&ast::Path, Role)> = match &f.spec {
            FitSpec::Paths(a, b) => vec![(a, Role::Hole), (b, Role::Shaft)],
            FitSpec::Class(class) => match cadetta_fits::parse_class(&class.name) {
                Ok(parsed) => vec![(&f.on, crate::fits::role_of(parsed.side).mate())],
                Err(_) => continue,
            },
            FitSpec::Pair(..) => continue,
        };
        for (path, role) in named {
            let Ok(Resolved::Member { path: p, .. }) = resolver.resolve_path(&ctx, path, None)
            else {
                continue;
            };
            trace!(member = %p, ?role, fit = %f.name.name, "fits: fit implies a role");
            out.entry(p).or_insert(role);
        }
    }
    out
}

/// Evaluates the required range of every fit; run before `Evaluator::evaluate` consumes it.
pub fn eval_ranges(evaluator: &mut Evaluator, packages: &[PackageSrc]) -> FitRanges {
    let mut out = BTreeMap::new();
    for (pkg, module, c, f, _) in each_fit(packages) {
        let Some((lo, hi)) = &f.range else { continue };
        let ctx = ctx_of(pkg, module, c);
        let path = fit_path(pkg, module, c, f);
        match (
            evaluator.eval_input(ctx.clone(), lo),
            evaluator.eval_input(ctx, hi),
        ) {
            (Ok(l), Ok(h)) => {
                trace!(fit = %path, "fits: required range evaluated");
                out.insert(path, (l, h));
            }
            _ => warn!(fit = %path, "fits: required range did not evaluate; fit has no bound"),
        }
    }
    FitRanges(out)
}

fn exact(sum: cadetta_units::Sum) -> Result<Magnitude, ToleranceError> {
    match sum {
        cadetta_units::Sum::Exact(m) => Ok(m),
        cadetta_units::Sum::Inexact(_) => Err(ToleranceError::MixedPiExponent),
    }
}

/// The toleranced value of an evaluated quantity: its nominal with the form's bounds as deviations.
pub(crate) fn toleranced(q: &Quant) -> Result<Toleranced, ToleranceError> {
    let dim = q.form.dim().clone();
    let (lo, hi) = q.form.bounds()?;
    let lower = exact(lo.sub(&q.nominal))?;
    let upper = exact(hi.sub(&q.nominal))?;
    Toleranced::from_deviations(
        dim.clone(),
        q.nominal.clone(),
        (dim.clone(), lower),
        (dim, upper),
    )
}

/// Reads an argument operand (`bearing.outer_dia`, or a bare quantity argument) of instance `inst`.
// frob:ticket 01M4Y772AZC00VRGAKXK8V6DSY
fn instance_arg(evaluation: &Evaluation, inst: &InstanceSite, path: &CanonPath) -> Option<Quant> {
    let (MemberSeg::Name(_), rest) = path.members.split_first()? else {
        return None;
    };
    let mut key = inst.prefix.clone();
    key.members = vec![path.members[0].clone()];
    let value = &evaluation.values.get(&key)?.value;
    let value = match (value, rest) {
        (v, []) => v.clone(),
        (Val::Component(target), [MemberSeg::Name(n)]) => {
            let mut p = target.clone();
            p.members = vec![MemberSeg::Name(n.clone())];
            evaluation.values.get(&p)?.value.clone()
        }
        _ => return None,
    };
    match value {
        Val::Quantity(q) => Some(q),
        _ => None,
    }
}

/// The evaluated tolerance at `path` as read from component `c`'s scope (or from instance `inst`).
fn operand(
    resolver: &Resolver,
    evaluation: &Evaluation,
    ctx: &Ctx,
    path: &ast::Path,
    inst: Option<&InstanceSite>,
) -> Option<(Toleranced, CanonPath)> {
    let (q, at) = match (resolver.resolve_path(ctx, path, None), inst) {
        (Ok(Resolved::Arg { path: p, name }), Some(inst)) => {
            let q = instance_arg(evaluation, inst, &p);
            trace!(instance = %inst.prefix, arg = %name, found = q.is_some(), "fits: argument operand read");
            let mut at = inst.prefix.clone();
            at.members = p.members.clone();
            (q?, at)
        }
        (Ok(Resolved::Member { path: mut p, .. }), inst) => {
            if let Some(inst) = inst.filter(|i| p.parts.is_empty() && p.item_only() == i.callee) {
                let members = std::mem::take(&mut p.members);
                p = inst.prefix.clone();
                p.members = members;
            }
            match evaluation.values.get(&p).map(|e| &e.value) {
                Some(Val::Quantity(q)) => (q.clone(), p),
                _ => {
                    debug!(operand = %p, "fits: operand has no evaluated quantity; fit skipped");
                    return None;
                }
            }
        }
        _ => {
            debug!("fits: operand is not a member; fit skipped");
            return None;
        }
    };
    toleranced(&q).ok().map(|t| (t, at))
}

fn bound_of(lo: &Quant, hi: &Quant) -> Bound {
    Bound {
        op: BoundOp::In,
        lo: Some(Exact(lo.nominal.clone())),
        hi: Some(Exact(hi.nominal.clone())),
    }
}

fn derive_one(
    resolver: &Resolver,
    evaluation: &Evaluation,
    ranges: &FitRanges,
    site: FitSite,
    inst: Option<&InstanceSite>,
    findings: &mut Vec<FitFinding>,
) -> Option<FitInput> {
    let (pkg, module, c, f, span) = site;
    let ctx = ctx_of(pkg, module, c);
    let def_path = definition_path(resolver, &site)?;
    // A per-instance fit lives under the instance prefix; ranges and findings stay on the definition.
    let path = match inst {
        Some(i) => {
            let mut p = i.prefix.clone();
            p.members.push(MemberSeg::Name(f.name.name.clone()));
            p
        }
        None => def_path.clone(),
    };
    let synthetic = |n: &str| {
        let mut m = path.clone();
        m.members.push(MemberSeg::Name(n.to_owned()));
        m
    };
    // The hole and shaft are the real operand members; a side no member names (the class of a
    // class fit, both sides of a class pair) keeps the synthetic `<fit>.hole`/`.shaft` path.
    let (first, second, hole, shaft) = match &f.spec {
        FitSpec::Paths(a, b) => {
            let (ta, pa) = operand(resolver, evaluation, &ctx, a, inst)?;
            let (tb, pb) = operand(resolver, evaluation, &ctx, b, inst)?;
            (ta, Some(tb), pa, pb)
        }
        spec => {
            let (t, on) = operand(resolver, evaluation, &ctx, &f.on, inst)?;
            // `on` carries the mate of a class's side (quantities.md 5.6); a class pair names none.
            let mate = match spec {
                FitSpec::Class(class) => cadetta_fits::parse_class(&class.name)
                    .ok()
                    .map(|c| crate::fits::role_of(c.side).mate()),
                _ => None,
            };
            let (hole, shaft) = match mate {
                Some(Role::Hole) => (on, synthetic("shaft")),
                Some(_) => (synthetic("hole"), on),
                None => (synthetic("hole"), synthetic("shaft")),
            };
            (t, None, hole, shaft)
        }
    };
    let required = ranges.0.get(&fit_path(pkg, module, c, f));
    let range = required.map(|(l, h)| Ok::<_, ToleranceError>((toleranced(l)?, toleranced(h)?)));
    let range = match range.transpose() {
        Ok(r) => r,
        Err(e) => {
            warn!(fit = %path, error = ?e, "fits: required range is not a tolerance; ignored");
            None
        }
    };
    let result = match analyze(
        &f.spec,
        &first,
        second.as_ref(),
        range.as_ref().map(|(l, h)| (l, h)),
    ) {
        Ok(r) => r,
        Err(e) => {
            warn!(fit = %path, code = ?e.code(), error = ?e, "fits: fit could not be evaluated");
            if let Some(code) = e.code() {
                findings.push(FitFinding {
                    code,
                    message: format!(
                        "fit `{}` could not be evaluated: {}",
                        f.name.name,
                        failure_reason(&e)
                    ),
                    path: def_path,
                    span,
                });
            }
            return None;
        }
    };
    for d in result
        .diagnostics
        .iter()
        .filter(|d| d.code == crate::fits::F003)
    {
        findings.push(FitFinding {
            code: crate::fits::F003,
            message: d.message.clone(),
            path: def_path.clone(),
            span,
        });
    }
    debug!(fit = %path, fit_type = result.fit_type.name(), "fits: fit derived");
    Some(FitInput {
        hole,
        shaft,
        path,
        result,
        required: required.map(|(l, h)| bound_of(l, h)),
    })
}

/// The reason of a fit failure without the leading lint code of the `cadetta-fits` message.
fn failure_reason(e: &FitFailure) -> String {
    match e {
        FitFailure::Iso(inner) => {
            let text = inner.to_string();
            text.split_once(": ")
                .map_or(text.clone(), |(_, rest)| rest.to_owned())
        }
        FitFailure::NominalNotNanometres => {
            "the nominal size is not a whole number of nanometres".to_owned()
        }
        other => format!("{other:?}"),
    }
}

/// Analyses every fit whose operands are evaluated values, for `LowerInput::fits`.
pub fn derive_fits(
    resolver: &Resolver,
    evaluation: &Evaluation,
    ranges: &FitRanges,
    packages: &[PackageSrc],
) -> Vec<FitInput> {
    derive_fits_with_findings(resolver, evaluation, ranges, packages).0
}

/// [`derive_fits`] plus the F001/F002/F003 findings the analysis raised, for the F lift.
// frob:ticket 01M4TRKDAC6FR44KEW64H7PTZX
pub fn derive_fits_with_findings(
    resolver: &Resolver,
    evaluation: &Evaluation,
    ranges: &FitRanges,
    packages: &[PackageSrc],
) -> (Vec<FitInput>, Vec<FitFinding>) {
    let sites = instance_sites(resolver, packages);
    let mut findings = Vec::new();
    let mut fits = Vec::new();
    for site in each_fit(packages) {
        let callee = site
            .2
            .map(|c| CanonPath::item(site.0, site.1, &c.name.name));
        let own = derive_one(resolver, evaluation, ranges, site, None, &mut findings);
        // Only a component's own fits are derived per instance; a file-scope fit is derived once.
        let per_instance = sites
            .iter()
            .filter(|i| Some(&i.callee) == callee.as_ref())
            .filter_map(|i| derive_one(resolver, evaluation, ranges, site, Some(i), &mut findings));
        fits.extend(own.into_iter().chain(per_instance));
    }
    // One finding per fit definition, however many instances raised it.
    let mut seen: Vec<(&str, CanonPath, String)> = Vec::new();
    findings.retain(|f| {
        let key = (f.code, f.path.clone(), f.message.clone());
        let fresh = !seen.contains(&key);
        if fresh {
            seen.push(key);
        }
        fresh
    });
    debug!(
        fits = fits.len(),
        findings = findings.len(),
        "fits: derived"
    );
    (fits, findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::{ModuleSrc, PackageSrc};
    use cadetta_syntax::parser::parse;
    use cadetta_units::UnitTable;

    /// Derives the fits of one-package source `src`.
    fn derive(src: &str) -> Vec<FitInput> {
        let parsed = parse(src);
        assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
        let pkgs = vec![PackageSrc {
            name: "p".to_owned(),
            deps: Vec::new(),
            modules: vec![ModuleSrc {
                path: Vec::new(),
                file: &parsed.file,
                enums: &[],
            }],
            defaults: Default::default(),
        }];
        let (resolver, errs) = Resolver::new(pkgs.clone());
        assert!(errs.is_empty(), "{errs:?}");
        let units = UnitTable::builtin();
        let mut evaluator = Evaluator::new(&resolver, &units, &pkgs);
        let ranges = eval_ranges(&mut evaluator, &pkgs);
        let evaluation = evaluator.evaluate();
        derive_fits(&resolver, &evaluation, &ranges, &pkgs)
    }

    // frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits kind=unit
    // frob:ticket 01M4Y772AZC00VRGAKXK8V6DSY
    #[test]
    fn fit_on_a_component_argument_member_is_derived_per_instance() {
        let fits = derive(
            "model Bearing { outer_dia = 22mm [+0mm -0.009mm] }\n\
             model Housing(bearing: Bearing) { fit seat = H7 on bearing.outer_dia }\n\
             assembly Top {\n  part b: Bearing\n  part h: Housing(bearing = b)\n}\n",
        );
        assert_eq!(fits.len(), 1, "{fits:?}");
        assert_eq!(fits[0].path.to_string(), "p::Top/h.seat");
        // H7 is a hole class: the operand is the shaft, the hole side has no member.
        assert_eq!(fits[0].hole.to_string(), "p::Top/h.seat.hole");
        assert_eq!(fits[0].shaft.to_string(), "p::Top/h.bearing.outer_dia");
        assert_eq!(fits[0].result.fit_type.name(), "clearance");
    }

    // frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits kind=unit
    // frob:ticket 01M4Y772AZC00VRGAKXK8V6DSY
    #[test]
    fn fit_on_a_quantity_argument_is_derived_for_each_instance() {
        let fits = derive(
            "model Plate(d: Length) { fit seat = H7 on d }\n\
             assembly Top {\n  part a: Plate(d = 20mm)\n  part b: Plate(d = 30mm)\n}\n",
        );
        let mut paths: Vec<String> = fits.iter().map(|f| f.path.to_string()).collect();
        paths.sort();
        assert_eq!(paths, ["p::Top/a.seat", "p::Top/b.seat"]);
        assert_ne!(
            fits[0].result.hole.value, fits[1].result.hole.value,
            "each instance reads its own diameter"
        );
    }

    // frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits kind=unit
    // frob:ticket 01M4Y772AZC00VRGAKXK8V6DSY
    #[test]
    fn unevaluated_argument_operand_derives_no_fit() {
        let fits = derive(
            "model Plate(d: Length) { fit seat = H7 on d }\n\
             assembly Top { part a: Plate }\n",
        );
        assert!(fits.is_empty(), "{fits:?}");
    }

    // frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits kind=unit
    // frob:ticket 01M4Y772CY1967MXZ9T5T8FKPM
    #[test]
    fn file_scope_fit_is_derived_under_its_first_operands_component() {
        let fits = derive(
            "model Bore { d = 22mm [+0.021mm -0] }\n\
             model Pin { d = 22mm [+0.0mm -0.009mm] }\n\
             fit press = Bore.d / Pin.d [-0.05mm .. 0.05mm]\n",
        );
        assert_eq!(fits.len(), 1, "{fits:?}");
        assert_eq!(fits[0].path.to_string(), "p::Bore.press");
        // The operands are the real members, so M003 can match them (~3YZ3NSG).
        assert_eq!(fits[0].hole.to_string(), "p::Bore.d");
        assert_eq!(fits[0].shaft.to_string(), "p::Pin.d");
        assert!(fits[0].required.is_some(), "range is carried");
    }

    // frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits kind=unit
    // frob:ticket 01M4Y772CY1967MXZ9T5T8FKPM
    #[test]
    fn file_scope_class_fit_uses_the_on_path_and_unresolved_operand_derives_nothing() {
        let fits = derive("model Shaft { d = 20mm [+0mm -0.013mm] }\nfit seat = H7 on Shaft.d\n");
        assert_eq!(fits.len(), 1, "{fits:?}");
        assert_eq!(fits[0].path.to_string(), "p::Shaft.seat");
        assert_eq!(fits[0].shaft.to_string(), "p::Shaft.d");
        assert_eq!(fits[0].hole.to_string(), "p::Shaft.seat.hole");
        let none = derive("model Plate(d: Length) { }\nfit seat = H7 on d\n");
        assert!(none.is_empty(), "{none:?}");
    }
}
