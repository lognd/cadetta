//! Locator field validation, `[*]` divergence (R012) and native-name collisions (R013).
//!
//! Specification: `docs/spec/adapters.md` 3; `docs/spec/ir.md` 3; `docs/spec/lints.md` 3 R.
// frob:ticket 01M4SWMCDEH4NF2SBVCRGE5VV9

use std::collections::BTreeMap;
use std::fmt;

use cadetta_ir::model::{Binding, Locator, LocatorKind, LocatorMode, TolerancePolicy};
use cadetta_ir::path::Path;

use crate::rpc::DimTolerance;
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
    /// `tolerance` is set on a locator that is not a `dimension`.
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    ToleranceNotDimension,
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
            Self::ToleranceNotDimension => {
                write!(f, "`tolerance` applies to a dimension locator only")
            }
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
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    if l.tolerance.is_some() && l.kind != LocatorKind::Dimension {
        return Err(LocatorError::ToleranceNotDimension);
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

/// The shape of a path's design tolerance, with the limits the emit point resolves to.
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesignForm {
    /// Untoleranced.
    None,
    /// `[+-t]`.
    Symmetric(String),
    /// Asymmetric bracket: upper and lower deviations.
    Bilateral(String, String),
    /// An interval value.
    Interval,
    /// An ISO 286 class (or fit side), with its deviations.
    Class {
        /// The class name, e.g. `H7`.
        class: String,
        /// Upper deviation.
        upper: String,
        /// Lower deviation.
        lower: String,
    },
}

/// A path's design tolerance as the planner sees it (adapters.md 3.6).
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignTolerance {
    /// The authored shape.
    pub form: DesignForm,
    /// Lower limit (`.min`) as a native-unit literal.
    pub min: String,
    /// Upper limit (`.max`) as a native-unit literal.
    pub max: String,
}

/// What the adapter and the policy say about writing a tolerance.
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TolerancePlan {
    /// The `[[adapter]] tolerances` default.
    pub adapter_default: TolerancePolicy,
    /// The adapter reported the `tolerances` feature at `initialize`.
    pub has_feature: bool,
    /// The emit point is `nominal` (quantities.md 5.6).
    pub nominal_emit: bool,
    /// The tool has a native fit tolerance that can name a class.
    pub tool_has_fit: bool,
}

/// The tolerance to push with a locator's value, or `None` when none is written (adapters.md 3.6).
///
/// Only `dimension` locators carry one (a `ref`-mode dimension takes it on the dimension, never
/// the global); a non-nominal emit point writes limits instead of a class or deviations.
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
pub fn plan_tolerance(
    l: &Locator,
    plan: &TolerancePlan,
    design: &DesignTolerance,
) -> Option<DimTolerance> {
    if l.kind != LocatorKind::Dimension || !plan.has_feature {
        return None;
    }
    if l.tolerance.unwrap_or(plan.adapter_default) == TolerancePolicy::Ignore {
        tracing::debug!(locator = %l.locator, "tolerance ignored by policy");
        return None;
    }
    let limits = || DimTolerance::Limits {
        min: design.min.clone(),
        max: design.max.clone(),
    };
    let out = match &design.form {
        DesignForm::None => None,
        _ if !plan.nominal_emit => Some(limits()),
        DesignForm::Interval => Some(limits()),
        DesignForm::Symmetric(t) => Some(DimTolerance::Symmetric { t: t.clone() }),
        DesignForm::Bilateral(u, lo) => Some(DimTolerance::Bilateral {
            upper: u.clone(),
            lower: lo.clone(),
        }),
        DesignForm::Class { class, .. } if plan.tool_has_fit => Some(DimTolerance::Fit {
            class: class.clone(),
        }),
        DesignForm::Class { upper, lower, .. } => Some(DimTolerance::Bilateral {
            upper: upper.clone(),
            lower: lower.clone(),
        }),
    };
    tracing::debug!(locator = %l.locator, tolerance = ?out, "tolerance planned");
    out
}

/// R012 (reserved, dormant; unreachable from source) for a `[*]` binding: `values` are the emitted values of instances 0, 1, ...
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
        LocatorKind::Instance => "instance",
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
            implicit: None,
            tolerance: None,
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

    fn dim(tol: Option<TolerancePolicy>) -> Locator {
        Locator {
            tolerance: tol,
            ..loc(LocatorKind::Dimension, LocatorMode::Ref)
        }
    }

    fn plan(has_feature: bool, nominal_emit: bool, tool_has_fit: bool) -> TolerancePlan {
        TolerancePlan {
            adapter_default: TolerancePolicy::Drive,
            has_feature,
            nominal_emit,
            tool_has_fit,
        }
    }

    fn design(form: DesignForm) -> DesignTolerance {
        DesignTolerance {
            form,
            min: "11.9".into(),
            max: "12.1".into(),
        }
    }

    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    // frob:tests tolerance_forms_map_per_spec
    #[test]
    fn tolerance_forms_map_per_spec() {
        let p = plan(true, true, true);
        let d = dim(None);
        assert_eq!(
            plan_tolerance(&d, &p, &design(DesignForm::Symmetric("0.1".into()))),
            Some(DimTolerance::Symmetric { t: "0.1".into() })
        );
        assert_eq!(
            plan_tolerance(
                &d,
                &p,
                &design(DesignForm::Bilateral("0.2".into(), "-0.1".into()))
            ),
            Some(DimTolerance::Bilateral {
                upper: "0.2".into(),
                lower: "-0.1".into()
            })
        );
        let lim = Some(DimTolerance::Limits {
            min: "11.9".into(),
            max: "12.1".into(),
        });
        assert_eq!(plan_tolerance(&d, &p, &design(DesignForm::Interval)), lim);
        let class = DesignForm::Class {
            class: "H7".into(),
            upper: "0.018".into(),
            lower: "0".into(),
        };
        assert_eq!(
            plan_tolerance(&d, &p, &design(class.clone())),
            Some(DimTolerance::Fit { class: "H7".into() })
        );
        assert_eq!(
            plan_tolerance(&d, &plan(true, true, false), &design(class.clone())),
            Some(DimTolerance::Bilateral {
                upper: "0.018".into(),
                lower: "0".into()
            })
        );
        assert_eq!(plan_tolerance(&d, &p, &design(DesignForm::None)), None);
        // non-nominal emit point writes limits
        assert_eq!(
            plan_tolerance(&d, &plan(true, false, true), &design(class)),
            lim
        );
        assert_eq!(
            plan_tolerance(
                &d,
                &plan(true, false, true),
                &design(DesignForm::Symmetric("0.1".into()))
            ),
            lim
        );
    }

    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    // frob:tests tolerance_gated_by_feature_policy_and_kind
    #[test]
    fn tolerance_gated_by_feature_policy_and_kind() {
        let sym = design(DesignForm::Symmetric("0.1".into()));
        assert!(plan_tolerance(&dim(None), &plan(false, true, true), &sym).is_none());
        let ign = dim(Some(TolerancePolicy::Ignore));
        assert!(plan_tolerance(&ign, &plan(true, true, true), &sym).is_none());
        let mut ignoring_default = plan(true, true, true);
        ignoring_default.adapter_default = TolerancePolicy::Ignore;
        assert!(plan_tolerance(&dim(None), &ignoring_default, &sym).is_none());
        assert!(
            plan_tolerance(&dim(Some(TolerancePolicy::Drive)), &ignoring_default, &sym).is_some()
        );
        let global = loc(LocatorKind::Global, LocatorMode::Value);
        assert!(plan_tolerance(&global, &plan(true, true, true), &sym).is_none());
    }

    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    // frob:tests tolerance_on_non_dimension_rejected
    #[test]
    fn tolerance_on_non_dimension_rejected() {
        let bad = "[\"a.b::C.d\"]\nlocators = [{ locator = \"x\", kind = \"global\", tolerance = \"drive\" }]";
        let e = BindingSet::load_str("sw", bad).expect_err("global");
        assert!(e.to_string().contains("tolerance"), "{e}");
        let ok = "[\"a.b::C.d\"]\nlocators = [{ locator = \"x\", kind = \"dimension\", tolerance = \"ignore\" }]";
        assert_eq!(
            set(ok).entries()[0].locators[0].tolerance,
            Some(TolerancePolicy::Ignore)
        );
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
