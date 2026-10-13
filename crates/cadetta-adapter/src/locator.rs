//! Locator field validation, `[*]` divergence (R012) and native-name collisions (R013).
//!
//! Specification: `docs/spec/adapters.md` 3; `docs/spec/ir.md` 3; `docs/spec/lints.md` 3 R.
// frob:ticket 01M4SWMCDEH4NF2SBVCRGE5VV9

use std::collections::BTreeMap;
use std::fmt;

use cadetta_ir::model::{Binding, Locator, LocatorKind, LocatorMode};
use cadetta_ir::path::Path;

use crate::{AdapterDiagnostic, AdapterError, Severity};

/// Diagnostic code for a `[*]` binding whose instances emit different values (adapters.md 3).
pub const R012: &str = "R012";
/// Diagnostic code for one native name created from two paths (adapters.md 3.1).
pub const R013: &str = "R013";

/// A locator field that fails validation (adapters.md 3 field table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocatorError {
    /// `locator` is empty.
    EmptyLocator,
    /// `ref_to` is set on a locator that is not a `ref`-mode dimension.
    RefToWithoutRef,
    /// `sign` is set on a `property` locator (not a coordinate).
    SignOnProperty,
    /// `component` is not of the form `name<N>`.
    BadComponent(String),
    /// `for` or `config` is present but empty.
    EmptyField(&'static str),
}

impl fmt::Display for LocatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLocator => write!(f, "locator text is empty"),
            Self::RefToWithoutRef => {
                write!(f, "`ref_to` needs a dimension locator in `ref` mode")
            }
            Self::SignOnProperty => write!(f, "`sign` does not apply to a property locator"),
            Self::BadComponent(c) => write!(f, "component `{c}` is not of the form `name<N>`"),
            Self::EmptyField(n) => write!(f, "`{n}` is present but empty"),
        }
    }
}

impl std::error::Error for LocatorError {}

/// Validates the cross-field rules of one locator.
pub fn validate(l: &Locator) -> Result<(), LocatorError> {
    let r = validate_inner(l);
    if let Err(e) = &r {
        tracing::error!(locator = %l.locator, error = %e, "locator: invalid");
    }
    r
}

fn validate_inner(l: &Locator) -> Result<(), LocatorError> {
    if l.locator.trim().is_empty() {
        return Err(LocatorError::EmptyLocator);
    }
    if l.ref_to.is_some() && !(l.kind == LocatorKind::Dimension && l.mode == LocatorMode::Ref) {
        return Err(LocatorError::RefToWithoutRef);
    }
    if l.sign.is_some() && l.kind == LocatorKind::Property {
        return Err(LocatorError::SignOnProperty);
    }
    if l.for_alternate.as_deref().is_some_and(str::is_empty) {
        return Err(LocatorError::EmptyField("for"));
    }
    if l.config.as_deref().is_some_and(str::is_empty) {
        return Err(LocatorError::EmptyField("config"));
    }
    if let Some(c) = &l.component {
        let ok = c
            .strip_suffix('>')
            .and_then(|s| s.split_once('<'))
            .is_some_and(|(n, i)| {
                !n.is_empty() && !i.is_empty() && i.bytes().all(|b| b.is_ascii_digit())
            });
        if !ok {
            return Err(LocatorError::BadComponent(c.clone()));
        }
    }
    Ok(())
}

/// R012 for a `[*]` binding: `values` are the emitted values of instances 0, 1, ...
///
/// Returns an error diagnostic naming the first instance that differs from instance 0.
pub fn check_star_divergence<T: PartialEq + fmt::Display>(
    path: &Path,
    values: &[T],
) -> Option<AdapterDiagnostic> {
    let (first, rest) = values.split_first()?;
    let (i, v) = rest.iter().enumerate().find(|(_, v)| *v != first)?;
    tracing::warn!(%path, instance = i + 1, "locator: [*] instances diverge (R012)");
    Some(AdapterDiagnostic {
        code: R012.to_owned(),
        severity: Severity::Error,
        message: format!(
            "`{path}` emits {first} at instance 0 but {v} at instance {}; bind the instances separately",
            i + 1
        ),
    })
}

/// The namespace a locator's native name lives in (names never collide across namespaces).
fn namespace(kind: LocatorKind) -> &'static str {
    match kind {
        LocatorKind::Global => "global",
        LocatorKind::Dimension => "dimension",
        LocatorKind::Property => "property",
        LocatorKind::Frame => "frame",
    }
}

/// The namespace-qualified native name, e.g. `global:Plate_Thk` (adapters.md 3).
pub fn qualified_name(l: &Locator) -> String {
    format!("{}:{}", namespace(l.kind), l.locator)
}

/// Qualified name, configuration and component: the identity of one native name.
type OwnerKey<'a> = (String, Option<&'a str>, Option<&'a str>);

/// R013: native names that two different paths would address, per namespace and configuration.
///
/// The same name in two namespaces is not a collision. Returns `AdapterError::Failed` so the
/// emit stops before any write.
pub fn check_native_collisions(bindings: &[Binding]) -> Result<(), AdapterError> {
    let mut seen: BTreeMap<OwnerKey<'_>, Vec<&Path>> = BTreeMap::new();
    for b in bindings {
        for l in &b.locators {
            let owners = seen
                .entry((
                    qualified_name(l),
                    l.config.as_deref(),
                    l.component.as_deref(),
                ))
                .or_default();
            if !owners.contains(&&b.path) {
                owners.push(&b.path);
            }
        }
    }
    let diags: Vec<AdapterDiagnostic> = seen
        .into_iter()
        .filter(|(_, p)| p.len() > 1)
        .map(|((name, _, _), paths)| {
            let list: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
            tracing::error!(%name, "locator: native name collision (R013)");
            AdapterDiagnostic {
                code: R013.to_owned(),
                severity: Severity::Error,
                message: format!("native name `{name}` is created by {}", list.join(", ")),
            }
        })
        .collect();
    if diags.is_empty() {
        Ok(())
    } else {
        Err(AdapterError::Failed(diags))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::BindingSet;
    use cadetta_ir::path::PathContext;

    fn loc(kind: LocatorKind, mode: LocatorMode) -> Locator {
        Locator {
            locator: "X".into(),
            kind,
            mode,
            native_unit: String::new(),
            ref_to: None,
            for_alternate: None,
            config: None,
            sign: None,
            component: None,
        }
    }

    fn set(text: &str) -> BindingSet {
        BindingSet::load_str("sw", text).expect("loads")
    }

    #[test]
    fn locator_fields_validated() {
        let ok = "[\"a.b::C.d\"]\nlocators = [{ locator = \"x\", kind = \"dimension\", ref_to = \"g\", config = \"Default\", component = \"wheel<2>\" }, { locator = \"f\", kind = \"frame\", sign = -1, for = \"Alt\" }]";
        assert_eq!(set(ok).entries()[0].locators.len(), 2);
        for (bad, want) in [
            (
                "{ locator = \"x\", kind = \"global\", ref_to = \"g\" }",
                "ref_to",
            ),
            (
                "{ locator = \"x\", kind = \"property\", sign = -1 }",
                "sign",
            ),
            (
                "{ locator = \"x\", kind = \"global\", component = \"wheel\" }",
                "component",
            ),
            ("{ locator = \" \", kind = \"global\" }", "empty"),
            (
                "{ locator = \"x\", kind = \"global\", config = \"\" }",
                "config",
            ),
        ] {
            let text = format!("[\"a.b::C.d\"]\nlocators = [{bad}]");
            let e = BindingSet::load_str("sw", &text).expect_err(want);
            assert!(e.to_string().contains(want), "{e}");
        }
    }

    #[test]
    fn star_divergence_fires_r012() {
        let p = Path::parse("a.b::Rover/axle[*].end_diameter", PathContext::Binding).unwrap();
        assert!(check_star_divergence(&p, &[10, 10, 10]).is_none());
        assert!(check_star_divergence::<i32>(&p, &[]).is_none());
        let d = check_star_divergence(&p, &[10, 10, 12]).unwrap();
        assert_eq!(d.code, "R012");
        assert_eq!(d.severity, Severity::Error);
        assert!(d.message.contains("instance 2"));
    }

    #[test]
    fn native_name_from_two_paths_fires_r013() {
        let s = set(
            "[\"a.b::C.d\"]\nlocators = [{ locator = \"T\", kind = \"global\" }]\n[\"a.b::C.e\"]\nlocators = [{ locator = \"T\", kind = \"global\" }]",
        );
        match check_native_collisions(s.entries()) {
            Err(AdapterError::Failed(ds)) => {
                assert_eq!(ds.len(), 1);
                assert_eq!(ds[0].code, "R013");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn same_name_in_two_namespaces_is_qualified() {
        let s = set(
            "[\"a.b::C.d\"]\nlocators = [{ locator = \"T\", kind = \"global\" }]\n[\"a.b::C.e\"]\nlocators = [{ locator = \"T\", kind = \"property\" }]",
        );
        assert_eq!(check_native_collisions(s.entries()), Ok(()));
        let a = loc(LocatorKind::Global, LocatorMode::Value);
        let b = loc(LocatorKind::Property, LocatorMode::Value);
        assert_eq!(qualified_name(&a), "global:X");
        assert_ne!(qualified_name(&a), qualified_name(&b));
    }
}
