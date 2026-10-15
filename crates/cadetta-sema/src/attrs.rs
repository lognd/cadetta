//! Attribute resolution: scope defaults and member attributes (language.md 3.11).
//!
//! `#![...]` at file or component scope is a default for everything inside;
//! a member's leading and trailing `#[...]` win over it. Rule roles are never
//! inherited from a scope (language.md 3.8, 3.11).

use std::collections::BTreeMap;

use cadetta_syntax::ast::{Attr, AttrArg, Expr, ExprKind};
use cadetta_units::{Role, RoleInfo, RoleSource};
use tracing::{debug, trace};

/// Attribute names that never inherit from a scope default (the tolerance role).
pub const NON_INHERITED: &[&str] = &["role"];

/// Renders an argument expression as text (string literals unquoted).
fn expr_text(src: &str, e: &Expr) -> String {
    match &e.kind {
        ExprKind::Str(s) => s.clone(),
        _ => src.get(e.span.start..e.span.end).unwrap_or("").to_owned(),
    }
}

/// Renders one attribute argument as text.
fn arg_text(src: &str, arg: &AttrArg) -> String {
    match arg {
        AttrArg::Expr(e) => expr_text(src, e),
        AttrArg::Named { name, value } => format!("{}={}", name.name, expr_text(src, value)),
    }
}

/// Expands one attribute into IR `(key, value)` entries.
///
/// `default(Iface = Impl)` and `allow(CODE, "because")` may repeat, so they key
/// by their first argument (`default:Iface`, `allow:CODE`); everything else keys
/// by name with the arguments joined by `, `.
fn entries(src: &str, a: &Attr) -> Vec<(String, String)> {
    let name = a.name.name.as_str();
    match name {
        "default" => a
            .args
            .iter()
            .filter_map(|g| match g {
                AttrArg::Named { name: n, value } => {
                    Some((format!("default:{}", n.name), expr_text(src, value)))
                }
                AttrArg::Expr(_) => None,
            })
            .collect(),
        "allow" => {
            let texts: Vec<String> = a.args.iter().map(|g| arg_text(src, g)).collect();
            match texts.split_first() {
                Some((code, rest)) => vec![(format!("allow:{code}"), rest.join(", "))],
                None => Vec::new(),
            }
        }
        _ => {
            let v: Vec<String> = a.args.iter().map(|g| arg_text(src, g)).collect();
            vec![(name.to_owned(), v.join(", "))]
        }
    }
}

/// Folds `attrs` into `out`; later attributes replace earlier ones of the same key.
fn apply(src: &str, attrs: &[Attr], out: &mut BTreeMap<String, String>, inherit: bool) {
    for a in attrs {
        if inherit && (NON_INHERITED.contains(&a.name.name.as_str()) || is_role_hint(a)) {
            trace!(attr = %a.name.name, "scope default skipped: not inheritable");
            continue;
        }
        for (k, v) in entries(src, a) {
            trace!(key = %k, value = %v, "attribute applied");
            out.insert(k, v);
        }
    }
}

/// True for a bare, argument-less attribute: a rule role (`#![hole]` is never a default).
fn is_role_hint(a: &Attr) -> bool {
    a.args.is_empty() && !matches!(a.name.name.as_str(), "priv" | "pub")
}

/// The explicit tolerance role of a member: `#[role(hole|shaft)]` or a bare `#[hole]`/`#[shaft]`.
// frob:ticket 01M4V1HDVGKT6AVWCWY7078TNV
pub fn explicit_role(leading: &[Attr], trailing: &[Attr]) -> Option<RoleInfo> {
    let role = leading
        .iter()
        .chain(trailing)
        .filter(|a| !a.inner)
        .filter_map(|a| match (a.name.name.as_str(), a.args.first()) {
            ("hole", None) => Some(Role::Hole),
            ("shaft", None) => Some(Role::Shaft),
            ("role", Some(AttrArg::Expr(Expr { kind: ExprKind::Path(p), .. }))) => p
                .segments
                .first()
                .and_then(|s| Role::from_attribute(&s.name).ok()),
            _ => None,
        })
        .next_back()?;
    trace!(?role, "attrs: explicit member role");
    Some(RoleInfo {
        role,
        source: RoleSource::Attribute,
    })
}

/// Resolves a member's attributes over its scopes, member attributes winning.
///
/// Precedence, lowest first: file `#![]`, component `#![]`, the member's
/// leading `#[]`, the member's trailing `#[]`. `src` is the file's text, used
/// to recover argument spellings.
pub fn resolve_attrs(
    src: &str,
    file_inner: &[Attr],
    component_inner: &[Attr],
    leading: &[Attr],
    trailing: &[Attr],
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    apply(src, file_inner, &mut out, true);
    apply(src, component_inner, &mut out, true);
    apply(src, leading, &mut out, false);
    apply(src, trailing, &mut out, false);
    debug!(count = out.len(), "resolved member attributes");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::{FileEntry, ItemKind, MemberKind};
    use cadetta_syntax::parser::parse;

    const SRC: &str = "#![display(mm)]\nmodel M {\n  #![emit(mid)]\n  #[emit(max)]\n  a = 1mm #[display(cm)]\n  b = 2mm\n  #[hole]\n  c = 3mm\n}\n";

    fn resolved() -> Vec<BTreeMap<String, String>> {
        let p = parse(SRC);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        let mut out = Vec::new();
        for c in p.file.components() {
            for m in &c.members {
                if matches!(m.kind, MemberKind::Param(_)) {
                    out.push(resolve_attrs(
                        SRC,
                        &p.file.inner_attrs,
                        &c.inner_attrs,
                        &m.attrs,
                        &m.trailing,
                    ));
                }
            }
        }
        out
    }

    // frob:tests crates/cadetta-sema/src/attrs.rs::resolve_attrs kind=unit
    #[test]
    fn scope_defaults_apply_and_member_wins() {
        let r = resolved();
        assert_eq!(r.len(), 3);
        // a: leading emit(max) beats component emit(mid); trailing display(cm) beats file display(mm).
        assert_eq!(r[0]["emit"], "max");
        assert_eq!(r[0]["display"], "cm");
        // b: pure defaults.
        assert_eq!(r[1]["emit"], "mid");
        assert_eq!(r[1]["display"], "mm");
        // c: rule role kept on the member only.
        assert!(r[2].contains_key("hole"));
    }

    #[test]
    fn roles_are_not_inherited() {
        let p = parse("#![hole]\nmodel M { a = 1mm }\n");
        let m = resolve_attrs("#![hole]\n", &p.file.inner_attrs, &[], &[], &[]);
        assert!(m.is_empty());
    }

    #[test]
    fn default_and_allow_key_by_argument() {
        let src = "#![default(Fastener = M3)]\nmodel M {\n #[allow(Q010, \"why\")]\n a = 1mm\n}\n";
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        let FileEntry::Item(it) = &p.file.entries[0] else {
            panic!("item")
        };
        let ItemKind::Component(c) = &it.kind else {
            panic!("component")
        };
        let m = &c.members[0];
        let r = resolve_attrs(src, &p.file.inner_attrs, &[], &m.attrs, &m.trailing);
        assert_eq!(r["default:Fastener"], "M3");
        assert_eq!(r["allow:Q010"], "why");
    }
}
