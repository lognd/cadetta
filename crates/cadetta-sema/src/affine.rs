//! Source-variable bookkeeping for affine tolerance evaluation (quantities.md 5.3, 5.4).
//!
//! The evaluator carries tolerance as `cadetta_units::Affine` forms over
//! independent source variables. This module owns which variable belongs to
//! which toleranced parameter of which part instance: each toleranced
//! parameter of each instance owns one variable, a pure reference shares its
//! referent's variable, and an expression-local tolerance owns an anonymous
//! one. It also records the C004 fact for a computed parameter that carries no
//! tolerance of its own while its inputs do.

// frob:ticket 01M4ST7AB8627SAJ7MGB1GSRH6

use std::collections::BTreeMap;

use cadetta_units::{Affine, VarId};
use tracing::{debug, trace};

use crate::eval::Evaluated;

/// Names one parameter of one part instance (quantities.md 5.4).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct VarKey {
    /// The instance path (`asm.ring_a`), or the component path for a bare component.
    pub instance: String,
    /// The parameter name.
    pub member: String,
}

impl VarKey {
    /// Builds a key from an instance path and a parameter name.
    pub fn new(instance: impl Into<String>, member: impl Into<String>) -> Self {
        Self {
            instance: instance.into(),
            member: member.into(),
        }
    }
}

/// Why an alias could not be bound.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AliasError {
    /// The referent owns no source variable (it is untoleranced or not yet registered).
    NoReferentVar(VarKey),
    /// The aliasing key already owns a different variable.
    AlreadyBound(VarKey),
}

/// Registry of source variables: one per toleranced instance parameter.
#[derive(Clone, Default, Debug)]
pub struct SourceVars {
    next: u32,
    by_key: BTreeMap<VarKey, VarId>,
    owner: BTreeMap<VarId, VarKey>,
}

impl SourceVars {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    fn fresh(&mut self) -> VarId {
        let v = VarId(self.next);
        self.next += 1;
        v
    }

    /// The variable `key` owns, allocating a fresh independent one on first use.
    pub fn own(&mut self, key: VarKey) -> VarId {
        if let Some(v) = self.by_key.get(&key) {
            return *v;
        }
        let v = self.fresh();
        debug!(?key, var = v.0, "affine: parameter owns a source variable");
        self.owner.insert(v, key.clone());
        self.by_key.insert(key, v);
        v
    }

    /// A fresh anonymous variable for a tolerance written inside an expression.
    pub fn anonymous(&mut self) -> VarId {
        let v = self.fresh();
        trace!(var = v.0, "affine: anonymous source variable");
        v
    }

    /// Binds `key` to the variable of `referent` (a pure reference, quantities.md 5.3).
    pub fn alias(&mut self, key: VarKey, referent: &VarKey) -> Result<VarId, AliasError> {
        let Some(v) = self.by_key.get(referent).copied() else {
            return Err(AliasError::NoReferentVar(referent.clone()));
        };
        match self.by_key.get(&key) {
            Some(existing) if *existing != v => return Err(AliasError::AlreadyBound(key)),
            _ => {}
        }
        debug!(
            ?key,
            ?referent,
            var = v.0,
            "affine: alias shares referent variable"
        );
        self.by_key.insert(key, v);
        Ok(v)
    }

    /// The variable bound to `key`, if any.
    pub fn get(&self, key: &VarKey) -> Option<VarId> {
        self.by_key.get(key).copied()
    }

    /// The key that first allocated `var`; `None` for anonymous variables.
    pub fn owner_of(&self, var: VarId) -> Option<&VarKey> {
        self.owner.get(&var)
    }

    /// Number of variables allocated so far.
    pub fn len(&self) -> usize {
        self.next as usize
    }

    /// True when no variable has been allocated.
    pub fn is_empty(&self) -> bool {
        self.next == 0
    }
}

/// The C004 fact: a check reads a computed parameter that hides upstream tolerance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct C004Fact {
    /// The parameter the check reads.
    pub param: VarKey,
}

/// Records C004 when `param` is a computed, tolerance-free parameter whose
/// expression form `expr_form` carries tolerance (quantities.md 5.3).
pub fn c004_fact(param: &VarKey, stored: &Evaluated, expr_form: &Affine) -> Option<C004Fact> {
    let computed = !stored.analysis && stored.var.is_none() && stored.alias_of.is_none();
    if computed && !expr_form.is_exact() {
        debug!(
            ?param,
            "affine: C004 fact, computed parameter treated as exact"
        );
        Some(C004Fact {
            param: param.clone(),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Val;
    use cadetta_syntax::span::Span;
    use cadetta_units::{Dim, Dimension, Magnitude, Rational, Toleranced};

    fn dim() -> Dim {
        Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
    }

    fn mag(n: i64) -> Magnitude {
        Magnitude::new(Rational::from_int(n), 0)
    }

    fn form(nominal: i64, dev: i64, var: VarId) -> Affine {
        Toleranced::symmetric(dim(), mag(nominal), (dim(), mag(dev)))
            .unwrap()
            .to_affine(var)
            .unwrap()
    }

    fn evaluated(var: Option<VarId>, analysis: bool, alias: bool) -> Evaluated {
        let q = crate::eval::Quant::constant(dim(), mag(1));
        Evaluated {
            value: Val::Quantity(q),
            analysis,
            var,
            alias_of: alias.then(unreachable_path),
            span: Span::default(),
            role: None,
        }
    }

    fn unreachable_path() -> crate::resolve::CanonPath {
        crate::resolve::CanonPath::item("p", &["m".to_owned()], "x")
    }

    // frob:tests SourceVars.own
    // frob:tests SourceVars.is_empty
    #[test]
    fn each_instance_parameter_is_independent() {
        let mut s = SourceVars::new();
        assert!(s.is_empty());
        let a = s.own(VarKey::new("asm.a", "width"));
        let b = s.own(VarKey::new("asm.b", "width"));
        let c = s.own(VarKey::new("asm.a", "depth"));
        assert!(a != b && a != c && b != c);
        assert_eq!(s.own(VarKey::new("asm.a", "width")), a);
        assert_eq!(s.owner_of(a), Some(&VarKey::new("asm.a", "width")));
        assert_eq!(s.len(), 3);
        assert!(!s.is_empty());
        // two instances vary independently: the difference keeps both spreads
        let d = form(10, 1, a).sub(&form(10, 1, b)).unwrap();
        let (lo, hi) = d.bounds().unwrap();
        assert_eq!((lo, hi), (mag(-2), mag(2)));
    }

    // frob:tests SourceVars.alias
    #[test]
    fn pure_reference_shares_referent_variable() {
        let mut s = SourceVars::new();
        let g = s.own(VarKey::new("housing", "groove_width"));
        let r = s
            .alias(
                VarKey::new("ring", "groove_width"),
                &VarKey::new("housing", "groove_width"),
            )
            .unwrap();
        assert_eq!(g, r);
        assert_eq!(s.len(), 1);
        // varying together: housing - ring cancels exactly
        let d = form(5, 1, g).sub(&form(5, 1, r)).unwrap();
        assert!(d.is_exact());
        assert!(matches!(
            s.alias(VarKey::new("x", "y"), &VarKey::new("nope", "z")),
            Err(AliasError::NoReferentVar(_))
        ));
        let other = s.own(VarKey::new("ring", "od"));
        assert!(matches!(
            s.alias(
                VarKey::new("ring", "od"),
                &VarKey::new("housing", "groove_width")
            ),
            Err(AliasError::AlreadyBound(_))
        ));
        assert_eq!(s.get(&VarKey::new("ring", "od")), Some(other));
    }

    // frob:tests SourceVars.anonymous
    #[test]
    fn arithmetic_parameter_gets_its_own_variable() {
        let mut s = SourceVars::new();
        let upstream = s.own(VarKey::new("p", "a"));
        let anon = s.anonymous();
        assert_ne!(upstream, anon);
        assert_eq!(s.owner_of(anon), None);
        // only its written tolerance: no coefficient on the upstream variable
        let f = form(3, 1, anon);
        assert!(f.coefficient(upstream).is_none());
        assert!(f.coefficient(anon).is_some());
    }

    // frob:tests SourceVars.own
    #[test]
    fn let_stack_up_counts_repeated_terms_once() {
        let mut s = SourceVars::new();
        let v = s.own(VarKey::new("p", "a"));
        let w = s.own(VarKey::new("p", "b"));
        let (a, b) = (form(10, 1, v), form(4, 2, w));
        // (a + b) - a cancels a's tolerance entirely
        let stack = a.add(&b).unwrap().sub(&a).unwrap();
        assert!(stack.coefficient(v).is_none());
        let (lo, hi) = stack.bounds().unwrap();
        assert_eq!((lo, hi), (mag(2), mag(6)));
        // a + a counts the term once, doubled (not root-sum or independent)
        let (lo, hi) = a.add(&a).unwrap().bounds().unwrap();
        assert_eq!((lo, hi), (mag(18), mag(22)));
    }

    // frob:tests c004_fact
    #[test]
    fn c004_for_computed_parameter_over_toleranced_inputs() {
        let key = VarKey::new("p", "cb");
        let toleranced = form(9, 1, VarId(0));
        let computed = evaluated(None, false, false);
        assert!(c004_fact(&key, &computed, &toleranced).is_some());
        assert!(c004_fact(&key, &computed, &Affine::constant(dim(), mag(9))).is_none());
        assert!(c004_fact(&key, &evaluated(Some(VarId(1)), false, false), &toleranced).is_none());
        assert!(c004_fact(&key, &evaluated(None, false, true), &toleranced).is_none());
        assert!(c004_fact(&key, &evaluated(None, true, false), &toleranced).is_none());
    }

    // frob:tests SourceVars.own
    #[test]
    fn nonlinear_operation_is_conservative() {
        let mut s = SourceVars::new();
        let a = form(3, 1, s.own(VarKey::new("p", "a")));
        let b = form(4, 1, s.own(VarKey::new("p", "b")));
        assert!(!a.add(&b).unwrap().is_conservative());
        let p = a.mul(&b).unwrap();
        assert!(p.is_conservative());
        let (lo, hi) = p.bounds().unwrap();
        assert!(lo.to_f64() <= 8.0 && hi.to_f64() >= 20.0);
    }
}
