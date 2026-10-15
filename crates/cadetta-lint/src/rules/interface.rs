//! Lints I001-I004: interface conformance, alternate viability and unevaluated roots.
//!
//! Specification: `docs/spec/lints.md` (I); `docs/spec/language.md` 3.2.

// frob:ticket 01M4STCCK2B2XG83N4BRS5GR7B

use std::collections::BTreeMap;

use cadetta_ir::entities::Interface;
use cadetta_ir::model::{
    Check, Component, ComponentClass, MemberKind, Project, SpanRef, VerdictState,
};
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Severity};

/// Rule code: an implementer lacks an interface member (error, fixed).
pub const I001: &str = "I001";
/// Rule code: an approved alternate fails a check (warning).
pub const I002: &str = "I002";
/// Rule code: an approved alternate leaves a decided check undecided (info).
pub const I003: &str = "I003";
/// Rule code: a root with parameters and no instances is never evaluated (info).
pub const I004: &str = "I004";

/// `pkg::mod::Item` text of a component or interface path (no member tail).
pub(crate) fn item_text(package: &str, modules: &[String], item: &str) -> String {
    let mut s = package.to_owned();
    for m in modules {
        s.push_str("::");
        s.push_str(m);
    }
    s.push_str("::");
    s.push_str(item);
    s
}

/// True when the reference `text` (full or short form) names the item `full`.
pub(crate) fn names(text: &str, full: &str) -> bool {
    text == full || full.ends_with(&format!("::{text}"))
}

/// The full item text of a component.
fn component_text(c: &Component) -> String {
    item_text(&c.path.package, &c.path.modules, &c.path.item)
}

/// Run I001 over `components` against `interfaces`; `locate` maps a span to a location.
pub fn check_members(
    components: &[Component],
    interfaces: &[Interface],
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for c in components {
        for r in &c.implements {
            let Some(iface) = interfaces.iter().find(|i| {
                names(
                    r,
                    &item_text(&i.path.package, &i.path.modules, &i.path.item),
                )
            }) else {
                trace!(implements = %r, "I001: interface not in scope of this check");
                continue;
            };
            let has = |name: &str, kind: Option<MemberKind>| {
                c.members
                    .iter()
                    .any(|m| m.path.member_root() == Some(name) && kind.is_none_or(|k| m.kind == k))
            };
            // Datums live in `Component.datums`; a bare `datum d = origin` has no members.
            let has_datum = |name: &str| {
                c.datums.iter().any(|d| d.path.member_root() == Some(name))
                    || has(name, Some(MemberKind::Datum))
            };
            let missing = iface
                .members
                .iter()
                .filter(|m| !m.optional && !has(&m.name, None))
                .map(|m| m.name.as_str())
                .chain(
                    iface
                        .datums
                        .iter()
                        .filter(|d| !has_datum(d))
                        .map(String::as_str),
                );
            for name in missing {
                debug!(component = %component_text(c), interface = %r, member = name, "I001: missing member");
                let mut d = Diagnostic::new(
                    I001,
                    Severity::Error,
                    format!(
                        "`{}` implements `{r}` but lacks member `{name}`",
                        component_text(c)
                    ),
                );
                d.path = Some(format!("{}.{name}", component_text(c)));
                d.location = c.span.as_ref().map(&locate);
                out.push(d);
            }
        }
    }
    out
}

/// Run I002 and I003 over the checks' per-alternate verdicts.
pub fn check_alternates(project: &Project) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    // An alternate is viable when no check fails under it.
    let mut viable: BTreeMap<&str, bool> = BTreeMap::new();
    for k in &project.checks {
        for (alt, v) in &k.verdicts.per_alternate {
            let e = viable.entry(alt.as_str()).or_insert(true);
            *e &= *v != VerdictState::Fail;
        }
    }
    for k in &project.checks {
        for (alt, v) in &k.verdicts.per_alternate {
            let primary = k.verdicts.primary;
            match v {
                VerdictState::Fail if primary != Some(VerdictState::Fail) => {
                    debug!(check = %k.path, alt = %alt, "I002: alternate not viable");
                    out.push(alt_diag(
                        I002,
                        Severity::Warning,
                        k,
                        alt,
                        format!("approved alternate `{alt}` violates check `{}`", k.path),
                    ));
                }
                VerdictState::Unknown
                    if viable.get(alt.as_str()) == Some(&true)
                        && matches!(primary, Some(VerdictState::Pass | VerdictState::Fail)) =>
                {
                    debug!(check = %k.path, alt = %alt, "I003: alternate undecided");
                    out.push(alt_diag(
                        I003,
                        Severity::Info,
                        k,
                        alt,
                        format!(
                            "approved alternate `{alt}` leaves check `{}` undecided",
                            k.path
                        ),
                    ));
                }
                _ => trace!(check = %k.path, alt = %alt, "I002/I003: nothing to report"),
            }
        }
    }
    out
}

/// Build an alternate diagnostic on `check`.
fn alt_diag(code: &str, sev: Severity, check: &Check, alt: &str, msg: String) -> Diagnostic {
    let mut d = Diagnostic::new(code, sev, msg);
    d.path = Some(check.path.to_string());
    d.alt = Some(alt.to_owned());
    d
}

/// Run I004: parameterized roots of the checked package that nothing instantiates.
///
/// `is_private` reports a component's visibility; private ones are U002, not I004.
pub fn check_roots(
    project: &Project,
    is_private: impl Fn(&Component) -> bool,
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for c in &project.components {
        let text = component_text(c);
        let evaluable = matches!(c.class, ComponentClass::Model | ComponentClass::Assembly);
        if c.path.package != project.package.name
            || !evaluable
            || c.params_sig.is_none()
            || is_private(c)
        {
            continue;
        }
        let instantiated = project.components.iter().any(|o| {
            o.parts.iter().any(|p| {
                names(&p.primary, &text)
                    || names(&p.type_name, &text)
                    || p.alternates.iter().any(|a| names(a, &text))
            })
        });
        if instantiated {
            trace!(component = %text, "I004: instantiated");
            continue;
        }
        debug!(component = %text, "I004: root not evaluated");
        let mut d = Diagnostic::new(
            I004,
            Severity::Info,
            format!(
                "`{text}` has parameters without defaults and no instances; its members are never evaluated"
            ),
        );
        d.path = Some(text);
        d.location = c.span.as_ref().map(&locate);
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{LintConfig, Suppressions, apply};
    use cadetta_ir::entities::InterfaceMember;
    use cadetta_ir::entities::Waiver;
    use cadetta_ir::model::{Flavour, Member, PackageId, Part, Verdicts};
    use cadetta_ir::path::{Path, PathContext};
    use cadetta_ir::variant_name::ComponentPath;

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn comp(path: &str) -> Component {
        Component {
            path: p(path),
            class: ComponentClass::Model,
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
        }
    }

    fn project() -> Project {
        Project::new(
            Flavour::Design,
            PackageId {
                name: "demo".into(),
                version: "0.1.0".into(),
            },
        )
    }

    fn iface() -> Interface {
        Interface {
            path: ComponentPath {
                package: "demo".into(),
                modules: vec![],
                item: "Caster".into(),
            },
            members: vec![
                InterfaceMember {
                    name: "bore".into(),
                    type_name: "Length".into(),
                    optional: false,
                },
                InterfaceMember {
                    name: "note".into(),
                    type_name: "Length".into(),
                    optional: true,
                },
            ],
            bounds: vec![],
            datums: vec![],
        }
    }

    fn check_with(primary: VerdictState, alt: VerdictState) -> Check {
        let mut v = Verdicts {
            primary: Some(primary),
            ..Default::default()
        };
        v.per_alternate.insert("wheel=Colson".into(), alt);
        Check {
            path: p("demo::Cart.ok"),
            expr: "x".into(),
            verdicts: v,
            conservative: false,
            waived_for: vec![],
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/interface.rs::check_members
    #[test]
    fn i001_fires_on_missing_required_member_only() {
        let mut c = comp("demo::Wheel.x");
        c.implements.push("Caster".into());
        let d = check_members(&[c.clone()], &[iface()], |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "I001");
        assert_eq!(d[0].severity, Severity::Error);
        assert!(d[0].message.contains("bore"));
        c.members
            .push(Member::new(p("demo::Wheel.bore"), MemberKind::Param));
        assert!(check_members(&[c], &[iface()], |_| unreachable!()).is_empty());
    }

    // frob:ticket 01M4V43BDMB7CKR8W79S5DC5W0
    // frob:tests crates/cadetta-lint/src/rules/interface.rs::check_members
    #[test]
    fn i001_datum_is_found_in_component_datums_and_missing_one_fires() {
        let mut i = iface();
        i.datums.push("mount".into());
        let mut c = comp("demo::Wheel.x");
        c.implements.push("Caster".into());
        c.members
            .push(Member::new(p("demo::Wheel.bore"), MemberKind::Param));
        let d = check_members(&[c.clone()], &[i.clone()], |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert!(d[0].message.contains("mount"));
        c.datums.push(cadetta_ir::geometry::Datum::new(
            p("demo::Wheel.mount"),
            p("demo::Wheel.origin"),
            vec![],
        ));
        assert!(check_members(&[c], &[i], |_| unreachable!()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/interface.rs::check_members
    #[test]
    fn i001_waiver_is_refused() {
        let mut c = comp("demo::Wheel.x");
        c.implements.push("Caster".into());
        let diags = check_members(&[c], &[iface()], |_| unreachable!());
        let sup = Suppressions {
            waivers: vec![Waiver::new("I001", "demo::Wheel.*", "no")],
            ..Default::default()
        };
        let out = apply(diags, &LintConfig::new(), &sup);
        assert_eq!(out.diagnostics.len(), 1);
        assert!(out.diagnostics[0].waived.is_none());
        assert_eq!(out.diagnostics[0].severity, Severity::Error);
    }

    // frob:tests crates/cadetta-lint/src/rules/interface.rs::check_alternates
    #[test]
    fn i002_only_when_primary_does_not_fail() {
        let mut proj = project();
        proj.checks
            .push(check_with(VerdictState::Pass, VerdictState::Fail));
        let d = check_alternates(&proj);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "I002");
        assert_eq!(d[0].severity, Severity::Warning);
        assert_eq!(d[0].alt.as_deref(), Some("wheel=Colson"));
        proj.checks[0] = check_with(VerdictState::Fail, VerdictState::Fail);
        assert!(check_alternates(&proj).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/interface.rs::check_alternates
    #[test]
    fn i003_only_for_viable_alternate_primary_decided() {
        let mut proj = project();
        proj.checks
            .push(check_with(VerdictState::Pass, VerdictState::Unknown));
        let d = check_alternates(&proj);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "I003");
        assert_eq!(d[0].severity, Severity::Info);
        // The alternate fails another check: not viable, so I002 only.
        let mut other = check_with(VerdictState::Pass, VerdictState::Fail);
        other.path = p("demo::Cart.other");
        proj.checks.push(other);
        let codes: Vec<_> = check_alternates(&proj)
            .into_iter()
            .map(|d| d.code)
            .collect();
        assert_eq!(codes, ["I002"]);
        // Primary undecided too: nothing to report.
        proj.checks = vec![check_with(VerdictState::Unknown, VerdictState::Unknown)];
        assert!(check_alternates(&proj).is_empty());
    }

    fn root() -> Component {
        let mut c = comp("demo::Bracket.x");
        c.params_sig = Some("width".into());
        c
    }

    // frob:tests crates/cadetta-lint/src/rules/interface.rs::check_roots
    #[test]
    fn i004_fires_for_uninstantiated_public_root() {
        let mut proj = project();
        proj.components.push(root());
        let d = check_roots(&proj, |_| false, |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "I004");
        assert_eq!(d[0].severity, Severity::Info);
    }

    // frob:tests crates/cadetta-lint/src/rules/interface.rs::check_roots
    #[test]
    fn i004_quiet_when_private_instantiated_or_foreign() {
        let mut proj = project();
        proj.components.push(root());
        assert!(check_roots(&proj, |_| true, |_| unreachable!()).is_empty());
        let mut asm = comp("demo::Cart.x");
        asm.parts.push(Part {
            path: p("demo::Cart.b"),
            type_name: "Bracket".into(),
            count: None,
            primary: "Bracket".into(),
            alternates: vec![],
            args: Default::default(),
        });
        proj.components.push(asm);
        assert!(check_roots(&proj, |_| false, |_| unreachable!()).is_empty());
        let mut other = project();
        let mut foreign = root();
        foreign.path = p("other::Bracket.x");
        other.components.push(foreign);
        assert!(check_roots(&other, |_| false, |_| unreachable!()).is_empty());
    }
}
