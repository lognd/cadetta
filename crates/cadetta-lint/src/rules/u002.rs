//! Lints U002 `unused-private-item` and I004 `root-not-evaluated`.
//!
//! Both rules read one fact table, [`ItemFacts`], that the driver builds from
//! name resolution (sema owns visibility; the IR does not carry it). Decisions
//! written into packages.md 3: a private item used nowhere in its package is
//! U002; a private parameterized component with no instances is U002, never
//! I004; I004 applies only to roots of the package being checked, so a shared
//! library is not noisy.
//!
//! Specification: `docs/spec/lints.md` (U, I), `docs/spec/packages.md` 3.

// frob:ticket 01M4SXRVB9C797ZD6KGK60DHZ8

use cadetta_ir::model::SpanRef;
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Severity};

/// The U002 rule code.
pub const CODE: &str = "U002";

/// The I004 rule code.
pub const CODE_I004: &str = "I004";

/// Declared visibility of an item (packages.md 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    /// No modifier, or `priv` on a member: not visible outside its module or package.
    Private,
    /// `pub(package)`: visible anywhere in the declaring package.
    PubPackage,
    /// `pub`: visible everywhere the module is reachable.
    Pub,
}

/// What kind of item a fact row describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// A component (`model`, `assembly`, `ots`, `process`).
    Component,
    /// A function.
    Function,
    /// An interface.
    Interface,
    /// A table.
    Table,
    /// A `priv` member of a component.
    Member,
}

impl ItemKind {
    /// Lower-case noun for messages.
    fn noun(self) -> &'static str {
        match self {
            Self::Component => "component",
            Self::Function => "function",
            Self::Interface => "interface",
            Self::Table => "table",
            Self::Member => "member",
        }
    }
}

/// Resolution facts about one item, built by the driver for one package.
#[derive(Clone, Debug)]
pub struct ItemFacts {
    /// Canonical path of the item.
    pub path: String,
    /// What the item is.
    pub kind: ItemKind,
    /// Declared visibility.
    pub visibility: Visibility,
    /// True when the item belongs to the package being checked (not a dependency).
    pub in_checked_package: bool,
    /// True when something in the declaring package names or uses the item,
    /// including an instantiation, an `implements` or a read.
    pub used: bool,
    /// True for an assembly: an entry point, never dead even when nothing names it.
    pub entry_point: bool,
    /// True for a component with at least one parameter lacking a default.
    pub parameterized: bool,
    /// True when no `part` of another component instantiates the component.
    pub is_root: bool,
    /// Number of instances (explicit instantiations) of the component.
    pub instances: usize,
    /// Source span of the declaration.
    pub span: Option<SpanRef>,
}

/// Whether U002 fires for `i`: private, in the checked package, used nowhere.
fn is_dead_private(i: &ItemFacts) -> bool {
    i.in_checked_package
        && i.visibility == Visibility::Private
        && !i.used
        && !i.entry_point
        && i.instances == 0
}

/// Run U002 over `items`; `locate` turns a span into a diagnostic location.
pub fn check(items: &[ItemFacts], locate: impl Fn(&SpanRef) -> Location) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for i in items {
        if !is_dead_private(i) {
            trace!(path = %i.path, "U002: item live or not private");
            continue;
        }
        debug!(path = %i.path, kind = i.kind.noun(), "U002: unused private item");
        let mut d = Diagnostic::new(
            CODE,
            Severity::Warning,
            format!(
                "private {} `{}` is used nowhere in its package",
                i.kind.noun(),
                i.path
            ),
        );
        d.path = Some(i.path.clone());
        d.location = i.span.as_ref().map(&locate);
        out.push(d);
    }
    out
}

/// Run I004 over `items`: parameterized roots of the checked package with no instances.
///
/// A row U002 already reports is skipped, so the two rules never double-report.
pub fn check_i004(items: &[ItemFacts], locate: impl Fn(&SpanRef) -> Location) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for i in items {
        let candidate = i.kind == ItemKind::Component
            && i.in_checked_package
            && i.is_root
            && i.parameterized
            && i.instances == 0;
        if !candidate {
            trace!(path = %i.path, "I004: not a root needing instances");
            continue;
        }
        if is_dead_private(i) {
            debug!(path = %i.path, "I004: left to U002 (private and unused)");
            continue;
        }
        debug!(path = %i.path, "I004: root not evaluated");
        let mut d = Diagnostic::new(
            CODE_I004,
            Severity::Info,
            format!(
                "root `{}` has parameters without defaults and no instances, so its members are never evaluated",
                i.path
            ),
        );
        d.path = Some(i.path.clone());
        d.location = i.span.as_ref().map(&locate);
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(vis: Visibility) -> ItemFacts {
        ItemFacts {
            path: "demo::Plate".into(),
            kind: ItemKind::Component,
            visibility: vis,
            in_checked_package: true,
            used: false,
            entry_point: false,
            parameterized: false,
            is_root: true,
            instances: 0,
            span: None,
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/u002.rs::check
    #[test]
    fn u002_fires_on_unused_private_item() {
        let d = check(&[item(Visibility::Private)], |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "U002");
        assert_eq!(d[0].severity, Severity::Warning);
    }

    // frob:tests crates/cadetta-lint/src/rules/u002.rs::check
    #[test]
    fn u002_quiet_once_used() {
        let mut i = item(Visibility::Private);
        i.used = true;
        assert!(check(&[i], |_| unreachable!()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/u002.rs::check
    #[test]
    fn u002_quiet_for_public_and_dependency_items() {
        let mut dep = item(Visibility::Private);
        dep.in_checked_package = false;
        let items = [item(Visibility::Pub), item(Visibility::PubPackage), dep];
        assert!(check(&items, |_| unreachable!()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/u002.rs::check
    #[test]
    fn u002_quiet_for_private_entry_point_assembly() {
        let mut i = item(Visibility::Private);
        i.entry_point = true;
        assert!(check(&[i], |_| unreachable!()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/u002.rs::check_i004
    #[test]
    fn private_parameterized_without_instances_is_u002_not_i004() {
        let mut i = item(Visibility::Private);
        i.parameterized = true;
        let items = [i];
        assert_eq!(check(&items, |_| unreachable!()).len(), 1);
        assert!(check_i004(&items, |_| unreachable!()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/u002.rs::check_i004
    #[test]
    fn i004_fires_for_public_parameterized_root() {
        let mut i = item(Visibility::Pub);
        i.parameterized = true;
        let d = check_i004(&[i], |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "I004");
        assert_eq!(d[0].severity, Severity::Info);
    }

    // frob:tests crates/cadetta-lint/src/rules/u002.rs::check_i004
    #[test]
    fn i004_only_for_roots_of_the_checked_package() {
        let mut dep = item(Visibility::Pub);
        dep.parameterized = true;
        dep.in_checked_package = false;
        let mut inst = item(Visibility::Pub);
        inst.parameterized = true;
        inst.instances = 2;
        let mut defaults = item(Visibility::Pub);
        defaults.parameterized = false;
        let mut nonroot = item(Visibility::Pub);
        nonroot.parameterized = true;
        nonroot.is_root = false;
        assert!(check_i004(&[dep, inst, defaults, nonroot], |_| unreachable!()).is_empty());
    }
}
