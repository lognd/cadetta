// frob:ticket 01M4ST8EABK1PHDV3MGSY9GBF1
//! Interface-typed part slots with a primary and an alt list (language.md 3.2, 3.11).
//!
//! `part wheel[4]: Caster = Vendor4471B alt [Colson2Series]` is resolved into
//! a [`Slot`]: one primary and its approved alternates, each checked to
//! implement the slot's interface. Every instance of an array slot shares the
//! same [`Slot`], so a variant swaps the primary in all instances together.
//! [`Defaults`] holds the `default(Iface = Impl)` attributes in scope and
//! fills an unset component parameter of that interface type.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use cadetta_syntax::ast::{Attr, AttrArg, Ctor, ExprKind, Part, Span, TypeKind};
use tracing::{debug, trace, warn};

/// A resolved part slot: the interface, the primary and the approved alternates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The slot name (`wheel`).
    pub name: String,
    /// The interface the slot is typed by (`Caster`).
    pub interface: String,
    /// The primary implementer, when the slot names one.
    pub primary: Option<String>,
    /// The approved alternates, in written order.
    pub alts: Vec<String>,
    /// Where the slot is declared.
    pub span: Span,
}

impl Slot {
    /// Every implementer the slot names: the primary first, then the alternates.
    pub fn implementers(&self) -> impl Iterator<Item = &str> {
        self.primary
            .iter()
            .chain(self.alts.iter())
            .map(String::as_str)
    }
}

/// Why a slot could not be resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotError {
    /// The slot type is not a plain interface path.
    NotInterface {
        /// The slot name.
        slot: String,
    },
    /// An implementer (primary or alternate) does not implement the slot's interface.
    NotImplementing {
        /// The slot name.
        slot: String,
        /// The offending primary or alternate.
        implementer: String,
        /// The interface it should implement.
        interface: String,
        /// True when the offender is an alternate, false for the primary.
        alternate: bool,
        /// Where the implementer is written.
        span: Span,
    },
}

impl fmt::Display for SlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlotError::NotInterface { slot } => {
                write!(f, "slot `{slot}` is not typed by an interface path")
            }
            SlotError::NotImplementing {
                slot,
                implementer,
                interface,
                alternate,
                ..
            } => write!(
                f,
                "{} `{implementer}` of slot `{slot}` does not implement {interface}",
                if *alternate { "alternate" } else { "primary" }
            ),
        }
    }
}

impl std::error::Error for SlotError {}

/// Resolves a `part` into a [`Slot`].
///
/// `implements(implementer, interface)` says whether an implementer path
/// implements an interface. Every error is returned, not just the first, so
/// each bad alternate is named.
pub fn resolve<F>(part: &Part, implements: F) -> Result<Slot, Vec<SlotError>>
where
    F: Fn(&str, &str) -> bool,
{
    let slot = part.name.name.clone();
    let TypeKind::Path(ty) = &part.ty.kind else {
        warn!(%slot, "slot type is not an interface path");
        return Err(vec![SlotError::NotInterface { slot }]);
    };
    let interface = ty.text();
    let mut errors = Vec::new();
    let mut check = |c: &Ctor, alternate: bool| {
        let name = c.path.text();
        if implements(&name, &interface) {
            trace!(%slot, implementer = %name, %interface, alternate, "implementer accepted");
        } else {
            warn!(%slot, implementer = %name, %interface, alternate, "implementer does not implement interface");
            errors.push(SlotError::NotImplementing {
                slot: slot.clone(),
                implementer: name.clone(),
                interface: interface.clone(),
                alternate,
                span: c.span,
            });
        }
        name
    };
    let primary = part.primary.as_ref().map(|c| check(c, false));
    let alts: Vec<String> = part.alts.iter().map(|c| check(c, true)).collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    debug!(%slot, %interface, primary = ?primary, alts = alts.len(), "slot resolved");
    Ok(Slot {
        name: slot,
        interface,
        primary,
        alts,
        span: part.name.span,
    })
}

/// One instance of a slot, sharing the slot's primary and alternates.
#[derive(Clone, Debug)]
pub struct Instance {
    /// The instance path: `wheel[2]` for an array slot, `wheel` for a single one.
    pub path: String,
    /// The shared slot; every instance of the slot holds the same allocation.
    pub slot: Arc<Slot>,
}

/// Builds the instance paths of a slot; `count` is `None` for a non-array slot.
pub fn instances(slot: &Arc<Slot>, count: Option<u64>) -> Vec<Instance> {
    let paths: Vec<String> = match count {
        None => vec![slot.name.clone()],
        Some(n) => (0..n).map(|i| format!("{}[{i}]", slot.name)).collect(),
    };
    debug!(slot = %slot.name, instances = paths.len(), "instance paths built");
    paths
        .into_iter()
        .map(|path| Instance {
            path,
            slot: Arc::clone(slot),
        })
        .collect()
}

/// Why a `default` attribute could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefaultError {
    /// Not `default(Iface = Impl)`.
    Shape,
}

impl fmt::Display for DefaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "default takes `Iface = Impl` arguments")
    }
}

impl std::error::Error for DefaultError {}

/// The `default(Iface = Impl)` attributes in scope, innermost scope last.
///
/// Defaults are keyed per interface, so several coexist in one scope; an
/// inner scope overrides an outer one for the same interface.
#[derive(Clone, Debug, Default)]
pub struct Defaults {
    scopes: Vec<BTreeMap<String, String>>,
}

impl Defaults {
    /// An empty set of defaults.
    pub fn new() -> Defaults {
        Defaults::default()
    }

    /// Enters a scope whose attributes may carry `default(...)`; other attributes are ignored.
    pub fn push_scope(&mut self, attrs: &[Attr]) -> Result<(), DefaultError> {
        let mut scope = BTreeMap::new();
        for attr in attrs.iter().filter(|a| a.name.name == "default") {
            if attr.args.is_empty() {
                warn!(span = ?attr.span, "default attribute has no arguments");
                return Err(DefaultError::Shape);
            }
            for arg in &attr.args {
                let AttrArg::Named { name, value } = arg else {
                    warn!(span = ?attr.span, "default argument is not `Iface = Impl`");
                    return Err(DefaultError::Shape);
                };
                let ExprKind::Path(p) = &value.kind else {
                    return Err(DefaultError::Shape);
                };
                debug!(iface = %name.name, imp = %p.text(), "scoped default");
                scope.insert(name.name.clone(), p.text());
            }
        }
        self.scopes.push(scope);
        Ok(())
    }

    /// Leaves the innermost scope.
    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// The default implementer for `interface` in scope, innermost first.
    pub fn lookup(&self, interface: &str) -> Option<&str> {
        self.scopes
            .iter()
            .rev()
            .find_map(|s| s.get(interface))
            .map(String::as_str)
    }

    /// Returns `set` when the parameter is set, else the scoped default for `interface`.
    pub fn fill<'a>(&'a self, interface: &str, set: Option<&'a str>) -> Option<&'a str> {
        match set {
            Some(v) => Some(v),
            None => {
                let d = self.lookup(interface);
                debug!(%interface, filled = ?d, "unset parameter filled from default");
                d
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::ItemKind;
    use cadetta_syntax::parser::parse;

    fn part(src: &str) -> Part {
        let parsed = parse(&format!("model Rover {{\n  {src}\n}}\n"));
        let item = parsed.file.items().next().expect("item");
        let ItemKind::Component(c) = &item.kind else {
            panic!("not a component")
        };
        for m in &c.members {
            if let cadetta_syntax::ast::MemberKind::Part(p) = &m.kind {
                return p.clone();
            }
        }
        panic!("no part")
    }

    fn all_caster(imp: &str, iface: &str) -> bool {
        iface == "Caster" && (imp == "Vendor4471B" || imp == "Colson2Series")
    }

    // frob:tests crates/cadetta-variants/src/slot.rs::resolve kind=unit
    #[test]
    fn slot_has_primary_and_one_alternate() {
        let p = part("part wheel[4]: Caster = Vendor4471B alt [Colson2Series]");
        let s = resolve(&p, all_caster).unwrap();
        assert_eq!(s.interface, "Caster");
        assert_eq!(s.primary.as_deref(), Some("Vendor4471B"));
        assert_eq!(s.alts, vec!["Colson2Series".to_string()]);
        assert_eq!(s.implementers().count(), 2);
    }

    // frob:tests crates/cadetta-variants/src/slot.rs::resolve kind=unit
    #[test]
    fn alternate_not_implementing_is_named() {
        let p = part("part wheel[4]: Caster = Vendor4471B alt [Bogus]");
        let e = resolve(&p, all_caster).unwrap_err();
        assert_eq!(e.len(), 1);
        assert!(
            matches!(&e[0], SlotError::NotImplementing { implementer, alternate: true, .. } if implementer == "Bogus")
        );
        assert!(e[0].to_string().contains("Bogus"));
    }

    // frob:tests crates/cadetta-variants/src/slot.rs::instances kind=unit
    #[test]
    fn instances_share_primary_and_alts() {
        let p = part("part wheel[4]: Caster = Vendor4471B alt [Colson2Series]");
        let s = Arc::new(resolve(&p, all_caster).unwrap());
        let inst = instances(&s, Some(4));
        let paths: Vec<_> = inst.iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths, ["wheel[0]", "wheel[1]", "wheel[2]", "wheel[3]"]);
        assert!(inst.iter().all(|i| Arc::ptr_eq(&i.slot, &s)));
        assert_eq!(instances(&s, None)[0].path, "wheel");
    }

    // frob:tests crates/cadetta-variants/src/slot.rs::Defaults.fill kind=unit
    #[test]
    fn scoped_default_fills_unset_parameter() {
        let parsed = parse("#![default(FastenerPolicy = AcmeShop)]\n");
        let mut d = Defaults::new();
        d.push_scope(&parsed.file.inner_attrs).unwrap();
        assert_eq!(d.fill("FastenerPolicy", None), Some("AcmeShop"));
        assert_eq!(d.fill("FastenerPolicy", Some("Mine")), Some("Mine"));
        assert_eq!(d.fill("Other", None), None);
        let inner = parse("#![default(FastenerPolicy = Local)]\n");
        d.push_scope(&inner.file.inner_attrs).unwrap();
        assert_eq!(d.lookup("FastenerPolicy"), Some("Local"));
        d.pop_scope();
        assert_eq!(d.lookup("FastenerPolicy"), Some("AcmeShop"));
    }
}
