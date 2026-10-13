//! Variant names `SLOT=ALT`: the slot's instance path and the alternate's component path.
//!
//! Spec: `docs/spec/ir.md` 2. The full form (`acme.rover::Rover/wheel=acme.rover::Colson2Series`)
//! is what IR keys and lock records use; the short form (`wheel=Colson2Series`) appears in
//! source and diagnostics and is expanded against a [`Scope`] by [`ShortVariant::resolve`].
//! Item-level paths reuse the canonical grammar of [`crate::path`] (a dummy member is appended
//! so `Path::parse` accepts them, then removed), so there is one grammar in the repo.
// frob:ticket 01M4SWK9FZW89QXVMGTSA2RC7Z

use std::fmt;

use crate::path::{Path, PathContext, PathError, Step};

const DUMMY_MEMBER: &str = ".x";

/// A variant-name parse or resolution failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VariantError {
    /// No single `=` separating slot from alternate.
    #[error("variant `{0}` must have the form SLOT=ALT")]
    NotSlotAlt(String),
    /// A side is not a valid path.
    #[error("bad variant path: {0}")]
    Path(#[from] PathError),
    /// The slot path must name an instance step (`Rover/wheel`), the alternate a bare component.
    #[error("variant `{0}`: {1}")]
    Shape(String, String),
    /// The short alternate matched no candidate component.
    #[error("variant alternate `{0}` matches no component in scope")]
    UnknownAlt(String),
    /// The short alternate matched several candidate components.
    #[error("variant alternate `{0}` is ambiguous: {1}")]
    AmbiguousAlt(String, String),
}

/// A component path `PKG::mods::Item`, without instance steps or members.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentPath {
    /// The dotted package name.
    pub package: String,
    /// Module segments between package and item.
    pub modules: Vec<String>,
    /// The component name.
    pub item: String,
}

/// A slot instance path `PKG::mods::Assembly/part[idx]...`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SlotPath {
    /// The assembly component the slot lives in.
    pub assembly: ComponentPath,
    /// The instance steps down to the slot; never empty.
    pub steps: Vec<Step>,
}

/// A full-form variant name: `SLOT=ALT`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantName {
    /// The slot's instance path.
    pub slot: SlotPath,
    /// The alternate's component path.
    pub alt: ComponentPath,
}

/// The module and assembly a short-form name is written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// The assembly whose slots short names refer to.
    pub assembly: ComponentPath,
    /// Components visible as alternates (same module, imports, prelude), as full paths.
    pub candidates: Vec<ComponentPath>,
}

/// A short-form variant name: `wheel=Colson2Series`, not yet resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortVariant {
    /// Slot steps relative to the scope's assembly.
    pub slot: Vec<Step>,
    /// Optional module qualifier on the alternate (`drive::motor`), empty if bare.
    pub alt_modules: Vec<String>,
    /// The alternate component name.
    pub alt_item: String,
}

fn parse_item_path(text: &str) -> Result<Path, VariantError> {
    let p = Path::parse(&format!("{text}{DUMMY_MEMBER}"), PathContext::General)?;
    Ok(p)
}

fn component_of(p: &Path) -> ComponentPath {
    ComponentPath {
        package: p.package.clone(),
        modules: p.modules.clone(),
        item: p.item.clone(),
    }
}

impl ComponentPath {
    /// Parse `PKG::mods::Item`.
    pub fn parse(text: &str) -> Result<Self, VariantError> {
        let p = parse_item_path(text)?;
        if !p.steps.is_empty() {
            return Err(VariantError::Shape(
                text.to_owned(),
                "a component path has no `/` steps".into(),
            ));
        }
        Ok(component_of(&p))
    }
}

impl fmt::Display for ComponentPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.package)?;
        for m in &self.modules {
            write!(f, "::{m}")?;
        }
        write!(f, "::{}", self.item)
    }
}

fn write_steps(f: &mut fmt::Formatter<'_>, steps: &[Step]) -> fmt::Result {
    for s in steps {
        write!(f, "/{}", s.name)?;
        if let Some(i) = &s.index {
            write!(f, "[{i}]")?;
        }
    }
    Ok(())
}

impl fmt::Display for SlotPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.assembly)?;
        write_steps(f, &self.steps)
    }
}

impl fmt::Display for VariantName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.slot, self.alt)
    }
}

fn split_slot_alt(text: &str) -> Result<(&str, &str), VariantError> {
    match text.split_once('=') {
        Some((s, a)) if !s.is_empty() && !a.is_empty() && !a.contains('=') => Ok((s, a)),
        _ => {
            tracing::debug!(variant = text, "variant: not SLOT=ALT");
            Err(VariantError::NotSlotAlt(text.to_owned()))
        }
    }
}

impl VariantName {
    /// Parse the full form `PKG::Asm/slot=PKG::Alt`, separating slot and alternate paths.
    pub fn parse(text: &str) -> Result<Self, VariantError> {
        let (s, a) = split_slot_alt(text)?;
        let sp = parse_item_path(s)?;
        if sp.steps.is_empty() {
            return Err(VariantError::Shape(
                text.to_owned(),
                "the slot must be an instance path with a `/slot` step".into(),
            ));
        }
        let v = VariantName {
            slot: SlotPath {
                assembly: component_of(&sp),
                steps: sp.steps,
            },
            alt: ComponentPath::parse(a)?,
        };
        tracing::trace!(variant = text, "variant: parsed full form");
        Ok(v)
    }

    /// The short form: slot steps and bare alternate name, without package, modules or assembly.
    pub fn short(&self) -> String {
        let mut s = String::new();
        for (i, st) in self.slot.steps.iter().enumerate() {
            if i > 0 {
                s.push('/');
            }
            s.push_str(&st.name);
            if let Some(ix) = &st.index {
                s.push_str(&format!("[{ix}]"));
            }
        }
        format!("{s}={}", self.alt.item)
    }
}

impl std::str::FromStr for VariantName {
    type Err = VariantError;
    /// Parse the full form.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        VariantName::parse(s)
    }
}

impl ShortVariant {
    /// Parse the short form `slot[/slot...]=[mods::]Item`.
    pub fn parse(text: &str) -> Result<Self, VariantError> {
        let (s, a) = split_slot_alt(text)?;
        // Reuse the canonical grammar with a placeholder package and assembly.
        let sp = parse_item_path(&format!("p::A/{s}"))?;
        let mut parts: Vec<&str> = a.split("::").collect();
        let item = parts.pop().unwrap_or_default();
        let ap = parse_item_path(&format!("p::{}{}{item}", parts.join("::"), {
            if parts.is_empty() { "" } else { "::" }
        }))?;
        if !ap.steps.is_empty() {
            return Err(VariantError::Shape(
                text.to_owned(),
                "an alternate has no `/` steps".into(),
            ));
        }
        Ok(ShortVariant {
            slot: sp.steps,
            alt_modules: ap.modules,
            alt_item: ap.item,
        })
    }

    /// Expand to the full form: the slot hangs off `scope.assembly`; the alternate must match
    /// exactly one candidate by item name (and module suffix if qualified).
    pub fn resolve(&self, scope: &Scope) -> Result<VariantName, VariantError> {
        let hits: Vec<&ComponentPath> = scope
            .candidates
            .iter()
            .filter(|c| c.item == self.alt_item && c.modules.ends_with(&self.alt_modules))
            .collect();
        let shown = format!("{}{}", qual(&self.alt_modules), self.alt_item);
        match hits.as_slice() {
            [one] => Ok(VariantName {
                slot: SlotPath {
                    assembly: scope.assembly.clone(),
                    steps: self.slot.clone(),
                },
                alt: (*one).clone(),
            }),
            [] => {
                tracing::warn!(alt = %shown, "variant: alternate matches no component");
                Err(VariantError::UnknownAlt(shown))
            }
            many => {
                let list = many
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                tracing::warn!(alt = %shown, %list, "variant: ambiguous alternate");
                Err(VariantError::AmbiguousAlt(shown, list))
            }
        }
    }
}

fn qual(mods: &[String]) -> String {
    mods.iter().map(|m| format!("{m}::")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = "acme.rover::Rover/wheel=acme.rover::Colson2Series";

    fn cp(s: &str) -> ComponentPath {
        ComponentPath::parse(s).unwrap()
    }

    fn scope(cands: &[&str]) -> Scope {
        Scope {
            assembly: cp("acme.rover::Rover"),
            candidates: cands.iter().map(|c| cp(c)).collect(),
        }
    }

    // frob:tests crates/cadetta-ir/src/variant_name.rs::VariantName.parse
    #[test]
    fn full_form_separates_slot_and_alternate() {
        let v: VariantName = FULL.parse().unwrap();
        assert_eq!(v.slot.assembly, cp("acme.rover::Rover"));
        assert_eq!(v.slot.steps.len(), 1);
        assert_eq!(v.slot.steps[0].name, "wheel");
        assert_eq!(v.alt, cp("acme.rover::Colson2Series"));
        assert_eq!(v.to_string(), FULL);
        let idx = "acme.rover::Rover/wheel[2]=acme.rover::drive::Colson";
        assert_eq!(VariantName::parse(idx).unwrap().to_string(), idx);
    }

    // frob:tests crates/cadetta-ir/src/variant_name.rs::VariantName.parse
    #[test]
    fn malformed_variants_are_rejected() {
        for bad in [
            "acme.rover::Rover/wheel",
            "acme.rover::Rover=acme.rover::C",
            "acme.rover::Rover/wheel=acme.rover::C/x",
            "acme.rover::Rover/wheel=C",
            "wheel=Colson2Series",
            "a=b=c",
            "=acme.rover::C",
        ] {
            assert!(VariantName::parse(bad).is_err(), "{bad}");
        }
    }

    // frob:tests crates/cadetta-ir/src/variant_name.rs::ShortVariant.resolve
    #[test]
    fn short_form_resolves_to_full_form() {
        let s = ShortVariant::parse("wheel=Colson2Series").unwrap();
        let v = s
            .resolve(&scope(&["acme.rover::Colson2Series", "acme.rover::Other"]))
            .unwrap();
        assert_eq!(v.to_string(), FULL);
        assert_eq!(v.short(), "wheel=Colson2Series");
        assert_eq!(VariantName::parse(&v.to_string()).unwrap(), v);
    }

    // frob:tests crates/cadetta-ir/src/variant_name.rs::ShortVariant.resolve
    #[test]
    fn short_form_ambiguity_and_qualification() {
        let cands = scope(&["acme.rover::a::Colson", "acme.rover::b::Colson"]);
        let bare = ShortVariant::parse("wheel=Colson").unwrap();
        assert!(matches!(
            bare.resolve(&cands),
            Err(VariantError::AmbiguousAlt(..))
        ));
        let q = ShortVariant::parse("wheel=b::Colson").unwrap();
        assert_eq!(
            q.resolve(&cands).unwrap().to_string(),
            "acme.rover::Rover/wheel=acme.rover::b::Colson"
        );
        let none = ShortVariant::parse("wheel=Nope").unwrap();
        assert!(matches!(
            none.resolve(&cands),
            Err(VariantError::UnknownAlt(_))
        ));
    }
}
