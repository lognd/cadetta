//! Lints C001-C004: check verdicts, toleranced equality and derived features in checks.
//!
//! Specification: `docs/spec/lints.md` (C); `docs/spec/quantities.md` 5.3 and 5.8;
//! `docs/spec/language.md` 3.2 and 6.

// frob:ticket 01M4SPWR7SG2GDJ254566CDH0B

use cadetta_ir::entities::Interface;
use cadetta_ir::model::{
    Bound, Check, Component, Member, MemberKind, Project, SpanRef, VerdictState,
};
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Severity};
use crate::rules::f::{bounds, holds, violates};
use crate::rules::interface::{item_text, names};

/// Rule code: a check is false for every value in tolerance (error).
pub const C001: &str = "C001";
/// Rule code: a check holds for some values and not others (warning).
pub const C002: &str = "C002";
/// Rule code: `==` or `!=` on a toleranced operand (warning).
pub const C003: &str = "C003";
/// Rule code: a check reads a derived parameter with toleranced inputs (info).
pub const C004: &str = "C004";

/// True when the member carries a tolerance form with at least one term.
fn toleranced(m: &Member) -> bool {
    m.value
        .as_ref()
        .and_then(|v| v.tol.as_ref())
        .is_some_and(|a| !a.terms.is_empty())
}

/// The members of the component the check belongs to (same package, modules and item).
fn siblings<'a>(project: &'a Project, check: &Check) -> Vec<&'a Member> {
    project
        .components
        .iter()
        .filter(|c: &&Component| {
            c.path.package == check.path.package
                && c.path.modules == check.path.modules
                && c.path.item == check.path.item
        })
        .flat_map(|c| c.members.iter())
        .collect()
}

/// Dotted identifier tokens of `text`, in order.
fn tokens(text: &str) -> Vec<&str> {
    let is_start = |c: char| c.is_ascii_alphabetic() || c == '_';
    let is_body = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.';
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        match start {
            None if is_start(c) => start = Some(i),
            Some(s) if !is_body(c) => {
                out.push(&text[s..i]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(&text[s..]);
    }
    out
}

/// True when `token` reads member `name` (itself or a sub-member), not through `.nominal`.
fn reads_raw(token: &str, name: &str) -> bool {
    let reads = token == name || token.starts_with(&format!("{name}."));
    reads && !token.ends_with(".nominal")
}

/// True when `token` reads member `name` in any form.
fn reads_any(token: &str, name: &str) -> bool {
    token == name || token.starts_with(&format!("{name}."))
}

/// The textual operands on each side of every `==` and `!=` in `expr`.
fn equality_operands(expr: &str) -> Vec<(&str, &str)> {
    let b = expr.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < b.len() {
        let pair = &expr[i..i + 2];
        let lone = i == 0 || !matches!(b[i - 1], b'<' | b'>' | b'=' | b'!');
        if (pair == "==" || pair == "!=") && lone {
            let mut lo = 0;
            let mut depth = 0i32;
            for j in (0..i).rev() {
                match b[j] {
                    b')' => depth += 1,
                    b'(' if depth == 0 => {
                        lo = j + 1;
                        break;
                    }
                    b'(' => depth -= 1,
                    b'<' | b'>' if depth == 0 => {
                        lo = j + 1;
                        break;
                    }
                    _ => {}
                }
                if depth == 0 && (expr[..j].ends_with(" and") || expr[..j].ends_with(" or")) {
                    lo = j;
                    break;
                }
            }
            let mut hi = expr.len();
            depth = 0;
            for j in i + 2..b.len() {
                match b[j] {
                    b'(' => depth += 1,
                    b')' if depth == 0 => {
                        hi = j;
                        break;
                    }
                    b')' => depth -= 1,
                    b'<' | b'>' if depth == 0 => {
                        hi = j;
                        break;
                    }
                    _ => {}
                }
                if depth == 0 && (expr[j..].starts_with(" and ") || expr[j..].starts_with(" or ")) {
                    hi = j;
                    break;
                }
            }
            out.push((&expr[lo..i], &expr[i + 2..hi]));
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

/// Build a diagnostic on `check`, optionally scoped to alternate `alt`.
fn on_check(
    code: &str,
    sev: Severity,
    check: &Check,
    alt: Option<&str>,
    msg: String,
) -> Diagnostic {
    let mut d = Diagnostic::new(code, sev, msg);
    d.path = Some(check.path.to_string());
    d.alt = alt.map(str::to_owned);
    d
}

/// Run C001 and C002 over the primary and per-alternate verdicts of every check.
pub fn check_verdicts(project: &Project) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for k in &project.checks {
        let alts = k
            .verdicts
            .per_alternate
            .iter()
            .map(|(a, v)| (Some(a.as_str()), *v));
        for (alt, v) in std::iter::once((None, k.verdicts.primary.unwrap_or(VerdictState::Pass)))
            .filter(|_| k.verdicts.primary.is_some())
            .chain(alts)
        {
            let scope = alt.map(|a| format!(" for alt {a}")).unwrap_or_default();
            match v {
                VerdictState::Fail => {
                    debug!(check = %k.path, alt = ?alt, "C001: check violated");
                    out.push(on_check(
                        C001,
                        Severity::Error,
                        k,
                        alt,
                        format!(
                            "check `{}` is false for every value in tolerance{scope}",
                            k.path
                        ),
                    ));
                }
                VerdictState::Unknown => {
                    debug!(check = %k.path, alt = ?alt, "C002: check undecided");
                    let how = if k.conservative {
                        " (the evaluation was conservative)"
                    } else {
                        ""
                    };
                    out.push(on_check(
                        C002,
                        Severity::Warning,
                        k,
                        alt,
                        format!(
                            "check `{}` holds for some values in tolerance and not others{how}{scope}",
                            k.path
                        ),
                    ));
                }
                VerdictState::Pass => trace!(check = %k.path, alt = ?alt, "C001/C002: holds"),
            }
        }
    }
    out
}

/// Run C003: `==` or `!=` with a toleranced operand, unless written on `.nominal`.
pub fn check_equality(project: &Project) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for k in &project.checks {
        let members = siblings(project, k);
        for (lhs, rhs) in equality_operands(&k.expr) {
            let hit = [lhs, rhs].iter().any(|side| {
                tokens(side).iter().any(|t| {
                    members.iter().any(|m| {
                        toleranced(m) && m.path.member_root().is_some_and(|n| reads_raw(t, n))
                    })
                })
            });
            if hit {
                debug!(check = %k.path, "C003: toleranced equality");
                out.push(on_check(
                    C003,
                    Severity::Warning,
                    k,
                    None,
                    format!(
                        "check `{}` compares toleranced values with `==`/`!=`; compare nominals instead, e.g. `{} == {}`",
                        k.path,
                        accessor(lhs),
                        accessor(rhs)
                    ),
                ));
            } else {
                trace!(check = %k.path, "C003: no toleranced operand");
            }
        }
    }
    out
}

/// An operand rewritten with `.nominal` when it is a single identifier.
fn accessor(operand: &str) -> String {
    let t = operand.trim();
    if tokens(t) == [t] && !t.ends_with(".nominal") {
        format!("{t}.nominal")
    } else {
        t.to_owned()
    }
}

/// Run C004: a check reads a derived parameter, with no tolerance of its own, over toleranced inputs.
///
/// The finding sits on the derived parameter (the member to turn into a `let`), once per
/// parameter however many checks read it; `locate` turns the member span into a location.
pub fn check_derived(project: &Project, locate: impl Fn(&SpanRef) -> Location) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();
    for k in &project.checks {
        let members = siblings(project, k);
        let toks = tokens(&k.expr);
        for m in members.iter().filter(|m| {
            m.kind == MemberKind::Param && m.expr.is_some() && !m.deps.is_empty() && !toleranced(m)
        }) {
            let Some(name) = m.path.member_root() else {
                continue;
            };
            if !toks.iter().any(|t| reads_any(t, name)) {
                continue;
            }
            let tol_input = m.deps.iter().any(|d| {
                project
                    .components
                    .iter()
                    .flat_map(|c| c.members.iter())
                    .any(|o| o.path == *d && toleranced(o))
            });
            let path = m.path.to_string();
            if tol_input && !out.iter().any(|d| d.path.as_deref() == Some(path.as_str())) {
                debug!(check = %k.path, param = %m.path, "C004: derived parameter in check");
                let mut d = Diagnostic::new(
                    C004,
                    Severity::Info,
                    format!(
                        "check `{}` reads `{}`, computed from toleranced inputs but exact itself; did you mean `let`?",
                        k.path, m.path
                    ),
                );
                d.path = Some(path);
                d.location = m.span.as_ref().map(&locate);
                out.push(d);
            }
        }
    }
    out
}

/// Where a toleranced range sits against a required bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Range {
    /// Every value in tolerance satisfies the bound.
    Inside,
    /// No value in tolerance satisfies the bound.
    Outside,
    /// Some values do and some do not.
    Straddles,
}

/// Classify the `[min, max]` range of `value` against `bound`; `None` when it is not exact.
fn classify(value: &cadetta_ir::model::Value, bound: &Bound) -> Option<Range> {
    let (min, max) = bounds(value)?;
    Some(if violates((&min, &max), bound) {
        Range::Outside
    } else if holds((&min, &max), bound) {
        Range::Inside
    } else {
        Range::Straddles
    })
}

/// Run C001/C002 for interface bounds (language.md 3.2): each implementer's bounded member,
/// nominal plus tolerance, against the interface limit. Fully outside is C001, straddling C002.
// frob:ticket 01M4V13T1DW08P87XP4J4802RH
pub fn check_interface_bounds(
    components: &[Component],
    interfaces: &[Interface],
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for c in components {
        let comp = item_text(&c.path.package, &c.path.modules, &c.path.item);
        for r in &c.implements {
            let Some(iface) = interfaces.iter().find(|i| {
                names(
                    r,
                    &item_text(&i.path.package, &i.path.modules, &i.path.item),
                )
            }) else {
                trace!(implements = %r, "C001/C002: interface not in scope of this check");
                continue;
            };
            for b in &iface.bounds {
                let Some(m) = c.members.iter().find(|m| {
                    m.path.member_root() == Some(b.name.as_str()) && m.path.members.len() == 1
                }) else {
                    trace!(component = %comp, member = %b.name, "C001/C002: bounded member absent");
                    continue;
                };
                let Some(v) = &m.value else { continue };
                let Some(state) = classify(v, &b.bound) else {
                    trace!(member = %m.path, "C001/C002: value not exact, bound skipped");
                    continue;
                };
                let (code, sev, what) = match state {
                    Range::Inside => continue,
                    Range::Outside => (C001, Severity::Error, "violates"),
                    Range::Straddles => (C002, Severity::Warning, "may violate"),
                };
                debug!(member = %m.path, interface = %r, code, "interface bound {what}");
                let mut d = Diagnostic::new(
                    code,
                    sev,
                    format!(
                        "`{}` {what} the bound `{}` of interface `{r}` {}",
                        m.path,
                        b.name,
                        if state == Range::Outside {
                            "for every value in tolerance"
                        } else {
                            "for some values in tolerance"
                        }
                    ),
                );
                d.path = Some(m.path.to_string());
                d.location = m.span.as_ref().or(c.span.as_ref()).map(&locate);
                out.push(d);
            }
        }
    }
    out
}

/// Run C002 for fits whose clearance meets the declared required range for some values only
/// (language.md 3.7); a clearance entirely outside is F004.
// frob:ticket 01M4V13T1DW08P87XP4J4802RH
pub fn check_fit_ranges(
    project: &Project,
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for c in &project.components {
        for f in &c.fits {
            let Some(req) = &f.required else { continue };
            if classify(&f.clearance, req) != Some(Range::Straddles) {
                trace!(fit = %f.path, "C002: fit range not straddled");
                continue;
            }
            debug!(fit = %f.path, "C002: clearance meets the required range for some values only");
            let mut d = Diagnostic::new(
                C002,
                Severity::Warning,
                format!(
                    "clearance of `{}` meets its required range for some values in tolerance and not others",
                    f.path
                ),
            );
            d.path = Some(f.path.to_string());
            d.location = c.span.as_ref().map(&locate);
            out.push(d);
        }
    }
    out
}

/// Run C001-C004 over `project`.
pub fn check(project: &Project, locate: impl Fn(&SpanRef) -> Location) -> Vec<Diagnostic> {
    let mut out = check_verdicts(project);
    out.extend(check_equality(project));
    out.extend(check_derived(project, locate));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{
        Affine, AffineTerm, Exact, Flavour, Nominal, PackageId, Value, Verdicts,
    };
    use cadetta_ir::path::{Path, PathContext};
    use cadetta_units::{Dim, Magnitude, Rational};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn exact() -> Exact {
        Exact(Magnitude::new(Rational::new(1, 1).unwrap(), 0))
    }

    fn val(tol: bool) -> Value {
        let mut v = Value::new(Nominal::Exact(exact()), Dim::default(), "1mm");
        if tol {
            v.tol = Some(Affine {
                terms: vec![AffineTerm {
                    var: p("demo::Plate.w"),
                    lo: exact(),
                    hi: exact(),
                }],
            });
        }
        v
    }

    fn project(expr: &str, primary: VerdictState) -> Project {
        let mut proj = Project::new(
            Flavour::Design,
            PackageId {
                name: "demo".into(),
                version: "0.1.0".into(),
            },
        );
        let mut c = Component {
            path: p("demo::Plate.w"),
            class: cadetta_ir::model::ComponentClass::Model,
            params_sig: None,
            implements: vec![],
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
        };
        let mut w = Member::new(p("demo::Plate.w"), MemberKind::Param);
        w.value = Some(val(true));
        let mut a = Member::new(p("demo::Plate.a"), MemberKind::Param);
        a.value = Some(val(false));
        let mut d = Member::new(p("demo::Plate.d"), MemberKind::Param);
        d.value = Some(val(false));
        d.expr = Some("w * 2".into());
        d.deps.push(p("demo::Plate.w"));
        c.members = vec![w, a, d];
        proj.components.push(c);
        proj.checks.push(Check {
            path: p("demo::Plate.ok"),
            expr: expr.into(),
            verdicts: Verdicts {
                primary: Some(primary),
                ..Verdicts::default()
            },
            conservative: false,
            waived_for: vec![],
        });
        proj
    }

    fn codes(d: &[Diagnostic]) -> Vec<&str> {
        d.iter().map(|d| d.code.as_str()).collect()
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_verdicts
    #[test]
    fn c001_fires_on_violation_and_names_alt() {
        let mut proj = project("a > 1mm", VerdictState::Fail);
        proj.checks[0]
            .verdicts
            .per_alternate
            .insert("Plate/wheel=Alt".into(), VerdictState::Fail);
        let d = check_verdicts(&proj);
        assert_eq!(codes(&d), vec!["C001", "C001"]);
        assert_eq!(d[0].severity, Severity::Error);
        assert_eq!(d[1].alt.as_deref(), Some("Plate/wheel=Alt"));
        assert!(d[1].message.contains("for alt Plate/wheel=Alt"));
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_verdicts
    #[test]
    fn c001_quiet_when_holds() {
        assert!(check_verdicts(&project("a > 1mm", VerdictState::Pass)).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_verdicts
    #[test]
    fn c002_fires_on_undecided() {
        let d = check_verdicts(&project("a > 1mm", VerdictState::Unknown));
        assert_eq!(codes(&d), vec!["C002"]);
        assert_eq!(d[0].severity, Severity::Warning);
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_equality
    #[test]
    fn c003_fires_on_toleranced_equality_and_suggests_nominal() {
        let d = check_equality(&project("w == a", VerdictState::Unknown));
        assert_eq!(codes(&d), vec!["C003"]);
        assert_eq!(d[0].severity, Severity::Warning);
        assert!(d[0].message.contains("w.nominal"));
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_equality
    #[test]
    fn c003_quiet_on_accessors_and_exact_operands() {
        assert!(check_equality(&project("w.nominal == a.nominal", VerdictState::Pass)).is_empty());
        assert!(check_equality(&project("a == a", VerdictState::Pass)).is_empty());
        assert!(check_equality(&project("w > a", VerdictState::Pass)).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_derived
    #[test]
    fn c004_fires_on_derived_parameter() {
        let d = check_derived(&project("d > 1mm", VerdictState::Pass), |_| {
            Location::Lock { line: 0 }
        });
        assert_eq!(codes(&d), vec!["C004"]);
        assert_eq!(d[0].severity, Severity::Info);
        assert!(d[0].message.contains("let"));
        // The finding names the derived parameter, not the check.
        assert_eq!(d[0].path.as_deref(), Some("demo::Plate.d"));
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_derived
    #[test]
    fn c004_quiet_when_not_read_or_not_derived() {
        assert!(
            check_derived(&project("a > 1mm", VerdictState::Pass), |_| {
                Location::Lock { line: 0 }
            })
            .is_empty()
        );
        assert!(
            check_derived(&project("w > 1mm", VerdictState::Pass), |_| {
                Location::Lock { line: 0 }
            })
            .is_empty()
        );
    }

    fn ex(n: i64) -> Exact {
        Exact(Magnitude::new(Rational::new(n, 1).unwrap(), 0))
    }

    /// A value spanning `[nominal + lo, nominal + hi]`.
    fn spanning(nominal: i64, lo: i64, hi: i64) -> Value {
        let mut v = Value::new(Nominal::Exact(ex(nominal)), Dim::default(), "v");
        v.tol = Some(Affine {
            terms: vec![AffineTerm {
                var: p("demo::Plate.w"),
                lo: ex(lo),
                hi: ex(hi),
            }],
        });
        v
    }

    fn at_least(n: i64) -> Bound {
        Bound {
            op: cadetta_ir::model::BoundOp::Ge,
            lo: Some(ex(n)),
            hi: None,
        }
    }

    fn plate_with(member: &str, v: Value) -> Component {
        let mut proj = project("a > 1mm", VerdictState::Pass);
        let mut c = proj.components.remove(0);
        let mut m = Member::new(p(member), MemberKind::Param);
        m.value = Some(v);
        c.members = vec![m];
        c.implements = vec!["Rated".into()];
        c
    }

    fn rated() -> Interface {
        Interface {
            path: cadetta_ir::variant_name::ComponentPath {
                package: "demo".into(),
                modules: vec![],
                item: "Rated".into(),
            },
            members: vec![],
            bounds: vec![cadetta_ir::entities::InterfaceBound {
                name: "load".into(),
                bound: at_least(5),
            }],
            datums: vec![],
        }
    }

    fn run_bounds(v: Value, member: &str) -> Vec<Diagnostic> {
        check_interface_bounds(&[plate_with(member, v)], &[rated()], |_| Location::Lock {
            line: 0,
        })
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_interface_bounds
    #[test]
    fn interface_bound_fully_outside_is_c001_error() {
        let d = run_bounds(spanning(2, -1, 1), "demo::Plate.load");
        assert_eq!(codes(&d), vec!["C001"]);
        assert_eq!(d[0].severity, Severity::Error);
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_interface_bounds
    #[test]
    fn interface_bound_straddling_is_c002_warning() {
        let d = run_bounds(spanning(5, -1, 1), "demo::Plate.load");
        assert_eq!(codes(&d), vec!["C002"]);
        assert_eq!(d[0].severity, Severity::Warning);
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_interface_bounds
    #[test]
    fn interface_bound_holding_or_absent_member_is_quiet() {
        assert!(run_bounds(spanning(8, -1, 1), "demo::Plate.load").is_empty());
        assert!(run_bounds(spanning(2, -1, 1), "demo::Plate.other").is_empty());
    }

    fn plate_with_fit(clearance: Value) -> Project {
        let mut proj = project("a > 1mm", VerdictState::Pass);
        proj.components[0].fits.push(cadetta_ir::model::Fit {
            path: p("demo::Plate.f"),
            hole: p("demo::Plate.f.hole"),
            shaft: p("demo::Plate.f.shaft"),
            clearance,
            fit_type: "clearance".into(),
            required: Some(Bound {
                op: cadetta_ir::model::BoundOp::In,
                lo: Some(ex(2)),
                hi: Some(ex(4)),
            }),
        });
        proj
    }

    // frob:tests crates/cadetta-lint/src/rules/c.rs::check_fit_ranges
    #[test]
    fn fit_range_straddle_is_c002_but_outside_and_inside_are_quiet() {
        let loc = |_: &SpanRef| Location::Lock { line: 0 };
        let d = check_fit_ranges(&plate_with_fit(spanning(3, -2, 2)), loc);
        assert_eq!(codes(&d), vec!["C002"]);
        assert!(check_fit_ranges(&plate_with_fit(spanning(10, -1, 1)), loc).is_empty());
        assert!(check_fit_ranges(&plate_with_fit(spanning(3, -1, 1)), loc).is_empty());
    }
}
