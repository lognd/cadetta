//! Lint U001 `unused-parameter`: a `model` parameter nothing reads and nothing binds.
//!
//! Specification: `docs/spec/lints.md` (U).

// frob:ticket 01M4STCCVN188CD3VRZC374QTM

use cadetta_ir::model::{Component, ComponentClass, Member, MemberKind, Project, SpanRef};
use cadetta_ir::path::Path;
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Severity};

/// The rule code.
pub const CODE: &str = "U001";

/// Identity of a member ignoring instance steps: package, modules, item, member tail.
fn key(p: &Path) -> String {
    let steps_free = Path {
        steps: Vec::new(),
        ..p.clone()
    };
    steps_free.to_string()
}

/// True when `target` equals `read` or `read` addresses a sub-member of `target`.
fn covers(target: &Path, read: &Path) -> bool {
    let t = key(target);
    let r = key(read);
    r == t || r.starts_with(&format!("{t}.")) || r.starts_with(&format!("{t}["))
}

/// True when `name` occurs in `text` as a whole identifier.
fn mentions(text: &str, name: &str) -> bool {
    let is_id = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(name).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + name.len()..].chars().next();
        !before.is_some_and(is_id) && !after.is_some_and(is_id)
    })
}

/// Whether anything in `project` reads or binds `m` (a member of `owner`).
fn is_used(project: &Project, owner: &Component, m: &Member) -> bool {
    let name = m.path.member_root().unwrap_or_default();
    let read = |deps: &[Path]| deps.iter().any(|d| covers(&m.path, d));
    for c in &project.components {
        for o in &c.members {
            if o.path != m.path
                && (read(&o.deps)
                    || (c.path == owner.path
                        && o.expr.as_deref().is_some_and(|e| mentions(e, name))))
            {
                return true;
            }
        }
        for f in &c.fits {
            if covers(&m.path, &f.hole) || covers(&m.path, &f.shaft) {
                return true;
            }
        }
        if c.path == owner.path
            && c.parts
                .iter()
                .any(|p| p.args.values().any(|a| mentions(a, name)))
        {
            return true;
        }
    }
    if project
        .checks
        .iter()
        .any(|k| k.path.item == owner.path.item && mentions(&k.expr, name))
    {
        return true;
    }
    if project.solvers.iter().any(|s| {
        read(&s.deps)
            || s.inputs
                .values()
                .chain(s.outputs.values())
                .any(|v| v.parse::<Path>().is_ok_and(|p| covers(&m.path, &p)))
    }) {
        return true;
    }
    project.bindings.iter().any(|b| covers(&m.path, &b.path))
}

/// Run U001 over `project`; `locate` turns a member span into a diagnostic location.
pub fn check(project: &Project, locate: impl Fn(&SpanRef) -> Location) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for c in project
        .components
        .iter()
        .filter(|c| c.class == ComponentClass::Model)
    {
        for m in c.members.iter().filter(|m| m.kind == MemberKind::Param) {
            if is_used(project, c, m) {
                trace!(path = %m.path, "U001: parameter used");
                continue;
            }
            debug!(path = %m.path, "U001: unused parameter");
            let mut d = Diagnostic::new(
                CODE,
                Severity::Info,
                format!(
                    "parameter `{}` is read by nothing and bound to nothing",
                    m.path
                ),
            );
            d.path = Some(m.path.to_string());
            d.location = m.span.as_ref().map(&locate);
            out.push(d);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{Binding, Flavour, PackageId};
    use cadetta_ir::path::PathContext;

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn project(extra: impl FnOnce(&mut Component), bind: bool) -> Project {
        let mut proj = Project::new(
            Flavour::Design,
            PackageId {
                name: "demo".into(),
                version: "0.1.0".into(),
            },
        );
        let mut c = Component {
            path: p("demo::Plate.width").clone(),
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
        };
        c.members
            .push(Member::new(p("demo::Plate.width"), MemberKind::Param));
        extra(&mut c);
        proj.components.push(c);
        if bind {
            proj.bindings.push(Binding {
                path: p("demo::Plate.width"),
                adapter: "freecad".into(),
                locators: vec![],
            });
        }
        proj
    }

    // frob:tests crates/cadetta-lint/src/rules/unused.rs::check
    #[test]
    fn fires_when_nothing_reads_or_binds() {
        let d = check(&project(|_| {}, false), |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "U001");
        assert_eq!(d[0].severity, Severity::Info);
    }

    // frob:tests crates/cadetta-lint/src/rules/unused.rs::check
    #[test]
    fn quiet_when_read() {
        let proj = project(
            |c| {
                let mut m = Member::new(p("demo::Plate.area"), MemberKind::Let);
                m.deps.push(p("demo::Plate.width"));
                c.members.push(m);
            },
            false,
        );
        assert!(check(&proj, |_| unreachable!()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/unused.rs::check
    #[test]
    fn quiet_when_bound() {
        assert!(check(&project(|_| {}, true), |_| unreachable!()).is_empty());
    }
}
