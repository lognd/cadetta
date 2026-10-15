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
use cadetta_syntax::span::Span;
use cadetta_units::{Magnitude, Role, ToleranceError, Toleranced};
use tracing::{debug, trace, warn};

use crate::eval::{Evaluation, Evaluator, Quant, Val};
use crate::fits::{FitFailure, analyze};
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

/// One fit member with where it lives: package, module path, component, fit and member span.
type FitSite<'a> = (&'a str, &'a [String], &'a Component, &'a Fit, Span);

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

/// Every fit member of every component, with the component and its location.
fn each_fit<'a>(packages: &'a [PackageSrc<'a>]) -> impl Iterator<Item = FitSite<'a>> {
    packages.iter().flat_map(|p| {
        p.modules.iter().flat_map(move |m| {
            m.file.components().flat_map(move |c| {
                c.members.iter().filter_map(move |mem| match &mem.kind {
                    MemberKind::Fit(f) => {
                        Some((p.name.as_str(), m.path.as_slice(), c, f, mem.span))
                    }
                    _ => None,
                })
            })
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
    (pkg, module, c, f, span): FitSite,
    findings: &mut Vec<FitFinding>,
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
            if let Some(code) = e.code() {
                findings.push(FitFinding {
                    code,
                    message: format!(
                        "fit `{}` could not be evaluated: {}",
                        f.name.name,
                        failure_reason(&e)
                    ),
                    path,
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
            path: path.clone(),
            span,
        });
    }
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
    let mut findings = Vec::new();
    let fits = each_fit(packages)
        .filter_map(|x| derive_one(resolver, evaluation, ranges, x, &mut findings))
        .collect();
    (fits, findings)
}
