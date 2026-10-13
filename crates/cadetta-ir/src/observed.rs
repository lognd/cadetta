//! Observed IR: the flavour an adapter `read` produces (ir.md 1, adapters.md 3).
//!
//! Observed IR holds parameters with values, native names, locators and, where the native
//! format exposes them, expressions. Design-only fields (citations, parts, fits, checks,
//! solvers) stay empty, so the value lints (Q, N, D001, M001) read it through [`value_members`].
// frob:ticket 01M4ST7B09TGN6VSXJBZ8FDVCN

use std::collections::BTreeSet;

use cadetta_units::Dim;
use tracing::{debug, trace, warn};

use crate::model::{
    Binding, Component, ComponentClass, Flavour, Locator, Member, MemberKind, Nominal, PackageId,
    Project, Provenance, Value,
};
use crate::path::{Path, PathContext, PathError};

/// The type name of the synthetic component that holds every observed parameter.
pub const OBSERVED_COMPONENT: &str = "Observed";

/// A failure while building observed IR.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ObservedError {
    /// The derived canonical path was rejected.
    #[error("observed parameter `{native}`: {source}")]
    Path {
        /// The native parameter name.
        native: String,
        /// The path failure.
        source: PathError,
    },
}

/// One parameter as an adapter `read` reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedParam {
    /// The native parameter name (`"A"`, `D1@Sketch1`).
    pub native_name: String,
    /// The native address.
    pub locator: Locator,
    /// The value read.
    pub nominal: Nominal,
    /// Its dimension.
    pub dim: Dim,
    /// The value text as the native tool shows it.
    pub text: String,
    /// The native expression (`"B" * 2`), when the format exposes one.
    pub expr: Option<String>,
    /// The unit taken from the adapter's declared native unit or `--assume-unit`.
    pub unit_assumed: Option<String>,
    /// Why the value is not faithful (sets `exact: false`).
    pub inexact_reason: Option<String>,
}

/// Builds an observed [`Project`] from adapter read results.
#[derive(Debug)]
pub struct ObservedBuilder {
    project: Project,
    adapter: String,
    file: String,
    comp_path: Path,
    members: Vec<Member>,
    names: BTreeSet<String>,
}

/// Map a native name to a path-safe identifier (non-alphanumerics become `_`).
fn ident(native: &str) -> String {
    let mut s: String = native
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    s
}

impl ObservedBuilder {
    /// Start observed IR for `package`, read by `adapter` from `file`.
    pub fn new(
        package: PackageId,
        adapter: impl Into<String>,
        file: impl Into<String>,
    ) -> Result<Self, ObservedError> {
        let comp_path = Path::parse(
            &format!("{}::{OBSERVED_COMPONENT}.value", package.name),
            PathContext::General,
        )
        .map_err(|source| ObservedError::Path {
            native: OBSERVED_COMPONENT.to_owned(),
            source,
        })?;
        Ok(Self {
            project: Project::new(Flavour::Observed, package),
            adapter: adapter.into(),
            file: file.into(),
            comp_path,
            members: Vec::new(),
            names: BTreeSet::new(),
        })
    }

    /// Add one read parameter as a member, binding and imported provenance.
    pub fn push(&mut self, p: ObservedParam) -> Result<(), ObservedError> {
        let mut name = ident(&p.native_name);
        let mut n = 1u32;
        while !self.names.insert(name.clone()) {
            n += 1;
            name = format!("{}_{n}", ident(&p.native_name));
        }
        let text = format!("{}::{OBSERVED_COMPONENT}.{name}", self.project.package.name);
        let path =
            Path::parse(&text, PathContext::General).map_err(|source| ObservedError::Path {
                native: p.native_name.clone(),
                source,
            })?;
        let mut value = Value::new(p.nominal, p.dim, p.text);
        value.unit_assumed = p.unit_assumed;
        if let Some(reason) = p.inexact_reason {
            warn!(native = %p.native_name, %reason, "observed: value not faithful");
            value.exact = false;
            value.reason = Some(reason);
        }
        let mut m = Member::new(path.clone(), MemberKind::Param);
        m.value = Some(value);
        m.expr = p.expr;
        m.provenance = Provenance::Imported {
            adapter: self.adapter.clone(),
            file: self.file.clone(),
            native_name: p.native_name.clone(),
        };
        trace!(native = %p.native_name, path = %path.to_string(), "observed: member added");
        self.project.bindings.push(Binding {
            path,
            adapter: self.adapter.clone(),
            locators: vec![p.locator],
        });
        self.members.push(m);
        Ok(())
    }

    /// Finish: one synthetic component holds every member.
    pub fn build(mut self) -> Project {
        debug!(members = self.members.len(), "observed: project built");
        self.project.components.push(Component {
            path: self.comp_path,
            class: ComponentClass::Model,
            params_sig: None,
            implements: Vec::new(),
            citation: None,
            attrs: Default::default(),
            members: self.members,
            parts: Vec::new(),
            fits: Vec::new(),
            span: None,
            digest: None,
        });
        self.project
    }
}

/// Every member that carries a value, on either flavour: the input of the value lints.
pub fn value_members(project: &Project) -> Vec<(&Path, &Value)> {
    project
        .components
        .iter()
        .flat_map(|c| c.members.iter())
        .filter_map(|m| m.value.as_ref().map(|v| (&m.path, v)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Exact, LocatorKind, LocatorMode};
    use cadetta_units::{Magnitude, Rational};

    fn param(name: &str, n: i64, expr: Option<&str>) -> ObservedParam {
        ObservedParam {
            native_name: name.into(),
            locator: Locator {
                locator: name.into(),
                kind: LocatorKind::Global,
                mode: LocatorMode::Value,
                native_unit: "mm".into(),
                ref_to: None,
                for_alternate: None,
                config: None,
                sign: None,
                component: None,
            },
            nominal: Nominal::Exact(Exact(Magnitude::new(Rational::new(n, 1).unwrap(), 0))),
            dim: Dim::default(),
            text: n.to_string(),
            expr: expr.map(str::to_owned),
            unit_assumed: Some("mm".into()),
            inexact_reason: None,
        }
    }

    fn builder() -> ObservedBuilder {
        let pkg = PackageId {
            name: "acme.rover".into(),
            version: "0.0.0".into(),
        };
        ObservedBuilder::new(pkg, "solidworks", "axle.sldprt").unwrap()
    }

    // frob:tests observed_param_fields
    #[test]
    fn observed_param_fields() {
        let mut b = builder();
        let mut p = param("D1@Sketch1", 5, None);
        p.inexact_reason = Some("rounded by tool".into());
        b.push(p).unwrap();
        let pr = b.build();
        assert_eq!(pr.flavour, Flavour::Observed);
        let m = &pr.components[0].members[0];
        let v = m.value.as_ref().unwrap();
        assert!(!v.exact);
        assert_eq!(v.reason.as_deref(), Some("rounded by tool"));
        assert_eq!(v.unit_assumed.as_deref(), Some("mm"));
        assert!(matches!(&m.provenance,
            Provenance::Imported { native_name, .. } if native_name == "D1@Sketch1"));
        assert_eq!(pr.bindings[0].locators[0].locator, "D1@Sketch1");
        assert_eq!(pr.bindings[0].path, m.path);
    }

    // frob:tests observed_expression_capture
    #[test]
    fn observed_expression_capture() {
        let mut b = builder();
        b.push(param("A", 10, Some("\"B\" * 2"))).unwrap();
        b.push(param("B", 5, None)).unwrap();
        let pr = b.build();
        let ms = &pr.components[0].members;
        assert_eq!(ms[0].expr.as_deref(), Some("\"B\" * 2"));
        assert_eq!(ms[1].expr, None);
    }

    // frob:tests observed_value_lint_input
    #[test]
    fn observed_value_lint_input() {
        let mut b = builder();
        b.push(param("A", 1, None)).unwrap();
        b.push(param("A", 2, None)).unwrap();
        let pr = b.build();
        let vs = value_members(&pr);
        assert_eq!(vs.len(), 2);
        assert_ne!(vs[0].0, vs[1].0);
        assert!(pr.citations.is_empty() && pr.checks.is_empty() && pr.solvers.is_empty());
        assert!(pr.components[0].parts.is_empty() && pr.components[0].fits.is_empty());
    }
}
