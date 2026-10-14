//! Lints R001-R006, the drift family (lints.md 3 R).
//!
//! The rules are pure functions over an observed-IR input: the facts an
//! adapter `read` or `sync` produced, joined with the lock `synced` base.
//! `cadetta check` builds these inputs by spawning each adapter that can
//! `read` (`cadetta-cli` `check_drift`), and `sync` builds them for its
//! planner (adapters.md 2, 3, 3a, 7.4).
//!
//! Decisions: R001 compares `theirs` against the emitted point (`base` and
//! `ours` are emit-point strings), never the nominal; with no base there is
//! no R001 (adopt or first push, adapters.md 2); `[*]` in a binding path
//! matches any index when testing R005 (adapters.md 3).
//!
//! Specification: `docs/spec/lints.md` 3 R, `docs/spec/adapters.md` 2, 3, 3a.

// frob:ticket 01M4SPWRAWXJTWT4G08578RNQM

use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Note, Severity};

/// R001 code.
pub const CODE_R001: &str = "R001";
/// R002 code.
pub const CODE_R002: &str = "R002";
/// R003 code.
pub const CODE_R003: &str = "R003";
/// R004 code.
pub const CODE_R004: &str = "R004";
/// R005 code.
pub const CODE_R005: &str = "R005";
/// R006 code.
pub const CODE_R006: &str = "R006";

/// A native CAD locator: adapter, file and adapter-defined locator string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeRef {
    /// Adapter name.
    pub adapter: String,
    /// Native file path.
    pub file: String,
    /// Adapter-defined locator.
    pub locator: String,
}

impl NativeRef {
    /// The diagnostic location for this locator.
    fn location(&self) -> Location {
        Location::Native {
            adapter: self.adapter.clone(),
            file: self.file.clone(),
            locator: self.locator.clone(),
        }
    }
}

/// Which side moved since the last sync (adapters.md 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moved {
    /// Only the CAD document changed.
    Cad,
    /// Both the design and the CAD document changed, to different values.
    Both,
}

/// One bound locator's three-way values, all emit-point strings in native unit.
#[derive(Clone, Debug)]
pub struct DriftFact {
    /// Canonical IR path of the bound parameter.
    pub path: String,
    /// The bound native locator.
    pub native: NativeRef,
    /// Lock `synced` value, if any.
    pub base: Option<String>,
    /// Freshly computed emit-point value.
    pub ours: String,
    /// Value read from the native document.
    pub theirs: String,
}

/// Classify a fact per the adapters.md 2 table; `None` when R001 does not apply.
pub fn classify(f: &DriftFact) -> Option<Moved> {
    let base = f.base.as_ref()?;
    let ours_moved = &f.ours != base;
    let theirs_moved = &f.theirs != base;
    match (ours_moved, theirs_moved) {
        (false, true) => Some(Moved::Cad),
        (true, true) if f.theirs != f.ours => Some(Moved::Both),
        _ => None,
    }
}

/// R001: observed value differs from the emitted point; names the side that moved.
pub fn check_r001(facts: &[DriftFact]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for f in facts {
        let Some(moved) = classify(f) else {
            trace!(path = %f.path, "R001: no drift");
            continue;
        };
        let who = match moved {
            Moved::Cad => "cad moved",
            Moved::Both => "both moved",
        };
        debug!(path = %f.path, who, "R001: drift");
        let base = f.base.as_deref().unwrap_or_default();
        let mut d = Diagnostic::new(
            CODE_R001,
            Severity::Error,
            format!(
                "{who}: `{}` base {base}, ours {}, theirs {}",
                f.path, f.ours, f.theirs
            ),
        );
        d.path = Some(f.path.clone());
        d.location = Some(f.native.location());
        for (label, v) in [("base", base), ("ours", &f.ours), ("theirs", &f.theirs)] {
            d.notes.push(Note {
                message: format!("{label}: {v}"),
                location: None,
            });
        }
        out.push(d);
    }
    out
}

/// R002: native parameters that have no binding.
pub fn check_r002(unbound: &[NativeRef]) -> Vec<Diagnostic> {
    unbound
        .iter()
        .map(|n| {
            debug!(locator = %n.locator, "R002: unbound native parameter");
            let mut d = Diagnostic::new(
                CODE_R002,
                Severity::Warning,
                format!("native parameter `{}` has no binding", n.locator),
            );
            d.location = Some(n.location());
            d
        })
        .collect()
}

/// R003: bound paths whose locator is absent from the native file.
pub fn check_r003(missing: &[(String, NativeRef)]) -> Vec<Diagnostic> {
    missing
        .iter()
        .map(|(path, n)| {
            debug!(%path, locator = %n.locator, "R003: bound path missing natively");
            let mut d = Diagnostic::new(
                CODE_R003,
                Severity::Error,
                format!("bound path `{path}` not found in native file `{}`", n.file),
            );
            d.path = Some(path.clone());
            d.location = Some(n.location());
            d
        })
        .collect()
}

/// A rebuild failure a drive adapter reported after a push.
#[derive(Clone, Debug)]
pub struct RebuildFailure {
    /// The document that failed to rebuild.
    pub native: NativeRef,
    /// The tool's error text.
    pub reason: String,
}

/// R004: the CAD tool failed to rebuild after a push; documents are restored.
pub fn check_r004(failures: &[RebuildFailure]) -> Vec<Diagnostic> {
    failures
        .iter()
        .map(|f| {
            debug!(file = %f.native.file, "R004: regen failed");
            let mut d = Diagnostic::new(
                CODE_R004,
                Severity::Error,
                format!(
                    "rebuild failed in `{}` ({}); document restored from snapshot",
                    f.native.file, f.reason
                ),
            );
            d.location = Some(f.native.location());
            d
        })
        .collect()
}

/// A binding as declared: the IR path it targets.
#[derive(Clone, Debug)]
pub struct BindingFact {
    /// The bound path pattern; `[*]` matches any index.
    pub path: String,
    /// Where the binding is declared.
    pub native: NativeRef,
}

/// Whether `pattern` (with `[*]` wildcards) matches `path`.
fn path_matches(pattern: &str, path: &str) -> bool {
    if !pattern.contains("[*]") {
        return pattern == path;
    }
    let mut rest = path;
    let mut parts = pattern.split("[*]");
    let first = parts.next().unwrap_or_default();
    let Some(r) = rest.strip_prefix(first) else {
        return false;
    };
    rest = r;
    for part in parts {
        // Each wildcard consumes `[<index>]` up to the next literal part.
        let Some(open) = rest.strip_prefix('[') else {
            return false;
        };
        let Some(close) = open.find(']') else {
            return false;
        };
        rest = &open[close + 1..];
        let Some(r) = rest.strip_prefix(part) else {
            return false;
        };
        rest = r;
    }
    rest.is_empty()
}

/// R005: bindings matching no IR path and no `was` path.
pub fn check_r005(
    bindings: &[BindingFact],
    ir_paths: &[String],
    was_paths: &[String],
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for b in bindings {
        let hit = ir_paths
            .iter()
            .chain(was_paths)
            .any(|p| path_matches(&b.path, p));
        if hit {
            trace!(path = %b.path, "R005: binding resolves");
            continue;
        }
        debug!(path = %b.path, "R005: orphan binding");
        let mut d = Diagnostic::new(
            CODE_R005,
            Severity::Warning,
            format!("binding `{}` matches no IR path and no `was` path", b.path),
        );
        d.path = Some(b.path.clone());
        d.location = Some(b.native.location());
        out.push(d);
    }
    out
}

/// A push that replaces a native expression or driving equation.
#[derive(Clone, Debug)]
pub struct Replacement {
    /// Canonical IR path of the bound parameter.
    pub path: String,
    /// The locator being overwritten.
    pub native: NativeRef,
    /// The expression being replaced.
    pub old_expression: String,
    /// True when it comes from an `adopted` lock record (adapters.md 7.4).
    pub adopted: bool,
}

/// R006: a push replaces a native expression; always a warning, lists the old expression.
pub fn check_r006(replacements: &[Replacement]) -> Vec<Diagnostic> {
    replacements
        .iter()
        .map(|r| {
            debug!(path = %r.path, adopted = r.adopted, "R006: native expression replaced");
            let origin = if r.adopted { " (adopted file)" } else { "" };
            let mut d = Diagnostic::new(
                CODE_R006,
                Severity::Warning,
                format!(
                    "push replaces native expression `{}` on `{}`{origin}",
                    r.old_expression, r.path
                ),
            );
            d.path = Some(r.path.clone());
            d.location = Some(r.native.location());
            d
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nref() -> NativeRef {
        NativeRef {
            adapter: "sw".into(),
            file: "a.sldprt".into(),
            locator: "D1@Sketch1".into(),
        }
    }

    fn fact(base: Option<&str>, ours: &str, theirs: &str) -> DriftFact {
        DriftFact {
            path: "demo::d".into(),
            native: nref(),
            base: base.map(Into::into),
            ours: ours.into(),
            theirs: theirs.into(),
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/r.rs::check_r001
    #[test]
    fn r001_cad_moved_shows_three_values() {
        let d = check_r001(&[fact(Some("12.029"), "12.029", "12.5")]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "R001");
        assert_eq!(d[0].severity, Severity::Error);
        assert!(d[0].message.starts_with("cad moved"));
        assert_eq!(d[0].notes.len(), 3);
    }

    // frob:tests crates/cadetta-lint/src/rules/r.rs::check_r001
    #[test]
    fn r001_both_moved() {
        let d = check_r001(&[fact(Some("1"), "2", "3")]);
        assert!(d[0].message.starts_with("both moved"));
    }

    // frob:tests crates/cadetta-lint/src/rules/r.rs::check_r001
    #[test]
    fn r001_compares_emitted_point_not_nominal() {
        // theirs equals the emitted point (12.029), though the nominal is 12.
        assert!(check_r001(&[fact(Some("12.029"), "12.029", "12.029")]).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/r.rs::check_r001
    #[test]
    fn r001_quiet_for_converged_push_and_no_base() {
        assert!(check_r001(&[fact(Some("1"), "2", "2")]).is_empty());
        assert!(check_r001(&[fact(Some("1"), "2", "1")]).is_empty());
        assert!(check_r001(&[fact(None, "2", "3")]).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/r.rs::check_r002
    #[test]
    fn r002_warns_on_unbound_native() {
        let d = check_r002(&[nref()]);
        assert_eq!(d[0].code, "R002");
        assert_eq!(d[0].severity, Severity::Warning);
        assert!(check_r002(&[]).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/r.rs::check_r003
    #[test]
    fn r003_errors_on_missing_native() {
        let d = check_r003(&[("demo::d".into(), nref())]);
        assert_eq!(d[0].code, "R003");
        assert_eq!(d[0].severity, Severity::Error);
        assert!(check_r003(&[]).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/r.rs::check_r004
    #[test]
    fn r004_surfaces_rebuild_failure() {
        let d = check_r004(&[RebuildFailure {
            native: nref(),
            reason: "sketch over-defined".into(),
        }]);
        assert_eq!(d[0].code, "R004");
        assert_eq!(d[0].severity, Severity::Error);
        assert!(d[0].message.contains("over-defined"));
    }

    // frob:tests crates/cadetta-lint/src/rules/r.rs::check_r005
    #[test]
    fn r005_orphan_binding_and_was_and_wildcard() {
        let b = |p: &str| BindingFact {
            path: p.into(),
            native: nref(),
        };
        let ir = vec!["demo::holes[2].d".to_string()];
        let was = vec!["demo::old".to_string()];
        let d = check_r005(&[b("demo::gone")], &ir, &was);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "R005");
        assert_eq!(d[0].severity, Severity::Warning);
        assert!(check_r005(&[b("demo::old")], &ir, &was).is_empty());
        assert!(check_r005(&[b("demo::holes[*].d")], &ir, &was).is_empty());
        assert_eq!(check_r005(&[b("demo::holes[*].x")], &ir, &was).len(), 1);
    }

    // frob:tests crates/cadetta-lint/src/rules/r.rs::check_r006
    #[test]
    fn r006_warns_with_old_expression_and_adopted() {
        let mut r = Replacement {
            path: "demo::d".into(),
            native: nref(),
            old_expression: "\"A\" * 2".into(),
            adopted: false,
        };
        let d = check_r006(std::slice::from_ref(&r));
        assert_eq!(d[0].severity, Severity::Warning);
        assert!(d[0].message.contains("\"A\" * 2"));
        r.adopted = true;
        let d = check_r006(&[r]);
        assert_eq!(d[0].code, "R006");
        assert!(d[0].message.contains("adopted"));
    }
}
