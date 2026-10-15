//! Visibility checking: `pub`, `pub(package)`, `priv`, E016 and E017 (packages.md 3).
//!
//! [`VisibilityChecker`] sits beside the [`Resolver`]: resolution answers what
//! a name means, this module answers whether the use site may see it.
//!
//! Decisions written into packages.md 3: items are private to their module and
//! its descendants; `pub(package)` is visible anywhere in the declaring
//! package; `pub` anywhere reachable (reachability is the resolver's job).
//! Component members are visible wherever the component is, except a `priv`
//! member, which only the declaring package may read. A `pub(package)` item
//! named by a `pub` signature is also E017 (it is less visible). The
//! member chain is followed through `part` slots (using the slot's declared
//! type) and every step is checked for `priv`; the walk stops at the first
//! non-part member, whose value type resolution does not carry. `std` must export only `pub` items ([`VisibilityChecker::non_pub_items`]).

// frob:ticket 01M4SXRV8HCNXHVFX87BTFJC7S

use std::collections::{BTreeMap, BTreeSet};

use cadetta_syntax::ast::{
    Component, Ident, Interface, InterfaceMemberKind, Item, ItemKind, Member, MemberKind, Path,
    Type, TypeKind, Vis,
};
use cadetta_syntax::span::Span;
use tracing::{debug, trace, warn};

use crate::resolve::{
    CanonPath, Ctx, MemberSeg, ModuleKey, PackageSrc, ResolveError, Resolved, Resolver,
};

/// Maximum depth when following `extends` chains.
const MAX_EXTENDS_DEPTH: usize = 32;

struct ItemInfo<'a> {
    vis: Vis,
    item: &'a Item,
}

/// True when an item of visibility `vis` declared in `decl_pkg::decl_mod` is visible from `from`.
pub fn is_visible(vis: Vis, decl_pkg: &str, decl_mod: &[String], from: &Ctx) -> bool {
    match vis {
        Vis::Pub => true,
        Vis::PubPackage => from.package == decl_pkg,
        Vis::Private => from.package == decl_pkg && from.module.starts_with(decl_mod),
    }
}

fn rank(v: Vis) -> u8 {
    match v {
        Vis::Private => 0,
        Vis::PubPackage => 1,
        Vis::Pub => 2,
    }
}

pub(crate) fn member_name(m: &Member) -> Option<&str> {
    match &m.kind {
        MemberKind::Param(p) => Some(&p.name.name),
        MemberKind::Let(l) => Some(&l.name.name),
        MemberKind::Part(p) => Some(&p.name.name),
        MemberKind::Fit(f) => Some(&f.name.name),
        MemberKind::Check(c) => Some(&c.name.name),
        MemberKind::Datum(d) => Some(&d.name.name),
        _ => None,
    }
}

fn err(ctx: &Ctx, code: &'static str, span: Span, message: String) -> ResolveError {
    warn!(code, %message, "visibility error");
    ResolveError {
        module: Some(Box::new(ModuleKey::of_ctx(ctx))),
        code,
        message,
        span,
        related: Vec::new(),
    }
}

/// Checks use-site visibility (E016) and signature leaks (E017) over a project.
pub struct VisibilityChecker<'a> {
    resolver: &'a Resolver<'a>,
    items: BTreeMap<CanonPath, ItemInfo<'a>>,
}

impl<'a> VisibilityChecker<'a> {
    /// Indexes the visibility of every item of every package.
    pub fn new(resolver: &'a Resolver<'a>, packages: &[PackageSrc<'a>]) -> Self {
        let mut items = BTreeMap::new();
        for p in packages {
            for m in &p.modules {
                for it in m.file.items() {
                    if let Some(name) = item_name(it) {
                        let id = CanonPath::item(&p.name, &m.path, name);
                        items.entry(id).or_insert(ItemInfo {
                            vis: it.vis,
                            item: it,
                        });
                    }
                }
            }
        }
        debug!(items = items.len(), "indexed item visibility");
        Self { resolver, items }
    }

    /// The declared visibility of an item path, when it is a known item.
    pub fn vis_of(&self, item: &CanonPath) -> Option<Vis> {
        self.items.get(&item.item_only()).map(|i| i.vis)
    }

    /// Checks that a resolved reference made from `from` may see what it names (E016).
    pub fn check_use(
        &self,
        from: &Ctx,
        resolved: &Resolved,
        span: Span,
    ) -> Result<(), ResolveError> {
        let path = match resolved {
            Resolved::Member { path, .. } | Resolved::Part { path } => path,
            Resolved::Item { path, .. } => path,
            Resolved::Arg { path, .. } => path,
            Resolved::Variant { enum_path, .. } => enum_path,
            Resolved::Local(_) | Resolved::Prelude(_) => return Ok(()),
        };
        self.check_item(from, path, span)?;
        if matches!(resolved, Resolved::Member { .. } | Resolved::Part { .. }) {
            self.check_members(from, path, span)?;
        }
        trace!(%path, "visible");
        Ok(())
    }

    /// Checks only the `priv` members a resolved member or part read passes through (E016).
    ///
    /// The item itself is judged at its `use`; this is the use-site half for a bare imported name.
    // frob:ticket 01M4V2J3BCSB3MFKDRSPGR0M75
    pub fn check_member_use(
        &self,
        from: &Ctx,
        resolved: &Resolved,
        span: Span,
    ) -> Result<(), ResolveError> {
        match resolved {
            Resolved::Member { path, .. } | Resolved::Part { path } => {
                self.check_members(from, path, span)
            }
            _ => Ok(()),
        }
    }

    /// E016 over every `use` of the project whose target item is not visible to the importing module.
    // frob:ticket 01M4YR00X3TTZGSBAKQZDNTT6T
    pub fn check_uses(&self) -> Vec<ResolveError> {
        let mut errors: Vec<ResolveError> = self
            .resolver
            .use_item_targets()
            .into_iter()
            .filter_map(|(ctx, path, span)| self.check_item(&ctx, &path, span).err())
            .collect();
        errors.sort_by_key(|e| (e.module.clone(), e.span));
        debug!(count = errors.len(), "checked use visibility");
        errors
    }

    fn check_item(&self, from: &Ctx, path: &CanonPath, span: Span) -> Result<(), ResolveError> {
        let id = path.item_only();
        let Some(info) = self.items.get(&id) else {
            return Ok(());
        };
        if is_visible(info.vis, &id.package, &id.modules, from) {
            return Ok(());
        }
        Err(err(
            from,
            "E016",
            span,
            format!("`{id}` is private to {}", scope_of(info.vis, &id)),
        ))
    }

    /// Walks the `part` slots and first member of `path`, rejecting a `priv` one read cross-package.
    fn check_members(&self, from: &Ctx, path: &CanonPath, span: Span) -> Result<(), ResolveError> {
        let mut names: Vec<&str> = path.parts.iter().map(|p| p.name.as_str()).collect();
        names.extend(path.members.iter().filter_map(|m| match m {
            MemberSeg::Name(n) => Some(n.as_str()),
            MemberSeg::Index(_) => None,
        }));
        let mut cur = path.item_only();
        for (i, name) in names.iter().enumerate() {
            let Some(comp) = self.component(&cur) else {
                return Ok(());
            };
            let Some(m) = comp.members.iter().find(|m| member_name(m) == Some(name)) else {
                return Ok(());
            };
            if m.private && from.package != cur.package {
                return Err(err(
                    from,
                    "E016",
                    span,
                    format!("`{name}` of `{cur}` is `priv` to package `{}`", cur.package),
                ));
            }
            if i + 1 < names.len() {
                let MemberKind::Part(part) = &m.kind else {
                    return Ok(());
                };
                let ctx = decl_ctx(&cur);
                match self.type_item(&ctx, &part.ty) {
                    Some(next) => cur = next,
                    None => return Ok(()),
                }
            }
        }
        Ok(())
    }

    fn component(&self, id: &CanonPath) -> Option<&'a Component> {
        match &self.items.get(id)?.item.kind {
            ItemKind::Component(c) => Some(c),
            _ => None,
        }
    }

    fn interface(&self, id: &CanonPath) -> Option<&'a Interface> {
        match &self.items.get(id)?.item.kind {
            ItemKind::Interface(i) => Some(i),
            _ => None,
        }
    }

    /// The item a type path names, when it resolves to one.
    fn type_item(&self, ctx: &Ctx, ty: &Type) -> Option<CanonPath> {
        let mut out = Vec::new();
        type_paths(ty, &mut out);
        let first = out.first()?;
        match self.resolver.resolve_path(ctx, first, None) {
            Ok(Resolved::Item { path, .. }) => Some(path),
            _ => None,
        }
    }

    /// E017 over every item of the project: private types in public signatures,
    /// and `priv` members implementing interface members.
    pub fn check_signatures(&self) -> Vec<ResolveError> {
        let mut errors = Vec::new();
        for (id, info) in &self.items {
            let ctx = decl_ctx(id);
            match &info.item.kind {
                ItemKind::Interface(i) if info.vis != Vis::Private => {
                    for m in &i.members {
                        if let InterfaceMemberKind::Field { ty, .. } = &m.kind {
                            self.check_type(&ctx, info.vis, id, ty, &mut errors);
                        }
                    }
                }
                ItemKind::Component(c) => {
                    if info.vis != Vis::Private {
                        for p in &c.params {
                            self.check_type(&ctx, info.vis, id, &p.ty, &mut errors);
                        }
                    }
                    self.check_implements(&ctx, c, &mut errors);
                }
                ItemKind::Fn(f) if info.vis != Vis::Private => {
                    for p in &f.params {
                        self.check_type(&ctx, info.vis, id, &p.ty, &mut errors);
                    }
                    self.check_type(&ctx, info.vis, id, &f.ret, &mut errors);
                }
                _ => {}
            }
        }
        errors.sort_by_key(|e| (e.span, e.code));
        errors
    }

    fn check_type(
        &self,
        ctx: &Ctx,
        vis: Vis,
        owner: &CanonPath,
        ty: &Type,
        errors: &mut Vec<ResolveError>,
    ) {
        let mut paths = Vec::new();
        type_paths(ty, &mut paths);
        for p in paths {
            let Ok(Resolved::Item { path, .. }) = self.resolver.resolve_path(ctx, p, None) else {
                continue;
            };
            let Some(named) = self.vis_of(&path) else {
                continue;
            };
            if rank(named) < rank(vis) {
                errors.push(err(
                    ctx,
                    "E017",
                    p.span,
                    format!(
                        "`{owner}` is more visible than `{}` in its signature",
                        path.item_only()
                    ),
                ));
            }
        }
    }

    fn check_implements(&self, ctx: &Ctx, c: &Component, errors: &mut Vec<ResolveError>) {
        let mut required: BTreeSet<String> = BTreeSet::new();
        for p in &c.implements {
            if let Ok(Resolved::Item { path, .. }) = self.resolver.resolve_path(ctx, p, None) {
                self.interface_members(&path, &mut required, 0);
            }
        }
        for m in c.members.iter().filter(|m| m.private) {
            if let Some(n) = member_name(m)
                && required.contains(n)
            {
                errors.push(err(
                    ctx,
                    "E017",
                    m.span,
                    format!("`priv` member `{n}` implements an interface member"),
                ));
            }
        }
    }

    fn interface_members(&self, id: &CanonPath, out: &mut BTreeSet<String>, depth: usize) {
        let Some(i) = self.interface(id) else { return };
        if depth > MAX_EXTENDS_DEPTH {
            return;
        }
        for m in &i.members {
            match &m.kind {
                InterfaceMemberKind::Field { name, .. } => {
                    out.insert(name.name.clone());
                }
                InterfaceMemberKind::Datum(d) => {
                    out.insert(d.name.name.clone());
                }
                _ => {}
            }
        }
        let ctx = decl_ctx(id);
        for e in &i.extends {
            if let Ok(Resolved::Item { path, .. }) = self.resolver.resolve_path(&ctx, e, None) {
                self.interface_members(&path, out, depth + 1);
            }
        }
    }

    /// Items of `package` that are not `pub`; for `std` this must be empty (packages.md 3).
    pub fn non_pub_items(&self, package: &str) -> Vec<CanonPath> {
        self.items
            .iter()
            .filter(|(id, i)| id.package == package && i.vis != Vis::Pub)
            .map(|(id, _)| id.clone())
            .collect()
    }
}

fn scope_of(vis: Vis, id: &CanonPath) -> String {
    match vis {
        Vis::Private => format!("module `{}`", module_display(id)),
        _ => format!("package `{}`", id.package),
    }
}

fn module_display(id: &CanonPath) -> String {
    let mut s = id.package.clone();
    for m in &id.modules {
        s.push_str("::");
        s.push_str(m);
    }
    s
}

fn decl_ctx(id: &CanonPath) -> Ctx {
    Ctx {
        package: id.package.clone(),
        module: id.modules.clone(),
        component: None,
        locals: Vec::new(),
    }
}

pub(crate) fn item_name(it: &Item) -> Option<&str> {
    let n: &Ident = match &it.kind {
        ItemKind::Component(c) => &c.name,
        ItemKind::Interface(i) => &i.name,
        ItemKind::Cite(c) => &c.name,
        ItemKind::Enum(e) => &e.name,
        ItemKind::Fn(f) => &f.name,
        _ => return None,
    };
    Some(&n.name)
}

fn type_paths<'t>(ty: &'t Type, out: &mut Vec<&'t Path>) {
    match &ty.kind {
        TypeKind::Path(p) => out.push(p),
        TypeKind::OnDatum { base, .. } => out.push(base),
        TypeKind::Array { elem, .. } => type_paths(elem, out),
        TypeKind::Table(inner) => type_paths(inner, out),
        TypeKind::KindExpr(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::{EnumDecl, ModuleSrc};
    use cadetta_syntax::ast::{Expr, File};
    use cadetta_syntax::parser::parse;

    fn file(src: &str) -> File {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        p.file
    }

    fn expr_of(src: &str) -> Expr {
        let f = file(&format!("model T {{\n  v = {src}\n}}"));
        match &f.components().next().unwrap().members[0].kind {
            MemberKind::Param(p) => p.value.clone(),
            _ => unreachable!(),
        }
    }

    fn ctx(pkg: &str, module: &[&str]) -> Ctx {
        Ctx {
            package: pkg.to_owned(),
            module: module.iter().map(|s| (*s).to_owned()).collect(),
            component: None,
            locals: Vec::new(),
        }
    }

    fn pkg<'a>(name: &str, deps: &[&str], mods: Vec<(&[&str], &'a File)>) -> PackageSrc<'a> {
        let no_enums: &'a [EnumDecl] = &[];
        PackageSrc {
            defaults: Default::default(),
            name: name.to_owned(),
            deps: deps.iter().map(|s| (*s).to_owned()).collect(),
            modules: mods
                .into_iter()
                .map(|(p, file)| ModuleSrc {
                    path: p.iter().map(|s| (*s).to_owned()).collect(),
                    file,
                    enums: no_enums,
                })
                .collect(),
        }
    }

    /// Resolves `src` from `from` and runs the E016 check on the result.
    fn use_code(pkgs: Vec<PackageSrc<'_>>, from: &Ctx, src: &str) -> Result<(), &'static str> {
        let (r, _) = Resolver::new(pkgs.clone());
        let v = VisibilityChecker::new(&r, &pkgs);
        let e = expr_of(src);
        let res = r.resolve_ref(from, &e, None).expect("resolves");
        v.check_use(from, &res, e.span).map_err(|e| e.code)
    }

    // frob:tests crates/cadetta-sema/src/visibility.rs::private_item_scope_and_pub_widening kind=unit
    #[test]
    fn private_item_scope_and_pub_widening() {
        let inner = file(
            "model Hidden { a = 1 }\npub(package) model Pkg { a = 1 }\npub model Open { a = 1 }\n",
        );
        let other = file("model User { x = 1 }\n");
        let mk = || {
            vec![
                pkg(
                    "p",
                    &[],
                    vec![(&["a"], &inner), (&["b"], &other), (&["a", "c"], &other)],
                ),
                pkg("q", &["p"], vec![(&[], &other)]),
            ]
        };
        // Same module and a descendant see the private item.
        assert_eq!(use_code(mk(), &ctx("p", &["a"]), "p::a::Hidden.a"), Ok(()));
        assert_eq!(
            use_code(mk(), &ctx("p", &["a", "c"]), "p::a::Hidden.a"),
            Ok(())
        );
        // A sibling module does not.
        assert_eq!(
            use_code(mk(), &ctx("p", &["b"]), "p::a::Hidden.a"),
            Err("E016")
        );
        // pub(package): same package yes, other package no.
        assert_eq!(use_code(mk(), &ctx("p", &["b"]), "p::a::Pkg.a"), Ok(()));
        assert_eq!(use_code(mk(), &ctx("q", &[]), "p::a::Pkg.a"), Err("E016"));
        // pub: everywhere.
        assert_eq!(use_code(mk(), &ctx("q", &[]), "p::a::Open.a"), Ok(()));
    }

    // frob:tests crates/cadetta-sema/src/visibility.rs::unmodified_members_are_visible_everywhere kind=unit
    #[test]
    fn unmodified_members_are_visible_everywhere() {
        let lib = file(
            "pub model C { a = 1\n  let l = 2\n  part p: C2\n  check ok: a > 0\n}\npub model C2 { z = 1 }\n",
        );
        let user = file("model User { x = 1 }\n");
        for member in ["a", "l", "p", "ok"] {
            let pk = vec![
                pkg("p", &[], vec![(&[], &lib)]),
                pkg("q", &["p"], vec![(&[], &user)]),
            ];
            assert_eq!(
                use_code(pk, &ctx("q", &[]), &format!("p::C.{member}")),
                Ok(()),
                "{member}"
            );
        }
    }

    // frob:tests crates/cadetta-sema/src/visibility.rs::priv_member_cross_package_is_e016 kind=unit
    #[test]
    fn priv_member_cross_package_is_e016() {
        let lib = file(
            "pub model C { priv row = 3\n  pub_val = row\n  part w: Inner\n  priv part hid: Inner\n}\npub model Inner { priv deep = 1\n  open = 2 }\n",
        );
        let user = file("model User { x = 1 }\n");
        let mk = || {
            vec![
                pkg("p", &[], vec![(&[], &lib)]),
                pkg("q", &["p"], vec![(&[], &user)]),
            ]
        };
        assert_eq!(use_code(mk(), &ctx("q", &[]), "p::C.row"), Err("E016"));
        assert_eq!(use_code(mk(), &ctx("q", &[]), "p::C.pub_val"), Ok(()));
        // Inside the declaring package the priv member is readable.
        assert_eq!(use_code(mk(), &ctx("p", &["x"]), "p::C.row"), Ok(()));
        // Through a part slot: the target's priv member and a priv slot.
        assert_eq!(use_code(mk(), &ctx("q", &[]), "p::C.w.deep"), Err("E016"));
        assert_eq!(use_code(mk(), &ctx("q", &[]), "p::C.w.open"), Ok(()));
        assert_eq!(use_code(mk(), &ctx("q", &[]), "p::C.hid"), Err("E016"));
    }

    // frob:tests crates/cadetta-sema/src/visibility.rs::priv_member_is_still_an_ordinary_member kind=unit
    #[test]
    fn priv_member_is_still_an_ordinary_member() {
        // Inside its own package a priv member resolves, and the own-component
        // read (how the package's public members use it) is never rejected.
        let lib = file("pub model C { priv row = 3\n  total = row }\n");
        let pk = vec![pkg("p", &[], vec![(&[], &lib)])];
        let (r, errs) = Resolver::new(pk.clone());
        assert!(errs.is_empty());
        let v = VisibilityChecker::new(&r, &pk);
        let c = Ctx {
            component: Some("C".to_owned()),
            ..ctx("p", &[])
        };
        let e = expr_of("row");
        let res = r.resolve_ref(&c, &e, None).unwrap();
        assert!(matches!(res, Resolved::Member { .. }));
        assert!(v.check_use(&c, &res, e.span).is_ok());
        assert!(v.check_signatures().is_empty());
    }

    fn sig_codes(src: &str) -> Vec<&'static str> {
        let f = file(src);
        let pk = vec![pkg("p", &[], vec![(&[], &f)])];
        let (r, _) = Resolver::new(pk.clone());
        VisibilityChecker::new(&r, &pk)
            .check_signatures()
            .into_iter()
            .map(|e| e.code)
            .collect()
    }

    // frob:tests crates/cadetta-sema/src/visibility.rs::private_in_public_signatures_are_e017 kind=unit
    #[test]
    fn private_in_public_signatures_are_e017() {
        // Component parameter type.
        assert_eq!(
            sig_codes("model Hid { a = 1 }\npub model M(h: Hid) { a = 1 }\n"),
            vec!["E017"]
        );
        // Function argument and result.
        assert_eq!(
            sig_codes("model Hid { a = 1 }\npub fn f(h: Hid) -> Hid { h }\n"),
            vec!["E017", "E017"]
        );
        // Interface member type.
        assert_eq!(
            sig_codes("model Hid { a = 1 }\npub interface I { m: Hid }\n"),
            vec!["E017"]
        );
        // A pub(package) type in a pub signature is also less visible.
        assert_eq!(
            sig_codes("pub(package) model Hid { a = 1 }\npub model M(h: Hid) { a = 1 }\n"),
            vec!["E017"]
        );
        // Must-not-fire: pub types, private owners, kinds.
        assert!(
            sig_codes("pub model Ok { a = 1 }\npub model M(h: Ok, l: Length) { a = 1 }\n")
                .is_empty()
        );
        assert!(sig_codes("model Hid { a = 1 }\nmodel M(h: Hid) { a = 1 }\n").is_empty());
    }

    // frob:tests crates/cadetta-sema/src/visibility.rs::priv_implementing_member_is_e017 kind=unit
    #[test]
    fn priv_implementing_member_is_e017() {
        let base = "pub interface I { bore: Length }\npub interface J: I { od: Length }\n";
        assert_eq!(
            sig_codes(&format!("{base}pub model M : I {{ priv bore = 1mm }}\n")),
            vec!["E017"]
        );
        assert_eq!(
            sig_codes(&format!("{base}pub model M : J {{ priv bore = 1mm }}\n")),
            vec!["E017"]
        );
        assert!(
            sig_codes(&format!(
                "{base}pub model M : I {{ bore = 1mm\n  priv extra = 2mm }}\n"
            ))
            .is_empty()
        );
    }

    // frob:tests crates/cadetta-sema/src/visibility.rs::std_exports_only_pub_items kind=unit
    #[test]
    fn std_exports_only_pub_items() {
        let good = file("pub model A { a = 1 }\npub fn f(x: Length) -> Length { x }\n");
        let bad =
            file("pub model A { a = 1 }\nmodel B { a = 1 }\npub(package) model C { a = 1 }\n");
        for (f, expect) in [(&good, 0usize), (&bad, 2)] {
            let pk = vec![pkg("std", &[], vec![(&[], f)])];
            let (r, _) = Resolver::new(pk.clone());
            let v = VisibilityChecker::new(&r, &pk);
            assert_eq!(v.non_pub_items("std").len(), expect);
        }
    }
}
