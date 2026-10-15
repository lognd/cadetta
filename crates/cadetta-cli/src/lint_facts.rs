//! Facts the remaining lint families need and the IR does not carry (~A8J3XJW).
//!
//! Interfaces (I001), item visibility and use (U002, I004), `require` paths
//! (never suppressed) and `#[allow]` attributes (lints.md 2.2) are all read
//! from the parsed modules or the lowered IR here, so `pipeline` can hand them
//! to the `cadetta-lint` rule entry points.
//!
//! Specification: `docs/spec/lints.md` (I, U, 2.2, 2.3).

// frob:ticket 01M4TA3CABXHEZFA7BXA8J3XJW

use cadetta_ir::entities::{Interface, InterfaceBound, InterfaceMember};
use cadetta_ir::model::{Bound, BoundOp, Component, Exact, Project, SpanRef};
use cadetta_ir::variant_name::ComponentPath;
use cadetta_lint::engine::Allow;
use cadetta_lint::rules::u002::{ItemFacts, ItemKind as FactKind, Visibility};
use cadetta_syntax::ast::{
    Attr, AttrArg, BoundKind, Expr, ExprKind, File, InterfaceMemberKind, Item, ItemKind,
    MemberKind, UnOp, Vis,
};
use cadetta_units::{Magnitude, Rational, UnitTable};

/// One parsed module: its path inside the package, its AST and its source text.
pub struct ModuleView<'a> {
    /// Module segments below the package root.
    pub path: &'a [String],
    /// The parsed file.
    pub file: &'a File,
    /// The source text the file was parsed from.
    pub text: &'a str,
    /// The display name `SpanRef`s of this module carry.
    pub display: &'a str,
}

/// Lint visibility of an AST visibility modifier.
fn visibility(v: Vis) -> Visibility {
    match v {
        Vis::Private => Visibility::Private,
        Vis::PubPackage => Visibility::PubPackage,
        Vis::Pub => Visibility::Pub,
    }
}

/// Lower the `interface` items of `modules` (package `package`) to IR interfaces.
///
/// `extends` is flattened by last path segment within the same list, so an
/// implementer owes the members of every interface it inherits. Bounds with a
/// plain quantity limit are lowered to SI for C001/C002 (~J4802RH); a limit that
/// is not a literal is skipped with a debug log.
pub fn interfaces(package: &str, modules: &[ModuleView<'_>]) -> Vec<Interface> {
    let mut found: Vec<(Interface, Vec<String>)> = Vec::new();
    for m in modules {
        for item in m.file.items() {
            let ItemKind::Interface(i) = &item.kind else {
                continue;
            };
            let mut out = Interface {
                path: ComponentPath {
                    package: package.to_owned(),
                    modules: m.path.to_vec(),
                    item: i.name.name.clone(),
                },
                members: Vec::new(),
                bounds: Vec::new(),
                datums: Vec::new(),
            };
            for mem in &i.members {
                match &mem.kind {
                    InterfaceMemberKind::Field { name, optional, ty } => {
                        out.members.push(InterfaceMember {
                            name: name.name.clone(),
                            type_name: m
                                .text
                                .get(ty.span.start..ty.span.end)
                                .unwrap_or("")
                                .to_owned(),
                            optional: *optional,
                        });
                    }
                    InterfaceMemberKind::Datum(d) => out.datums.push(d.name.name.clone()),
                    InterfaceMemberKind::Bound(b) => out.bounds.extend(lower_bound(&b.kind)),
                    InterfaceMemberKind::Error => {}
                }
            }
            let parents = i
                .extends
                .iter()
                .filter_map(|p| p.segments.last().map(|s| s.name.clone()))
                .collect();
            tracing::debug!(interface = %i.name.name, members = out.members.len(), "lint_facts: interface lowered");
            found.push((out, parents));
        }
    }
    // Flatten inheritance; the passes bound the work on a cyclic `extends`.
    for _ in 0..found.len() {
        let snapshot = found.clone();
        for (iface, parents) in &mut found {
            for parent in parents.iter() {
                let Some((p, _)) = snapshot.iter().find(|(o, _)| &o.path.item == parent) else {
                    continue;
                };
                for mem in &p.members {
                    if !iface.members.iter().any(|x| x.name == mem.name) {
                        iface.members.push(mem.clone());
                    }
                }
                for d in &p.datums {
                    if !iface.datums.contains(d) {
                        iface.datums.push(d.clone());
                    }
                }
                for b in &p.bounds {
                    if !iface.bounds.contains(b) {
                        iface.bounds.push(b.clone());
                    }
                }
            }
        }
    }
    found.into_iter().map(|(i, _)| i).collect()
}

/// The exact SI magnitude of a single-segment quantity literal (`400N`, `-5mm`); `None` otherwise.
// frob:ticket 01M4V13T1DW08P87XP4J4802RH
fn literal_si(e: &Expr) -> Option<Exact> {
    match &e.kind {
        ExprKind::Paren(inner) => literal_si(inner),
        ExprKind::Unary(UnOp::Neg, inner) => {
            let m = literal_si(inner)?.0;
            Some(Exact(m.mul(&Magnitude::new(Rational::from_int(-1), 0))))
        }
        ExprKind::Quantity(segs) if segs.len() == 1 => {
            let n = Rational::parse_decimal(&segs[0].number).ok()?;
            let unit = UnitTable::builtin().parse(&segs[0].unit).ok()?;
            Some(Exact(Magnitude::new(n, 0).mul(&unit.factor)))
        }
        _ => {
            tracing::debug!("lint_facts: interface bound limit is not a plain quantity literal");
            None
        }
    }
}

/// Lower one interface bound to its IR form, when its limits are plain quantity literals.
// frob:ticket 01M4V13T1DW08P87XP4J4802RH
fn lower_bound(kind: &BoundKind) -> Option<InterfaceBound> {
    let (name, op, lo, hi) = match kind {
        BoundKind::Le { name, value } => (name, BoundOp::Le, None, Some(literal_si(value)?)),
        BoundKind::Ge { name, value } => (name, BoundOp::Ge, Some(literal_si(value)?), None),
        BoundKind::In { name, lo, hi } => {
            (name, BoundOp::In, Some(literal_si(lo)?), Some(literal_si(hi)?))
        }
        BoundKind::Eq { .. } => return None,
    };
    tracing::trace!(member = %name.name, ?op, "lint_facts: interface bound lowered");
    Some(InterfaceBound {
        name: name.name.clone(),
        bound: Bound { op, lo, hi },
    })
}

/// True when `name` occurs in `text` as a whole identifier, counted `n` times.
fn mentions(text: &str, name: &str) -> usize {
    let is_id = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(name)
        .filter(|(i, _)| {
            let before = text[..*i].chars().next_back();
            let after = text[i + name.len()..].chars().next();
            !before.is_some_and(is_id) && !after.is_some_and(is_id)
        })
        .count()
}

/// The declared name and fact kind of an item U002 tracks.
fn tracked(item: &Item) -> Option<(&str, FactKind)> {
    match &item.kind {
        ItemKind::Component(c) => Some((&c.name.name, FactKind::Component)),
        ItemKind::Fn(f) => Some((&f.name.name, FactKind::Function)),
        ItemKind::Interface(i) => Some((&i.name.name, FactKind::Interface)),
        ItemKind::Table(t) => Some((&t.name.name, FactKind::Table)),
        _ => None,
    }
}

/// U002 fact rows for the items of the checked package `package`.
///
/// An item is `used` when its name occurs as a whole identifier anywhere in the
/// package beyond its own declaration; this is lexical and so errs towards
/// "used". `instances` counts `part`s of the IR that instantiate a component.
pub fn item_facts(package: &str, modules: &[ModuleView<'_>], ir: &Project) -> Vec<ItemFacts> {
    let mut out = Vec::new();
    for m in modules {
        for item in m.file.items() {
            let Some((name, kind)) = tracked(item) else {
                continue;
            };
            let total: usize = modules.iter().map(|o| mentions(o.text, name)).sum();
            let mut path = package.to_owned();
            for seg in m.path {
                path.push_str("::");
                path.push_str(seg);
            }
            path.push_str("::");
            path.push_str(name);
            let instances = ir
                .components
                .iter()
                .flat_map(|c| &c.parts)
                .filter(|p| p.primary.ends_with(&format!("::{name}")) || p.type_name == name)
                .count();
            tracing::trace!(%path, mentions = total, instances, "lint_facts: item fact");
            out.push(ItemFacts {
                span: Some(SpanRef {
                    file: m.display.to_owned(),
                    start: item.span.start as u32,
                    end: item.span.end as u32,
                }),
                path,
                kind,
                visibility: visibility(item.vis),
                in_checked_package: true,
                used: total > 1,
                parameterized: false,
                is_root: false,
                instances,
            });
        }
    }
    out
}

/// True when the component at `c` was declared private in `modules`.
pub fn is_private(c: &Component, modules: &[ModuleView<'_>]) -> bool {
    modules
        .iter()
        .filter(|m| m.path == c.path.modules.as_slice())
        .flat_map(|m| m.file.items())
        .any(|i| {
            matches!(&i.kind, ItemKind::Component(k) if k.name.name == c.path.item)
                && i.vis == Vis::Private
        })
}

/// `#[allow(CODE, "because")]` attributes of the lowered members (lints.md 2.2).
///
/// An allow without a reason is dropped here; the engine rejects it too.
pub fn allows(ir: &Project) -> Vec<Allow> {
    let mut out = Vec::new();
    for c in &ir.components {
        for m in &c.members {
            for (k, v) in &m.attrs {
                let Some(code) = k.strip_prefix("allow:") else {
                    continue;
                };
                let because = v.trim().trim_matches('"');
                match Allow::new(code, &m.path.to_string(), because) {
                    Ok(a) => out.push(a),
                    Err(error) => {
                        tracing::warn!(%code, path = %m.path, %error, "lint_facts: allow skipped");
                    }
                }
            }
        }
    }
    out
}

/// `#[allow(CODE, "because")]` on `check` and `require` members, read from the AST.
///
/// The lowered IR keeps no attributes for these statements, so `allows` cannot see them. A
/// `require` allow is collected too: the engine never suppresses it and W003 names it
/// (obligations.md 4; ~SN4G7QS).
// frob:ticket 01M4V1HYQACFV6PNTBESN4G7QS
pub fn statement_allows(package: &str, modules: &[ModuleView<'_>]) -> Vec<Allow> {
    let mut out = Vec::new();
    for m in modules {
        for item in m.file.items() {
            let ItemKind::Component(c) = &item.kind else {
                continue;
            };
            for member in &c.members {
                let name = match &member.kind {
                    MemberKind::Check(k) => &k.name.name,
                    MemberKind::Require(r) => &r.name.name,
                    _ => continue,
                };
                let mut path = package.to_owned();
                for seg in m.path {
                    path.push_str("::");
                    path.push_str(seg);
                }
                path.push_str(&format!("::{}.{name}", c.name.name));
                for a in member.attrs.iter().chain(&member.trailing) {
                    match statement_allow(m.text, a, &path) {
                        Some(Ok(allow)) => out.push(allow),
                        Some(Err(error)) => {
                            tracing::warn!(%path, %error, "lint_facts: statement allow skipped");
                        }
                        None => {}
                    }
                }
            }
        }
    }
    out
}

/// One `allow(CODE, "because")` attribute on the statement at `path`; `None` for other attributes.
fn statement_allow(
    text: &str,
    a: &Attr,
    path: &str,
) -> Option<Result<Allow, cadetta_lint::engine::EngineError>> {
    if a.name.name != "allow" {
        return None;
    }
    let arg_text = |g: &AttrArg| match g {
        AttrArg::Expr(e) => match &e.kind {
            ExprKind::Str(s) => s.clone(),
            _ => text.get(e.span.start..e.span.end).unwrap_or("").to_owned(),
        },
        AttrArg::Named { .. } => String::new(),
    };
    let code = a.args.first().map(arg_text)?;
    let because = a.args.get(1).map(arg_text).unwrap_or_default();
    tracing::trace!(%code, %path, "lint_facts: statement allow");
    Some(Allow::new(&code, path, &because))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lowered(src: &str) -> Vec<Interface> {
        let parsed = cadetta_syntax::parser::parse(src);
        let m = ModuleView {
            path: &[],
            file: &parsed.file,
            text: src,
            display: "t.cdt",
        };
        interfaces("fx", &[m])
    }

    // frob:tests crates/cadetta-cli/src/lint_facts.rs::interfaces
    #[test]
    fn interface_bounds_lower_to_si_and_inherit() {
        let ifs = lowered(
            "interface Base {\n  load >= 400N\n  dia in 2mm .. 5mm\n  gap == 3mm\n}\n\
             interface Child: Base {\n  mass <= 2kg\n}\n",
        );
        let base = ifs.iter().find(|i| i.path.to_string().ends_with("Base")).expect("base");
        assert_eq!(base.bounds.len(), 2, "Eq is skipped, Ge and In kept");
        assert_eq!(base.bounds[0].bound.op, BoundOp::Ge);
        assert!(base.bounds[0].bound.hi.is_none());
        assert_eq!(base.bounds[1].bound.op, BoundOp::In);
        let child = ifs.iter().find(|i| i.path.to_string().ends_with("Child")).expect("child");
        assert_eq!(child.bounds.len(), 3, "own Le plus the two inherited bounds");
    }
}
