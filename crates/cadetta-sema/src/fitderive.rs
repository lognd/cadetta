// frob:ticket 01M4TA3C37X4GWR2YBJQ1ZBN9Y
//! Derives the [`FitInput`]s lowering needs from the evaluated program (language.md 3.7).
//!
//! `fits::analyze` is pure; this module finds every `fit` member of every component, reads the
//! evaluated tolerance of the path(s) it names, runs `analyze` and packages the result with
//! the required range as an IR [`Bound`]. A fit whose operands are not evaluated values (a
//! component argument, an unevaluated member) is skipped with a log line.

use std::collections::BTreeMap;

use cadetta_ir::model::{Bound, BoundOp, Exact};
use cadetta_syntax::ast::{self, Component, Fit, FitSpec, MemberKind};
use cadetta_units::{Magnitude, ToleranceError, Toleranced};
use tracing::{debug, trace, warn};

use crate::eval::{Evaluation, Evaluator, Quant, Val};
use crate::fits::analyze;
use crate::lower::FitInput;
use crate::resolve::{CanonPath, Ctx, MemberSeg, PackageSrc, Resolved, Resolver};

/// The evaluated `[lo .. hi]` of each fit with a required range, by the fit's path.
#[derive(Default, Debug)]
pub struct FitRanges(BTreeMap<CanonPath, (Quant, Quant)>);

fn ctx_of(package: &str, module: &[String], c: &Component) -> Ctx {
    Ctx {
        package: package.to_owned(),
        module: module.to_vec(),
        component: Some(c.name.name.clone()),
        locals: Vec::new(),
    }
}

fn fit_path(package: &str, module: &[String], c: &Component, f: &Fit) -> CanonPath {
    let mut p = CanonPath::item(package, module, &c.name.name);
    p.members.push(MemberSeg::Name(f.name.name.clone()));
    p
}

/// Every fit member of every component, with the component and its location.
fn each_fit<'a>(
    packages: &'a [PackageSrc<'a>],
) -> impl Iterator<Item = (&'a str, &'a [String], &'a Component, &'a Fit)> {
    packages.iter().flat_map(|p| {
        p.modules.iter().flat_map(move |m| {
            m.file.components().flat_map(move |c| {
                c.members.iter().filter_map(move |mem| match &mem.kind {
                    MemberKind::Fit(f) => Some((p.name.as_str(), m.path.as_slice(), c, f)),
                    _ => None,
                })
            })
        })
    })
}

/// Evaluates the required range of every fit; run before `Evaluator::evaluate` consumes it.
pub fn eval_ranges(evaluator: &mut Evaluator, packages: &[PackageSrc]) -> FitRanges {
    let mut out = BTreeMap::new();
    for (pkg, module, c, f) in each_fit(packages) {
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
fn toleranced(q: &Quant) -> Result<Toleranced, ToleranceError> {
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

/// The evaluated tolerance at `path` as read from component `c`'s scope.
fn operand(
    resolver: &Resolver,
    evaluation: &Evaluation,
    ctx: &Ctx,
    path: &ast::Path,
) -> Option<(CanonPath, Toleranced)> {
    let Ok(Resolved::Member { path: p, .. }) = resolver.resolve_path(ctx, path, None) else {
        debug!("fits: operand is not a member; fit skipped");
        return None;
    };
    let Some(Val::Quantity(q)) = evaluation.values.get(&p).map(|e| &e.value) else {
        debug!(operand = %p, "fits: operand has no evaluated quantity; fit skipped");
        return None;
    };
    toleranced(q).ok().map(|t| (p, t))
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
    (pkg, module, c, f): (&str, &[String], &Component, &Fit),
) -> Option<FitInput> {
    let ctx = ctx_of(pkg, module, c);
    let path = fit_path(pkg, module, c, f);
    let (first, second) = match &f.spec {
        FitSpec::Paths(a, b) => {
            let (_, ta) = operand(resolver, evaluation, &ctx, a)?;
            let (_, tb) = operand(resolver, evaluation, &ctx, b)?;
            (ta, Some(tb))
        }
        _ => (operand(resolver, evaluation, &ctx, &f.on)?.1, None),
    };
    let required = ranges.0.get(&path);
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
            return None;
        }
    };
    let member = |n: &str| {
        let mut m = path.clone();
        m.members.push(MemberSeg::Name(n.to_owned()));
        m
    };
    debug!(fit = %path, fit_type = result.fit_type.name(), "fits: fit derived");
    Some(FitInput {
        hole: member("hole"),
        shaft: member("shaft"),
        path,
        result,
        required: required.map(|(l, h)| bound_of(l, h)),
    })
}

/// Analyses every fit whose operands are evaluated values, for `LowerInput::fits`.
pub fn derive_fits(
    resolver: &Resolver,
    evaluation: &Evaluation,
    ranges: &FitRanges,
    packages: &[PackageSrc],
) -> Vec<FitInput> {
    each_fit(packages)
        .filter_map(|x| derive_one(resolver, evaluation, ranges, x))
        .collect()
}
