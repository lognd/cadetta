// frob:ticket 01M4ST8E8AQ98D0DFQ0DT50R91
//! Interface declarations and implementation checking (language.md 3.2).
//!
//! An interface is resolved from its syntax into members with kinds and
//! bounds. An implementer is then checked against it: a missing member is
//! I001 (fixed, never waivable, lints.md I), a violated bound is C001
//! against that implementer, an undecided one is C002. Readers of
//! `wheel.hub_bore` depend on the interface member only, never on the
//! implementer behind the slot.

use std::collections::BTreeMap;

use cadetta_syntax::ast::{BoundKind, Expr, Interface, InterfaceMemberKind, Span};
use cadetta_units::{CmpOp, Operand, Verdict, compare};
use tracing::{debug, trace, warn};

/// The diagnostic code for a missing interface member (fixed severity).
pub const I001: &str = "I001";
/// The diagnostic code for a violated bound.
pub const C001: &str = "C001";
/// The diagnostic code for an undecided bound.
pub const C002: &str = "C002";

/// A member an interface requires: a name, a kind and whether it may be absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberReq {
    /// The member name.
    pub name: String,
    /// The kind (type) as written, for example `Length`.
    pub kind: String,
    /// True for `name?: Kind`; absence is listed by roll-ups, not I001.
    pub optional: bool,
    /// Where the member is declared.
    pub span: Span,
}

/// The evaluated limit of an interface bound.
#[derive(Clone, Debug)]
pub enum Limit {
    /// `name <= limit`
    Le(Operand),
    /// `name >= limit`
    Ge(Operand),
    /// `name in lo..hi`
    In(Box<(Operand, Operand)>),
    /// `a == b`, two members of the implementer.
    Eq(String, String),
}

/// A bound every implementer must meet, with its limit evaluated.
#[derive(Clone, Debug)]
pub struct BoundReq {
    /// The bounded member (the left member for `==`).
    pub member: String,
    /// The evaluated limit.
    pub limit: Limit,
    /// Where the bound is declared.
    pub span: Span,
}

/// An interface after resolution: members with kinds, bounds and datums.
#[derive(Clone, Debug)]
pub struct ResolvedInterface {
    /// The interface name.
    pub name: String,
    /// Members, parents' first, in declaration order.
    pub members: Vec<MemberReq>,
    /// Bounds, parents' first.
    pub bounds: Vec<BoundReq>,
    /// Datum names the interface declares (geometry.md 2).
    pub datums: Vec<String>,
}

/// Resolves an interface declaration.
///
/// `parents` are the already-resolved extended interfaces; `eval` turns a
/// limit expression into an operand, or `None` after reporting its own
/// error (the bound is then skipped, never guessed).
pub fn resolve<F>(decl: &Interface, parents: &[ResolvedInterface], mut eval: F) -> ResolvedInterface
where
    F: FnMut(&Expr) -> Option<Operand>,
{
    let mut out = ResolvedInterface {
        name: decl.name.name.clone(),
        members: Vec::new(),
        bounds: Vec::new(),
        datums: Vec::new(),
    };
    for p in parents {
        for m in &p.members {
            if !out.members.iter().any(|x| x.name == m.name) {
                out.members.push(m.clone());
            }
        }
        out.bounds.extend(p.bounds.iter().cloned());
        out.datums.extend(p.datums.iter().cloned());
    }
    for m in &decl.members {
        match &m.kind {
            InterfaceMemberKind::Field { name, optional, ty } => {
                let kind = match &ty.kind {
                    cadetta_syntax::ast::TypeKind::Path(p) => p.text(),
                    _ => format!("{:?}", ty.kind),
                };
                trace!(interface = %out.name, member = %name.name, %kind, optional, "interface member");
                out.members.push(MemberReq {
                    name: name.name.clone(),
                    kind,
                    optional: *optional,
                    span: m.span,
                });
            }
            InterfaceMemberKind::Datum(d) => out.datums.push(d.name.name.clone()),
            InterfaceMemberKind::Bound(b) => {
                let (member, limit) = match &b.kind {
                    BoundKind::Le { name, value } => {
                        (name.name.clone(), eval(value).map(Limit::Le))
                    }
                    BoundKind::Ge { name, value } => {
                        (name.name.clone(), eval(value).map(Limit::Ge))
                    }
                    BoundKind::In { name, lo, hi } => {
                        let l = eval(lo)
                            .zip(eval(hi))
                            .map(|(l, h)| Limit::In(Box::new((l, h))));
                        (name.name.clone(), l)
                    }
                    BoundKind::Eq { lhs, rhs } => {
                        (lhs.text(), Some(Limit::Eq(lhs.text(), rhs.text())))
                    }
                };
                match limit {
                    Some(limit) => out.bounds.push(BoundReq {
                        member,
                        limit,
                        span: b.span,
                    }),
                    None => {
                        warn!(interface = %out.name, %member, "bound limit not evaluable; skipped")
                    }
                }
            }
            InterfaceMemberKind::Error => {}
        }
    }
    debug!(
        interface = %out.name,
        members = out.members.len(),
        bounds = out.bounds.len(),
        "interface resolved"
    );
    out
}

impl ResolvedInterface {
    /// The member requirement named `name`, if the interface declares it.
    pub fn member(&self, name: &str) -> Option<&MemberReq> {
        self.members.iter().find(|m| m.name == name)
    }

    /// The dependency a read of member `name` creates: the interface member, never an implementer.
    pub fn member_dependency(&self, name: &str) -> Option<String> {
        self.member(name)
            .map(|m| format!("{}.{}", self.name, m.name))
    }
}

/// What an implementer provides for one member.
#[derive(Clone, Debug)]
pub struct ImplMember {
    /// The member's kind as written.
    pub kind: String,
    /// Its evaluated value, when it has one.
    pub value: Option<Operand>,
}

/// A component's view for interface checking: its name and members.
#[derive(Clone, Debug, Default)]
pub struct Implementer {
    /// The implementing component's name.
    pub name: String,
    /// Its members by name.
    pub members: BTreeMap<String, ImplMember>,
}

/// A diagnostic raised against an implementer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceDiag {
    /// The rule code (`I001`, `C001`, `C002`).
    pub code: &'static str,
    /// The implementer the diagnostic is against.
    pub implementer: String,
    /// The member concerned.
    pub member: String,
    /// One-sentence message.
    pub message: String,
    /// The interface declaration the requirement came from.
    pub span: Span,
}

impl InterfaceDiag {
    /// Whether a `waive` may suppress this diagnostic (I001 is fixed, lints.md I).
    pub fn waivable(&self) -> bool {
        self.code != I001
    }
}

/// Checks an implementer against a resolved interface.
pub fn check_implementer(iface: &ResolvedInterface, imp: &Implementer) -> Vec<InterfaceDiag> {
    let mut out = Vec::new();
    for req in &iface.members {
        match imp.members.get(&req.name) {
            None if req.optional => {
                debug!(implementer = %imp.name, member = %req.name, "optional member absent");
            }
            None => out.push(InterfaceDiag {
                code: I001,
                implementer: imp.name.clone(),
                member: req.name.clone(),
                message: format!(
                    "{} lacks member `{}: {}` required by interface {}",
                    imp.name, req.name, req.kind, iface.name
                ),
                span: req.span,
            }),
            Some(m) if m.kind != req.kind => out.push(InterfaceDiag {
                code: I001,
                implementer: imp.name.clone(),
                member: req.name.clone(),
                message: format!(
                    "{}.{} has kind {} but interface {} requires {}",
                    imp.name, req.name, m.kind, iface.name, req.kind
                ),
                span: req.span,
            }),
            Some(_) => {}
        }
    }
    for b in &iface.bounds {
        check_bound(iface, imp, b, &mut out);
    }
    debug!(implementer = %imp.name, interface = %iface.name, diags = out.len(), "implementer checked");
    out
}

fn value_of<'a>(imp: &'a Implementer, name: &str) -> Option<&'a Operand> {
    imp.members.get(name).and_then(|m| m.value.as_ref())
}

fn check_bound(
    iface: &ResolvedInterface,
    imp: &Implementer,
    b: &BoundReq,
    out: &mut Vec<InterfaceDiag>,
) {
    let Some(v) = value_of(imp, &b.member) else {
        trace!(implementer = %imp.name, member = %b.member, "no value; bound skipped");
        return;
    };
    let tests: Vec<(CmpOp, Operand)> = match &b.limit {
        Limit::Le(l) => vec![(CmpOp::Le, l.clone())],
        Limit::Ge(l) => vec![(CmpOp::Ge, l.clone())],
        Limit::In(r) => vec![(CmpOp::Ge, r.0.clone()), (CmpOp::Le, r.1.clone())],
        Limit::Eq(_, rhs) => match value_of(imp, rhs) {
            Some(r) => vec![(CmpOp::Eq, r.clone())],
            None => return,
        },
    };
    let mut verdict = Verdict::Holds;
    for (op, limit) in &tests {
        match compare(*op, v, limit) {
            Ok(c) => match (c.verdict, verdict) {
                (Verdict::Violates, _) => {
                    verdict = Verdict::Violates;
                    break;
                }
                (Verdict::Undecided, _) => verdict = Verdict::Undecided,
                _ => {}
            },
            Err(e) => {
                // A dimension mismatch is a kind error, reported by kind checking.
                warn!(implementer = %imp.name, member = %b.member, error = %e, "bound not comparable");
                return;
            }
        }
    }
    let (code, word) = match verdict {
        Verdict::Holds => return,
        Verdict::Violates => (C001, "violates"),
        Verdict::Undecided => (C002, "is undecided against"),
    };
    out.push(InterfaceDiag {
        code,
        implementer: imp.name.clone(),
        member: b.member.clone(),
        message: format!(
            "{}.{} {} a bound of interface {}",
            imp.name, b.member, word, iface.name
        ),
        span: b.span,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::ItemKind;
    use cadetta_syntax::parser::parse;
    use cadetta_units::{Affine, Dim, Dimension, Magnitude, Rational};

    fn force() -> Dim {
        Dim::new(Dimension::from_si([1, 1, -2, 0, 0, 0, 0]), 0)
    }

    fn newtons(n: i64) -> Operand {
        Operand::Form(Affine::constant(
            force(),
            Magnitude::new(Rational::from_int(n), 0),
        ))
    }

    fn caster() -> ResolvedInterface {
        let parsed = parse("interface Caster {\n  diameter: Length\n  load_rating >= 400N\n}\n");
        let item = parsed.file.items().next().unwrap();
        let ItemKind::Interface(i) = &item.kind else {
            panic!("not an interface")
        };
        resolve(i, &[], |_| Some(newtons(400)))
    }

    fn imp(name: &str, rating: Option<i64>, with_diameter: bool) -> Implementer {
        let mut members = BTreeMap::new();
        if with_diameter {
            members.insert(
                "diameter".into(),
                ImplMember {
                    kind: "Length".into(),
                    value: None,
                },
            );
        }
        if let Some(r) = rating {
            members.insert(
                "load_rating".into(),
                ImplMember {
                    kind: "Force".into(),
                    value: Some(newtons(r)),
                },
            );
        }
        Implementer {
            name: name.into(),
            members,
        }
    }

    // frob:tests crates/cadetta-variants/src/interface.rs::resolve kind=unit
    #[test]
    fn resolved_members_carry_kinds_and_bounds() {
        let c = caster();
        assert_eq!(c.member("diameter").unwrap().kind, "Length");
        assert!(!c.member("diameter").unwrap().optional);
        assert_eq!(c.bounds.len(), 1);
        assert_eq!(c.bounds[0].member, "load_rating");
        assert!(matches!(c.bounds[0].limit, Limit::Ge(_)));
    }

    // frob:tests crates/cadetta-variants/src/interface.rs::check_implementer kind=unit
    #[test]
    fn missing_member_is_i001_and_not_waivable() {
        let d = check_implementer(&caster(), &imp("Bad", Some(500), false));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, I001);
        assert_eq!(d[0].member, "diameter");
        assert!(!d[0].waivable());
    }

    // frob:tests crates/cadetta-variants/src/interface.rs::check_implementer kind=unit
    #[test]
    fn violated_bound_is_c001_against_implementer() {
        let d = check_implementer(&caster(), &imp("Weak", Some(300), true));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, C001);
        assert_eq!(d[0].implementer, "Weak");
        assert!(d[0].waivable());
        assert!(check_implementer(&caster(), &imp("Strong", Some(500), true)).is_empty());
    }

    // frob:tests crates/cadetta-variants/src/interface.rs::ResolvedInterface::member_dependency kind=unit
    #[test]
    fn reads_depend_on_the_interface_only() {
        let c = caster();
        assert_eq!(
            c.member_dependency("diameter").as_deref(),
            Some("Caster.diameter")
        );
        assert_eq!(c.member_dependency("nope"), None);
    }
}
