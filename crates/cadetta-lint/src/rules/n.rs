//! Lints N001-N006: the naming family.
//!
//! N001 (case canon), N002 (redundant prefix), N003 (unit in name), N005
//! (reserved separator) are derived from
//! the IR. N006 (type read beside an instance) is decided by the resolver and
//! reported with the dependency diagnostics (`cadetta-sema` `deps.rs`). N004 (renamed reference) is known to the resolver, which reads `was`
//! paths from text (language.md 4); it hands findings over and [`lift_renamed`]
//! turns them into catalog diagnostics. N005 is a fixed error and cannot be waived.
//!
//! Specification: `docs/spec/lints.md` (N), `docs/spec/language.md` 3.1 and 4.

// frob:ticket 01M4SPWQT4ZPZ7YT5W1Q41GK55

use std::collections::BTreeSet;

use cadetta_ir::model::{Component, Member, MemberKind, Project, SpanRef};
use cadetta_ir::path::Path;
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location};
use crate::engine::LintConfig;

/// Rule code: name breaks the case canon.
pub const N001: &str = "N001";
/// Rule code: member repeats its component's name.
pub const N002: &str = "N002";
/// Rule code: name encodes a unit.
pub const N003: &str = "N003";
/// Rule code: reference uses a `was` path.
pub const N004: &str = "N004";
/// Rule code: name contains the reserved `__` (fixed error).
pub const N005: &str = "N005";
/// Rule code: `Type.member` read beside a `part` of that type.
pub const N006: &str = "N006";

/// Unit tokens that mark a name as unit-bearing when they end it (`diameter_mm`).
const UNIT_SUFFIXES: &[&str] = &[
    "mm", "cm", "m", "km", "um", "in", "ft", "kg", "g", "mg", "lb", "lbf", "n", "kn", "pa", "kpa",
    "mpa", "gpa", "psi", "ksi", "deg", "rad", "s", "ms", "hz", "rpm", "w", "kw", "hp", "nm", "mm2",
    "mm3", "cm2", "cm3",
];

/// Where a case rule applies.
#[derive(Clone, Copy)]
enum Canon {
    /// `lower.dotted` package name.
    Package,
    /// `snake_case`.
    Snake,
    /// `PascalCase`, digits allowed, `_` only between two digits.
    Pascal,
}

/// True when `s` is `snake_case` (digits allowed, single underscores between tokens).
fn is_snake(s: &str) -> bool {
    let mut chars = s.chars().peekable();
    if !chars.next().is_some_and(|c| c.is_ascii_lowercase()) {
        return false;
    }
    let mut prev = 'a';
    for c in s.chars().skip(1) {
        let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || (c == '_' && prev != '_');
        if !ok {
            return false;
        }
        prev = c;
    }
    prev != '_'
}

/// True when `s` is `PascalCase`; an underscore is legal only between two digits (`M2_5`).
fn is_pascal(s: &str) -> bool {
    let cs: Vec<char> = s.chars().collect();
    if !cs.first().is_some_and(|c| c.is_ascii_uppercase()) {
        return false;
    }
    cs.iter().enumerate().all(|(i, &c)| {
        c.is_ascii_alphanumeric()
            || (c == '_'
                && i > 0
                && cs[i - 1].is_ascii_digit()
                && cs.get(i + 1).is_some_and(|n| n.is_ascii_digit()))
    })
}

/// True when `s` is `lower.dotted`: dot-separated snake segments.
fn is_dotted(s: &str) -> bool {
    !s.is_empty() && s.split('.').all(is_snake)
}

/// True when `name` satisfies `canon`.
fn canonical(canon: Canon, name: &str) -> bool {
    match canon {
        Canon::Package => is_dotted(name),
        Canon::Snake => is_snake(name),
        Canon::Pascal => is_pascal(name),
    }
}

/// `PascalCase` to `snake_case` (`FrontAxle` is `front_axle`).
fn to_snake(name: &str) -> String {
    let mut out = String::new();
    let mut prev: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            if prev.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit()) {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
        prev = Some(c);
    }
    out
}

/// The unit token ending `name`, if the name encodes one (`diameter_mm` gives `mm`).
fn unit_suffix(name: &str) -> Option<&str> {
    let (stem, last) = name.rsplit_once('_')?;
    (!stem.is_empty() && UNIT_SUFFIXES.contains(&last)).then_some(last)
}

/// The slot name of a part path: the last instance step, else the member root (`Rig.plate`).
fn part_name(path: &Path) -> Option<&str> {
    path.steps
        .last()
        .map(|s| s.name.as_str())
        .or_else(|| path.member_root())
}

/// Collects diagnostics for one run.
struct Run<'a, F: Fn(&SpanRef) -> Location> {
    cfg: &'a LintConfig,
    locate: F,
    out: Vec<Diagnostic>,
}

impl<F: Fn(&SpanRef) -> Location> Run<'_, F> {
    /// Emit `code` at the configured severity (nothing when the rule is off).
    fn emit(&mut self, code: &str, msg: String, path: Option<String>, span: Option<&SpanRef>) {
        let Some(sev) = self.cfg.severity(code) else {
            trace!(code, "N: rule off");
            return;
        };
        debug!(code, ?path, "N: diagnostic");
        let mut d = Diagnostic::new(code, sev, msg);
        d.path = path;
        d.location = span.map(&self.locate);
        self.out.push(d);
    }

    /// N005 when `name` has `__`, else N001 when it breaks `canon`.
    fn name(&mut self, canon: Canon, what: &str, name: &str, path: &str, span: Option<&SpanRef>) {
        if name.contains("__") {
            self.emit(
                N005,
                format!("{what} name `{name}` contains `__`, which is reserved for mangling"),
                Some(path.to_owned()),
                span,
            );
        } else if !canonical(canon, name) {
            self.emit(
                N001,
                format!("{what} name `{name}` breaks the case canon"),
                Some(path.to_owned()),
                span,
            );
        }
    }

    /// N002 and N003 for one member name of `c`.
    fn member_name(&mut self, c: &Component, m: &Member, name: &str) {
        let path = Some(m.path.to_string());
        let span = m.span.as_ref();
        let snake = to_snake(&c.path.item);
        let flat = c.path.item.to_ascii_lowercase();
        if [snake, flat]
            .iter()
            .any(|p| name.starts_with(&format!("{p}_")) && name.len() > p.len() + 1)
        {
            self.emit(
                N002,
                format!(
                    "member `{name}` repeats its component's name `{}`",
                    c.path.item
                ),
                path.clone(),
                span,
            );
        }
        // A dimensionless value cannot carry a unit, so `tap_lead_in = 3` is not unit-bearing.
        let dimensionless = m
            .value
            .as_ref()
            .is_some_and(|v| v.dim.dimension.is_dimensionless() && v.dim.angle == 0);
        if dimensionless {
            trace!(name, "N003: dimensionless value, unit-like suffix ignored");
        } else if let Some(u) = unit_suffix(name) {
            self.emit(
                N003,
                format!("member `{name}` encodes the unit `{u}` in its name"),
                path,
                span,
            );
        }
    }
}

/// Run N001, N002, N003 and N005 over `project`; `locate` maps a span to a location.
pub fn check(
    project: &Project,
    cfg: &LintConfig,
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let mut r = Run {
        cfg,
        locate,
        out: Vec::new(),
    };
    let pkg = &project.package.name;
    r.name(Canon::Package, "package", pkg, pkg, None);
    for m in &project.modules {
        // The IR joins a nested module path with `::`; each segment is checked alone.
        for seg in m.split("::") {
            r.name(Canon::Snake, "module", seg, m, None);
        }
    }
    for c in &project.components {
        let cpath = c.path.to_string();
        let item = c.path.item.clone();
        r.name(Canon::Pascal, "component", &item, &cpath, c.span.as_ref());
        for part in &c.parts {
            if let Some(name) = part_name(&part.path) {
                let name = name.to_owned();
                r.name(
                    Canon::Snake,
                    "part",
                    &name,
                    &part.path.to_string(),
                    c.span.as_ref(),
                );
            }
        }
        let mut seen = BTreeSet::new();
        for m in c.members.iter().filter(|m| m.kind != MemberKind::Place) {
            let Some(name) = m.path.member_root() else {
                continue;
            };
            if !seen.insert(name.to_owned()) {
                continue;
            }
            r.name(
                Canon::Snake,
                "member",
                name,
                &m.path.to_string(),
                m.span.as_ref(),
            );
            if !name.contains("__") {
                r.member_name(c, m, name);
            }
        }
    }
    r.out
}

/// A reference that resolved only through a `was` path, found by the resolver.
#[derive(Clone, Debug)]
pub struct RenamedRef {
    /// The old path as written (it may name another module, file or package).
    pub old: String,
    /// The canonical path it now resolves to.
    pub current: Path,
    /// Where the reference is written.
    pub location: Option<Location>,
}

/// N004: one diagnostic per renamed reference, at the configured severity.
pub fn lift_renamed(refs: &[RenamedRef], cfg: &LintConfig) -> Vec<Diagnostic> {
    let Some(sev) = cfg.severity(N004) else {
        return Vec::new();
    };
    refs.iter()
        .map(|r| {
            debug!(old = %r.old, current = %r.current, "N004: renamed reference");
            let mut d = Diagnostic::new(
                N004,
                sev,
                format!(
                    "reference uses the old name `{}`; it is now `{}`",
                    r.old, r.current
                ),
            );
            d.path = Some(r.current.to_string());
            d.location = r.location.clone();
            d
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Severity;
    use cadetta_ir::model::{ComponentClass, Flavour, PackageId, Part};
    use cadetta_ir::path::PathContext;

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn comp(path: &str, class: ComponentClass, members: &[&str]) -> Component {
        let mut path = path.to_owned();
        if !path.contains('.') {
            path.push_str(".x");
        }
        Component {
            path: p(&path),
            class,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members: members
                .iter()
                .map(|m| Member::new(p(m), MemberKind::Param))
                .collect(),
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        }
    }

    fn proj(cs: Vec<Component>) -> Project {
        let mut pr = Project::new(
            Flavour::Design,
            PackageId {
                name: "demo".into(),
                version: "0.1.0".into(),
            },
        );
        pr.components = cs;
        pr
    }

    fn codes(pr: &Project) -> Vec<String> {
        check(pr, &LintConfig::new(), |_| unreachable!())
            .into_iter()
            .map(|d| d.code)
            .collect()
    }

    // frob:tests crates/cadetta-lint/src/rules/n.rs::check
    #[test]
    fn n002_redundant_prefix() {
        let pr = proj(vec![comp(
            "demo::Axle",
            ComponentClass::Model,
            &["demo::Axle.axle_length"],
        )]);
        assert_eq!(codes(&pr), vec!["N002"]);
        let ok = proj(vec![comp(
            "demo::Axle",
            ComponentClass::Model,
            &["demo::Axle.length"],
        )]);
        assert!(codes(&ok).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/n.rs::check
    #[test]
    fn n003_unit_in_name() {
        let pr = proj(vec![comp(
            "demo::Axle",
            ComponentClass::Model,
            &["demo::Axle.diameter_mm"],
        )]);
        assert_eq!(codes(&pr), vec!["N003"]);
    }

    // frob:tests crates/cadetta-lint/src/rules/n.rs::check
    #[test]
    fn n003_skips_dimensionless_member() {
        let mut c = comp(
            "demo::Shop",
            ComponentClass::Model,
            &["demo::Shop.tap_lead_in"],
        );
        let three = || {
            let m = cadetta_units::Magnitude::new(cadetta_units::Rational::new(3, 1).unwrap(), 0);
            cadetta_ir::model::Nominal::Exact(cadetta_ir::model::Exact(m))
        };
        c.members[0].value = Some(cadetta_ir::model::Value::new(
            three(),
            Default::default(),
            "3",
        ));
        assert!(codes(&proj(vec![c.clone()])).is_empty());
        // Positive control: a length value keeps the finding.
        let len =
            cadetta_units::Dim::new(cadetta_units::Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0);
        c.members[0].value = Some(cadetta_ir::model::Value::new(three(), len, "3in"));
        assert_eq!(codes(&proj(vec![c])), vec!["N003"]);
    }

    // frob:tests crates/cadetta-lint/src/rules/n.rs::check
    #[test]
    fn n001_component_case() {
        let mut c = comp("demo::Axle", ComponentClass::Model, &[]);
        c.path.item = "my_axle".into();
        let pr = proj(vec![c]);
        assert_eq!(codes(&pr), vec!["N001"]);
    }

    // frob:tests crates/cadetta-lint/src/rules/n.rs::check
    #[test]
    fn n001_digits_and_decimal_mark_are_canonical() {
        let pr = proj(vec![
            comp("demo::Iso4762M5", ComponentClass::Ots, &[]),
            comp("demo::M2_5", ComponentClass::Ots, &[]),
        ]);
        assert!(codes(&pr).is_empty());
        let bad = proj(vec![comp("demo::M2_x", ComponentClass::Ots, &[])]);
        assert_eq!(codes(&bad), vec!["N001"]);
    }

    // frob:tests crates/cadetta-lint/src/rules/n.rs::check
    #[test]
    fn n001_module_and_package() {
        let mut pr = proj(vec![]);
        pr.package.name = "Acme.Rover".into();
        pr.modules = vec!["frame".into(), "FrameB".into()];
        assert_eq!(codes(&pr), vec!["N001", "N001"]);
    }

    // frob:tests crates/cadetta-lint/src/rules/n.rs::check
    #[test]
    fn n001_nested_module_checks_each_segment() {
        let mut pr = proj(vec![]);
        pr.modules = vec!["drive::motor".into()];
        assert!(codes(&pr).is_empty());
        pr.modules = vec!["drive::Motor".into()];
        assert_eq!(codes(&pr), vec!["N001"]);
    }

    // frob:tests crates/cadetta-lint/src/rules/n.rs::check
    #[test]
    fn n005_is_an_error_and_fixed() {
        let pr = proj(vec![comp(
            "demo::Axle",
            ComponentClass::Model,
            &["demo::Axle.hub__bore"],
        )]);
        let mut cfg = LintConfig::new();
        let d = check(&pr, &cfg, |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "N005");
        assert_eq!(d[0].severity, Severity::Error);
        assert!(cfg.set_severity("N005", "off").is_err());
    }

    // frob:tests crates/cadetta-lint/src/rules/n.rs::check
    #[test]
    fn n001_checks_part_slot_named_in_members() {
        let mut asm = comp("demo::Rig", ComponentClass::Assembly, &["demo::Rig.total"]);
        asm.parts.push(Part {
            path: p("demo::Rig.BadSlot"),
            type_name: "Plate".into(),
            count: None,
            primary: "Plate".into(),
            alternates: vec![],
            args: Default::default(),
        });
        assert_eq!(codes(&proj(vec![asm])), vec!["N001"]);
    }

    // frob:tests crates/cadetta-lint/src/rules/n.rs::lift_renamed
    #[test]
    fn n004_lifts_renamed_reference() {
        let refs = [RenamedRef {
            old: "demo::Axle.old_len".into(),
            current: p("demo::Axle.length"),
            location: None,
        }];
        let d = lift_renamed(&refs, &LintConfig::new());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "N004");
        assert_eq!(d[0].severity, Severity::Warning);
    }
}
