//! Name resolution and canonical paths (language.md 3.1 and 4, packages.md 2 and 4, ir.md 2).
//!
//! A [`Resolver`] indexes the items of every module of every package once, so
//! resolution never depends on declaration order (all lookups go through
//! sorted maps, never through source position). References are resolved from
//! a [`Ctx`] (package, module, component, locals) to a [`Resolved`] whose
//! [`CanonPath`] is the identity used by the IR, waivers and diagnostics.
//!
//! Decisions made here (written into language.md 4): enums are supplied as
//! [`EnumDecl`]s because the parser does not read `enum` items yet; visibility
//! (E016/E017) is not enforced here (lint family, not resolution); a
//! non-literal index in a reference is rejected because a canonical path
//! carries only integers (ir.md 2).

// frob:ticket 01M4SPWPCW1C0QC67GZG99EHTZ

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use cadetta_syntax::ast::{
    Arg, Block, Component, Ctor, Expr, ExprKind, File, FnDecl, Ident, Interface, InterfaceMemberKind,
    Item, ItemKind, Member, MemberKind, Part, Path, PathSep, Type, TypeKind, Use, UseKind, UsePath,
    UseRoot, UseTree, UseTreeKind, Vis,
};
use cadetta_syntax::span::Span;
use tracing::{debug, trace, warn};

/// Maximum depth when following `use` chains and glob re-exports.
const MAX_USE_DEPTH: usize = 32;

/// Built-in type names that need no `use` (language.md 4, the prelude).
const PRELUDE_TYPES: &[&str] = &[
    "Bool", "Text", "Count", "Ratio", "Point2", "Point3", "Vec2", "Vec3", "Datum", "Table",
];

pub use cadetta_ir::canon::{CanonPath, Index, MemberSeg, PartSeg};

/// An `enum` declaration, supplied by the caller until the parser reads `enum` items.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EnumDecl {
    /// The enum name.
    pub name: Ident,
    /// Its variants.
    pub variants: Vec<String>,
}

/// One parsed module of a package.
#[derive(Clone, Debug)]
pub struct ModuleSrc<'a> {
    /// The module path below the package root (empty for the root).
    pub path: Vec<String>,
    /// The parsed file.
    pub file: &'a File,
    /// The module's enums.
    pub enums: &'a [EnumDecl],
}

/// A package: its name, declared dependencies and modules.
#[derive(Clone, Debug)]
pub struct PackageSrc<'a> {
    /// The dotted package name.
    pub name: String,
    /// Names of the dependencies it may reach (`std` is always reachable).
    pub deps: Vec<String>,
    /// Its modules.
    pub modules: Vec<ModuleSrc<'a>>,
}

/// The module a diagnostic's span lies in: its package and module path.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ModuleKey {
    /// The dotted package name.
    pub package: String,
    /// The module path below the package root (empty for `lib.cdt`).
    pub module: Vec<String>,
}

impl ModuleKey {
    /// The module a package name and module path name.
    pub fn new(package: &str, module: &[String]) -> Self {
        Self {
            package: package.to_owned(),
            module: module.to_vec(),
        }
    }

    /// The module a reference context sits in.
    pub fn of_ctx(ctx: &Ctx) -> Self {
        Self::new(&ctx.package, &ctx.module)
    }

    /// The module a canonical item path lives in.
    pub fn of_path(path: &CanonPath) -> Self {
        Self::new(&path.package, &path.modules)
    }
}

/// A resolution diagnostic with its fixed E code (lints.md E002, E003, E004, E006, E011).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResolveError {
    /// The diagnostic code, such as `E002`.
    pub code: &'static str,
    /// The message.
    pub message: String,
    /// The primary span.
    pub span: Span,
    /// Other spans (both definitions of an E003).
    pub related: Vec<Span>,
    /// The module the primary span lies in, when known.
    pub module: Option<Box<ModuleKey>>,
}

impl ResolveError {
    /// Records `ctx`'s module as the span's module unless one is already set.
    pub fn in_ctx(mut self, ctx: &Ctx) -> Self {
        if self.module.is_none() {
            self.module = Some(Box::new(ModuleKey::of_ctx(ctx)));
        }
        self
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ResolveError {}

fn err<T>(code: &'static str, span: Span, message: String) -> Result<T, ResolveError> {
    warn!(code, %message, "resolution error");
    Err(ResolveError {
        module: None,
        code,
        message,
        span,
        related: Vec::new(),
    })
}

/// What an item is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemClass {
    /// `ots`, `model` or `assembly`.
    Component,
    /// An interface.
    Interface,
    /// An enum.
    Enum,
    /// A `fn`.
    Fn,
    /// A `cite`.
    Cite,
    /// A `table`.
    Table,
}

/// Where a reference is made from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ctx {
    /// The current package.
    pub package: String,
    /// The current module path.
    pub module: Vec<String>,
    /// The enclosing component, if any.
    pub component: Option<String>,
    /// Names bound by `for`, `let` or a block around the reference.
    pub locals: Vec<String>,
}

/// How an instance-level read is anchored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    /// A declaration (own member or independent `Type.member`).
    Definition,
    /// The one implicit instance of a parameterless component (language.md 3.1).
    ImplicitInstance,
    /// A `part` instance path.
    Instance,
}

/// The result of resolving a reference.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Resolved {
    /// A value member.
    Member {
        /// The canonical path.
        path: CanonPath,
        /// Definition, implicit instance or instance.
        level: Level,
        /// Set (N006) to the `part` name when the read is `Type.member` beside a part of that type.
        beside_instance: Option<String>,
    },
    /// A component argument, with any member chain read from it.
    Arg {
        /// The component path (definition) with the argument and chain as members.
        path: CanonPath,
        /// The argument name.
        name: String,
    },
    /// A `part` slot (whole, or one instance when indexed).
    Part {
        /// The instance path.
        path: CanonPath,
    },
    /// A local binding.
    Local(String),
    /// An item named without a member.
    Item {
        /// The item path.
        path: CanonPath,
        /// What it is.
        class: ItemClass,
    },
    /// An enum variant.
    Variant {
        /// The enum.
        enum_path: CanonPath,
        /// The variant.
        variant: String,
    },
    /// A prelude name (`origin` or a built-in type).
    Prelude(String),
}

enum EntryKind<'a> {
    Component(&'a Component),
    Interface(&'a Interface),
    Enum(Vec<String>),
    Fn(&'a FnDecl),
    Cite,
    Table,
}

struct Entry<'a> {
    kind: EntryKind<'a>,
    span: Span,
}

impl Entry<'_> {
    fn class(&self) -> ItemClass {
        match self.kind {
            EntryKind::Component(_) => ItemClass::Component,
            EntryKind::Interface(_) => ItemClass::Interface,
            EntryKind::Enum(_) => ItemClass::Enum,
            EntryKind::Fn(_) => ItemClass::Fn,
            EntryKind::Cite => ItemClass::Cite,
            EntryKind::Table => ItemClass::Table,
        }
    }
}

struct ModInfo<'a> {
    file: &'a File,
    items: BTreeMap<String, Entry<'a>>,
}

struct PkgInfo<'a> {
    deps: BTreeSet<String>,
    modules: BTreeMap<Vec<String>, ModInfo<'a>>,
}

/// What a name or `use` path points at.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Target {
    Item(CanonPath),
    Module(String, Vec<String>),
}

/// One step of a flattened reference.
#[derive(Clone, Debug)]
enum Step {
    Name(String, Span, PathSep),
    Index(Index),
}

struct Import<'u> {
    local: Option<String>,
    root: &'u UseRoot,
    segs: Vec<&'u Ident>,
    vis: Vis,
    span: Span,
}

/// The symbol tables of a project and the entry points that resolve references.
pub struct Resolver<'a> {
    pkgs: BTreeMap<String, PkgInfo<'a>>,
    prelude: BTreeSet<String>,
}

impl<'a> Resolver<'a> {
    /// Indexes every item; returns the resolver and the E003 duplicate-definition errors.
    pub fn new(packages: Vec<PackageSrc<'a>>) -> (Self, Vec<ResolveError>) {
        let mut errors = Vec::new();
        let mut pkgs = BTreeMap::new();
        for p in packages {
            let mut modules = BTreeMap::new();
            for m in p.modules {
                let mut items: BTreeMap<String, Entry<'a>> = BTreeMap::new();
                let here = ModuleKey::new(&p.name, &m.path);
                let mut add = |name: &Ident, kind: EntryKind<'a>| {
                    let entry = Entry {
                        kind,
                        span: name.span,
                    };
                    if let Some(prev) = items.get(&name.name) {
                        warn!(item = %name.name, "duplicate definition");
                        errors.push(ResolveError {
                            module: Some(Box::new(here.clone())),
                            code: "E003",
                            message: format!("`{}` is defined twice in one module", name.name),
                            span: name.span,
                            related: vec![prev.span],
                        });
                    } else {
                        items.insert(name.name.clone(), entry);
                    }
                };
                for it in m.file.items() {
                    index_item(it, &mut add);
                }
                for e in m.enums {
                    add(&e.name, EntryKind::Enum(e.variants.clone()));
                }
                debug!(package = %p.name, module = ?m.path, items = items.len(), "indexed module");
                modules.insert(
                    m.path,
                    ModInfo {
                        file: m.file,
                        items,
                    },
                );
            }
            pkgs.insert(
                p.name,
                PkgInfo {
                    deps: p.deps.into_iter().collect(),
                    modules,
                },
            );
        }
        errors.sort_by_key(|e| (e.span, e.code));
        let prelude = PRELUDE_TYPES.iter().map(|s| (*s).to_owned()).collect();
        (Self { pkgs, prelude }, errors)
    }

    /// Adds prelude names (units, kinds, built-in functions) that need no `use`.
    pub fn add_prelude<I: IntoIterator<Item = String>>(&mut self, names: I) {
        self.prelude.extend(names);
    }

    /// Resolves a source path such as `Other.param` or `Material::Al6061T6`.
    pub fn resolve_path(
        &self,
        ctx: &Ctx,
        path: &Path,
        expected: Option<&Type>,
    ) -> Result<Resolved, ResolveError> {
        let steps = path
            .segments
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let sep = if i == 0 {
                    PathSep::Dot
                } else {
                    path.seps[i - 1]
                };
                Step::Name(s.name.clone(), s.span, sep)
            })
            .collect::<Vec<_>>();
        self.resolve_steps(ctx, &steps, path.span, expected)
            .map_err(|e| e.in_ctx(ctx))
    }

    /// Resolves a reference expression: a path, `.field` and `[index]` chain.
    pub fn resolve_ref(
        &self,
        ctx: &Ctx,
        expr: &Expr,
        expected: Option<&Type>,
    ) -> Result<Resolved, ResolveError> {
        let mut steps = Vec::new();
        flatten(expr, &mut steps).map_err(|e| e.in_ctx(ctx))?;
        self.resolve_steps(ctx, &steps, expr.span, expected)
            .map_err(|e| e.in_ctx(ctx))
    }

    /// Resolves argument `index` of a constructor `callee(...)`, using the
    /// parameter's declared type so a bare enum variant is accepted (language.md 3.1).
    pub fn resolve_arg(
        &self,
        ctx: &Ctx,
        callee: &Path,
        index: usize,
        arg: &Arg,
    ) -> Result<Resolved, ResolveError> {
        self.resolve_arg_inner(ctx, callee, index, arg)
            .map_err(|e| e.in_ctx(ctx))
    }

    fn resolve_arg_inner(
        &self,
        ctx: &Ctx,
        callee: &Path,
        index: usize,
        arg: &Arg,
    ) -> Result<Resolved, ResolveError> {
        let comp = match self.resolve_path(ctx, callee, None)? {
            Resolved::Item {
                path,
                class: ItemClass::Component,
            } => path,
            _ => {
                return err(
                    "E006",
                    callee.span,
                    format!("`{}` is not a component", callee.text()),
                );
            }
        };
        let c = self.component(&comp).expect("resolved component exists");
        let param = match &arg.name {
            Some(n) => c.params.iter().find(|p| p.name.name == n.name),
            None => c.params.get(index),
        };
        let Some(param) = param else {
            return err(
                "E006",
                arg.span,
                format!("`{}` has no such parameter", callee.text()),
            );
        };
        // The argument resolves at the call site (`ctx`); the parameter's type
        // is resolved in the callee's module for the enum check.
        self.resolve_ref_in_param_type(ctx, &arg.value, &param.ty, &comp)
    }

    pub(crate) fn resolve_ref_in_param_type(
        &self,
        ctx: &Ctx,
        value: &Expr,
        ty: &Type,
        comp: &CanonPath,
    ) -> Result<Resolved, ResolveError> {
        self.resolve_param_ref(ctx, value, ty, comp)
            .map_err(|e| e.in_ctx(ctx))
    }

    fn resolve_param_ref(
        &self,
        ctx: &Ctx,
        value: &Expr,
        ty: &Type,
        comp: &CanonPath,
    ) -> Result<Resolved, ResolveError> {
        let callee_ctx = Ctx {
            package: comp.package.clone(),
            module: comp.modules.clone(),
            component: None,
            locals: Vec::new(),
        };
        let mut steps = Vec::new();
        flatten(value, &mut steps)?;
        // A lone bare name that is not otherwise in scope may be a variant of the parameter's enum.
        if let [Step::Name(n, _, _)] = steps.as_slice()
            && let Some(enum_path) = self.enum_of_type(&callee_ctx, ty)
            && self.enum_has(&enum_path, n)
            && !self.name_in_scope(ctx, n)
        {
            debug!(%enum_path, variant = %n, "bare variant accepted");
            return Ok(Resolved::Variant {
                enum_path,
                variant: n.clone(),
            });
        }
        self.resolve_steps(ctx, &steps, value.span, None)
    }

    fn name_in_scope(&self, ctx: &Ctx, name: &str) -> bool {
        ctx.locals.iter().any(|l| l == name)
            || ctx
                .component
                .as_deref()
                .and_then(|c| self.component(&CanonPath::item(&ctx.package, &ctx.module, c)))
                .is_some_and(|c| {
                    c.params.iter().any(|p| p.name.name == name) || find_member(c, name).is_some()
                })
            || matches!(
                self.scope_lookup(&ctx.package, &ctx.module, name, false, 0),
                Ok(Some(_))
            )
    }

    fn enum_of_type(&self, ctx: &Ctx, ty: &Type) -> Option<CanonPath> {
        let TypeKind::Path(p) = &ty.kind else {
            return None;
        };
        match self.resolve_path(ctx, p, None) {
            Ok(Resolved::Item {
                path,
                class: ItemClass::Enum,
            }) => Some(path),
            _ => None,
        }
    }

    fn enum_has(&self, path: &CanonPath, variant: &str) -> bool {
        matches!(self.entry(path), Some(Entry { kind: EntryKind::Enum(e), .. })
            if e.iter().any(|v| v == variant))
    }

    fn entry(&self, id: &CanonPath) -> Option<&Entry<'a>> {
        self.pkgs
            .get(&id.package)?
            .modules
            .get(&id.modules)?
            .items
            .get(&id.item)
    }

    /// The variants of the enum at `path`, in declaration order.
    // frob:ticket 01M4ST7ASWYQV13AAHHCXVR570
    pub fn enum_variants(&self, path: &CanonPath) -> Option<&[String]> {
        match &self.entry(path)?.kind {
            EntryKind::Enum(v) => Some(v),
            _ => None,
        }
    }

    /// The `fn` declared at `path`.
    // frob:ticket 01M4ST7ASWYQV13AAHHCXVR570
    pub fn function(&self, path: &CanonPath) -> Option<&'a FnDecl> {
        match self.entry(path)?.kind {
            EntryKind::Fn(f) => Some(f),
            _ => None,
        }
    }

    /// Whether any module declares a `kind` named `name` (user kinds are not indexed items).
    // frob:ticket 01M4V1HDNKJ9095119JPVAKYG1
    pub fn declares_kind(&self, name: &str) -> bool {
        self.pkgs.values().flat_map(|p| p.modules.values()).any(|m| {
            m.file
                .items()
                .any(|i| matches!(&i.kind, ItemKind::Kind(k) if k.name.name == name))
        })
    }

    /// The component declared at `id`.
    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
    pub fn component(&self, id: &CanonPath) -> Option<&'a Component> {
        match self.entry(id)?.kind {
            EntryKind::Component(c) => Some(c),
            _ => None,
        }
    }

    /// The component a part slot's declared type names, resolved from the slot's module.
    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
    pub fn part_type(
        &self,
        package: &str,
        module: &[String],
        ty: &Type,
    ) -> Option<(CanonPath, &'a Component)> {
        let TypeKind::Path(tp) = &ty.kind else {
            return None;
        };
        let scope = Ctx {
            package: package.to_owned(),
            module: module.to_vec(),
            component: None,
            locals: Vec::new(),
        };
        let Ok(Resolved::Item { path, .. }) = self.resolve_path(&scope, tp, None) else {
            return None;
        };
        Some((path.clone(), self.component(&path)?))
    }

    /// The component a part slot stands for: its declared component type, or, for an
    /// interface-typed slot, the component its primary constructor names.
    // frob:ticket 01M4Y77276XRHD2NDXHJMCB8NQ
    pub fn slot_component(
        &self,
        package: &str,
        module: &[String],
        part: &Part,
    ) -> Option<(CanonPath, &'a Component)> {
        if let Some(found) = self.part_type(package, module, &part.ty) {
            return Some(found);
        }
        let primary = part.primary.as_ref()?;
        let found = self.ctor_component(package, module, primary)?;
        trace!(slot = %part.name.name, primary = %found.0, "interface slot resolved to its primary");
        Some(found)
    }

    /// The component a constructor (`MotorA`, `MotorA(x = 1)`) names, resolved from `module`.
    // frob:ticket 01M4TRMJMNWCMHCE2H3BJF002X
    pub fn ctor_component(
        &self,
        package: &str,
        module: &[String],
        ctor: &Ctor,
    ) -> Option<(CanonPath, &'a Component)> {
        let scope = Ctx {
            package: package.to_owned(),
            module: module.to_vec(),
            component: None,
            locals: Vec::new(),
        };
        let Ok(Resolved::Item { path, .. }) = self.resolve_path(&scope, &ctor.path, None) else {
            return None;
        };
        Some((path.clone(), self.component(&path)?))
    }

    fn resolve_steps(
        &self,
        ctx: &Ctx,
        steps: &[Step],
        span: Span,
        expected: Option<&Type>,
    ) -> Result<Resolved, ResolveError> {
        let result = self.resolve_steps_inner(ctx, steps, span, expected);
        if let Ok(r) = &result {
            trace!(?r, "resolved");
        }
        result
    }

    fn resolve_steps_inner(
        &self,
        ctx: &Ctx,
        steps: &[Step],
        span: Span,
        expected: Option<&Type>,
    ) -> Result<Resolved, ResolveError> {
        let Some(Step::Name(first, first_span, _)) = steps.first() else {
            return err("E002", span, "empty reference".to_owned());
        };
        // The qualified prefix ends at the last `::` of the leading run of names.
        let mut last_colon = None;
        for (i, s) in steps.iter().enumerate() {
            match s {
                Step::Name(_, _, PathSep::ColonColon) => last_colon = Some(i),
                Step::Name(..) => {}
                Step::Index(_) => break,
            }
        }
        if let Some(i) = last_colon {
            return self.resolve_qualified(ctx, steps, i, span);
        }
        if ctx.locals.iter().any(|l| l == first) {
            return Ok(Resolved::Local(first.clone()));
        }
        if let Some(cname) = &ctx.component {
            let cid = CanonPath::item(&ctx.package, &ctx.module, cname);
            if let Some(c) = self.component(&cid) {
                if first == "self" {
                    return Ok(Resolved::Item {
                        path: cid,
                        class: ItemClass::Component,
                    });
                }
                if let Some(r) = self.resolve_own(ctx, &cid, c, steps)? {
                    return Ok(r);
                }
            }
        }
        if let Some(target) = self.scope_lookup(&ctx.package, &ctx.module, first, false, 0)? {
            return match target {
                Target::Item(id) => self.item_read(ctx, id, &steps[1..], *first_span),
                Target::Module(..) => err(
                    "E002",
                    *first_span,
                    format!("`{first}` is a module, not a value"),
                ),
            };
        }
        if let Some(ty) = expected
            && let (Some(ep), 1) = (self.enum_of_type(ctx, ty), steps.len())
            && self.enum_has(&ep, first)
        {
            return Ok(Resolved::Variant {
                enum_path: ep,
                variant: first.clone(),
            });
        }
        if first == "origin" || self.prelude.contains(first) {
            return Ok(Resolved::Prelude(first.clone()));
        }
        err(
            "E002",
            *first_span,
            format!("cannot find `{first}` in this scope"),
        )
    }

    /// Resolves a bare name against the component's own arguments, members and parts.
    fn resolve_own(
        &self,
        ctx: &Ctx,
        cid: &CanonPath,
        c: &Component,
        steps: &[Step],
    ) -> Result<Option<Resolved>, ResolveError> {
        let Step::Name(first, _, _) = &steps[0] else {
            return Ok(None);
        };
        if c.params.iter().any(|p| p.name.name == *first) {
            let mut path = cid.clone();
            path.members = member_segs(&steps[..1])?
                .into_iter()
                .chain(member_segs(&steps[1..])?)
                .collect();
            return Ok(Some(Resolved::Arg {
                path,
                name: first.clone(),
            }));
        }
        if let Some(m) = find_member(c, first) {
            if let MemberKind::Part(part) = &m.kind {
                return self.part_read(ctx, cid, part, steps).map(Some);
            }
            let mut path = cid.clone();
            path.members = std::iter::once(MemberSeg::Name(first.clone()))
                .chain(member_segs(&steps[1..])?)
                .collect();
            return Ok(Some(Resolved::Member {
                path,
                level: Level::Definition,
                beside_instance: None,
            }));
        }
        Ok(None)
    }

    /// `part`, `part[i]`, `part.member`, `part[i].member...` inside an assembly.
    fn part_read(
        &self,
        ctx: &Ctx,
        cid: &CanonPath,
        part: &cadetta_syntax::ast::Part,
        steps: &[Step],
    ) -> Result<Resolved, ResolveError> {
        let mut path = cid.clone();
        let mut rest = &steps[1..];
        let mut index = None;
        if let Some(Step::Index(i)) = rest.first() {
            index = Some(*i);
            rest = &rest[1..];
        }
        path.parts.push(PartSeg {
            name: part.name.name.clone(),
            index,
        });
        // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
        let mut ty = &part.ty;
        let mut scope = Ctx {
            component: None,
            locals: Vec::new(),
            ..ctx.clone()
        };
        loop {
            let Some(Step::Name(member, mspan, _)) = rest.first() else {
                return Ok(Resolved::Part { path });
            };
            let TypeKind::Path(tp) = &ty.kind else {
                break;
            };
            let Ok(Resolved::Item { path: tid, .. }) = self.resolve_path(&scope, tp, None) else {
                break;
            };
            let known = match self.entry(&tid).map(|e| &e.kind) {
                Some(EntryKind::Component(tc)) => {
                    // A part of the part's type is a further slot of the chain (`sub.bracket.x`).
                    if let Some(MemberKind::Part(inner)) = find_member(tc, member).map(|m| &m.kind)
                    {
                        let mut next = &rest[1..];
                        let mut index = None;
                        if let Some(Step::Index(i)) = next.first() {
                            index = Some(*i);
                            next = &next[1..];
                        }
                        path.parts.push(PartSeg {
                            name: inner.name.name.clone(),
                            index,
                        });
                        rest = next;
                        ty = &inner.ty;
                        scope = Ctx {
                            package: tid.package.clone(),
                            module: tid.modules.clone(),
                            component: None,
                            locals: Vec::new(),
                        };
                        continue;
                    }
                    tc.params.iter().any(|p| p.name.name == *member)
                        || find_member(tc, member).is_some()
                        || tc.members.iter().any(
                            |m| matches!(&m.kind, MemberKind::Datum(d) if d.name.name == *member),
                        )
                }
                Some(EntryKind::Interface(i)) => self.interface_has(&tid, i, member, 0),
                _ => false,
            };
            if !known {
                return err(
                    "E002",
                    *mspan,
                    format!("`{}` has no member `{member}`", tid.item),
                );
            }
            break;
        }
        path.members = member_segs(rest)?;
        Ok(Resolved::Member {
            path,
            level: Level::Instance,
            beside_instance: None,
        })
    }

    fn interface_has(&self, id: &CanonPath, i: &Interface, member: &str, depth: usize) -> bool {
        if i.members.iter().any(
            |m| matches!(&m.kind, InterfaceMemberKind::Field { name, .. } if name.name == member),
        ) {
            return true;
        }
        if depth >= MAX_USE_DEPTH {
            return false;
        }
        let ctx = Ctx {
            package: id.package.clone(),
            module: id.modules.clone(),
            component: None,
            locals: Vec::new(),
        };
        i.extends
            .iter()
            .any(|p| match self.resolve_path(&ctx, p, None) {
                Ok(Resolved::Item { path, .. }) => match self.entry(&path).map(|e| &e.kind) {
                    Some(EntryKind::Interface(parent)) => {
                        self.interface_has(&path, parent, member, depth + 1)
                    }
                    _ => false,
                },
                _ => false,
            })
    }

    /// Reads from an item found by name: the item itself or `Type.member...`.
    fn item_read(
        &self,
        ctx: &Ctx,
        id: CanonPath,
        rest: &[Step],
        span: Span,
    ) -> Result<Resolved, ResolveError> {
        let class = self.entry(&id).expect("target exists").class();
        let Some(Step::Name(member, mspan, _)) = rest.first() else {
            return Ok(Resolved::Item { path: id, class });
        };
        match self.entry(&id).map(|e| &e.kind) {
            Some(EntryKind::Component(c)) => {
                self.type_read(ctx, id.clone(), c, member, *mspan, rest)
            }
            Some(EntryKind::Enum(e)) => {
                if e.iter().any(|v| v == member) {
                    Ok(Resolved::Variant {
                        enum_path: id,
                        variant: member.clone(),
                    })
                } else {
                    err(
                        "E002",
                        *mspan,
                        format!("enum `{}` has no variant `{member}`", id.item),
                    )
                }
            }
            _ => err(
                "E002",
                span,
                format!("`{}` has no readable member `{member}`", id.item),
            ),
        }
    }

    /// `Type.member` (language.md 3.1): definition read, implicit instance or E011.
    fn type_read(
        &self,
        ctx: &Ctx,
        id: CanonPath,
        c: &Component,
        member: &str,
        mspan: Span,
        rest: &[Step],
    ) -> Result<Resolved, ResolveError> {
        if let Some(p) = c.params.iter().find(|p| p.name.name == member) {
            return err(
                "E011",
                mspan,
                format!(
                    "`{}.{member}` depends on parameter `{}`; read it through an instance",
                    id.item, p.name.name
                ),
            );
        }
        let Some(m) = find_member(c, member) else {
            return err(
                "E002",
                mspan,
                format!("`{}` has no member `{member}`", id.item),
            );
        };
        if let Some(param) = self.first_param_dep(c, m, &mut Vec::new()) {
            return err(
                "E011",
                mspan,
                format!(
                    "`{}.{member}` depends on parameter `{param}`; read it through an instance",
                    id.item
                ),
            );
        }
        let mut path = id.clone();
        path.members = member_segs(rest)?;
        let level = if c.params.is_empty() {
            Level::ImplicitInstance
        } else {
            Level::Definition
        };
        let beside_instance = ctx.component.as_ref().and_then(|cn| {
            let here = self.component(&CanonPath::item(&ctx.package, &ctx.module, cn))?;
            here.members.iter().find_map(|hm| match &hm.kind {
                MemberKind::Part(p) if is_type_named(&p.ty, &id.item) => Some(p.name.name.clone()),
                _ => None,
            })
        });
        if let Some(part) = &beside_instance {
            debug!(%id, %part, "type read beside an instance (N006)");
        }
        Ok(Resolved::Member {
            path,
            level,
            beside_instance,
        })
    }

    /// The first parameter (declaration order) a member depends on, directly or through members.
    fn first_param_dep(
        &self,
        c: &Component,
        m: &Member,
        visiting: &mut Vec<String>,
    ) -> Option<String> {
        let name = member_name(m)?;
        if visiting.iter().any(|v| v == name) {
            return None;
        }
        visiting.push(name.to_owned());
        let mut heads = Vec::new();
        member_heads(m, &mut heads);
        let found = c
            .params
            .iter()
            .find(|p| heads.contains(&p.name.name))
            .map(|p| p.name.name.clone())
            .or_else(|| {
                heads.iter().find_map(|h| {
                    let sib = find_member(c, h)?;
                    self.first_param_dep(c, sib, visiting)
                })
            });
        visiting.pop();
        found
    }

    fn resolve_qualified(
        &self,
        ctx: &Ctx,
        steps: &[Step],
        last: usize,
        span: Span,
    ) -> Result<Resolved, ResolveError> {
        let names = steps[..=last]
            .iter()
            .map(|s| match s {
                Step::Name(n, sp, sep) => (n.as_str(), *sp, *sep),
                Step::Index(_) => unreachable!("prefix is names only"),
            })
            .collect::<Vec<_>>();
        let colon = names
            .iter()
            .position(|(_, _, sep)| *sep == PathSep::ColonColon)
            .expect("a `::` exists");
        let head = names[..colon]
            .iter()
            .map(|(n, _, _)| *n)
            .collect::<Vec<_>>()
            .join(".");
        let after = &names[colon..];
        let mut cur = match head.as_str() {
            "package" => Target::Module(ctx.package.clone(), Vec::new()),
            "self" => Target::Module(ctx.package.clone(), ctx.module.clone()),
            "super" => Target::Module(ctx.package.clone(), parent(&ctx.module, 1, span)?),
            h if self.package_reachable(&ctx.package, h) => {
                Target::Module(h.to_owned(), Vec::new())
            }
            h if !h.contains('.') => {
                match self.scope_lookup(&ctx.package, &ctx.module, h, false, 0)? {
                    Some(t) => t,
                    None => {
                        return err(
                            "E002",
                            names[0].1,
                            format!("cannot find `{h}` in this scope"),
                        );
                    }
                }
            }
            h => return err("E002", names[0].1, format!("unknown package `{h}`")),
        };
        let mut result_item = None;
        for (k, (name, nspan, _)) in after.iter().enumerate() {
            let is_last = k + 1 == after.len();
            match cur.clone() {
                Target::Module(p, m) => {
                    if *name == "super" && head == "super" {
                        cur = Target::Module(p, parent(&m, 1, *nspan)?);
                        continue;
                    }
                    let mut child = m.clone();
                    child.push((*name).to_owned());
                    let is_mod = self.is_module(&p, &child);
                    if is_mod && !is_last {
                        cur = Target::Module(p, child);
                    } else {
                        let foreign = p != ctx.package || m != ctx.module;
                        match self.scope_lookup(&p, &m, name, foreign, 0)? {
                            Some(t) => cur = t,
                            None => {
                                return err(
                                    "E002",
                                    *nspan,
                                    format!("cannot find `{name}` in `{p}::{}`", m.join("::")),
                                );
                            }
                        }
                    }
                }
                Target::Item(id) => {
                    // Only `Enum::Variant` continues after an item.
                    let EntryKind::Enum(e) = &self.entry(&id).expect("item exists").kind else {
                        return err(
                            "E002",
                            *nspan,
                            format!("`{}` has no item `{name}`", id.item),
                        );
                    };
                    if !is_last || !e.iter().any(|v| v == name) {
                        return err(
                            "E002",
                            *nspan,
                            format!("enum `{}` has no variant `{name}`", id.item),
                        );
                    }
                    result_item = Some(Resolved::Variant {
                        enum_path: id,
                        variant: (*name).to_owned(),
                    });
                    cur = Target::Module(String::new(), Vec::new());
                }
            }
        }
        if let Some(v) = result_item {
            return Ok(v);
        }
        match cur {
            Target::Item(id) => self.item_read(ctx, id, &steps[last + 1..], span),
            Target::Module(..) => err("E002", span, "a module is not a value".to_owned()),
        }
    }

    fn package_reachable(&self, from: &str, name: &str) -> bool {
        name == from
            || name == "std" && self.pkgs.contains_key("std")
            || self.pkgs.get(from).is_some_and(|p| p.deps.contains(name))
                && self.pkgs.contains_key(name)
    }

    /// Looks `name` up in a module: own items, explicit `use`s, then globs (packages.md 4).
    fn scope_lookup(
        &self,
        pkg: &str,
        module: &[String],
        name: &str,
        only_pub: bool,
        depth: usize,
    ) -> Result<Option<Target>, ResolveError> {
        if depth > MAX_USE_DEPTH {
            return err(
                "E002",
                Span::default(),
                format!("`use` chain too deep at `{name}`"),
            );
        }
        let Some(mi) = self.pkgs.get(pkg).and_then(|p| p.modules.get(module)) else {
            return Ok(None);
        };
        if mi.items.contains_key(name) {
            return Ok(Some(Target::Item(CanonPath::item(pkg, module, name))));
        }
        let (named, globs) = collect_imports(mi.file);
        let mut hits: Vec<(Target, Span)> = Vec::new();
        for imp in named
            .iter()
            .filter(|i| i.local.as_deref() == Some(name) && (!only_pub || i.vis != Vis::Private))
        {
            let t = self.resolve_import(pkg, module, imp, depth + 1)?;
            push_unique(&mut hits, t, imp.span);
        }
        if hits.is_empty() {
            for imp in globs.iter().filter(|i| !only_pub || i.vis != Vis::Private) {
                let Target::Module(p, m) = self.resolve_import(pkg, module, imp, depth + 1)? else {
                    continue;
                };
                if let Some(t) = self.scope_lookup(&p, &m, name, true, depth + 1)? {
                    push_unique(&mut hits, t, imp.span);
                }
            }
        }
        match hits.len() {
            0 => Ok(None),
            1 => Ok(hits.pop().map(|(t, _)| t)),
            _ => Err(ResolveError {
                module: Some(Box::new(ModuleKey::new(pkg, module))),
                code: "E004",
                message: format!("`{name}` is brought in by more than one `use`"),
                span: hits[0].1,
                related: hits[1..].iter().map(|(_, s)| *s).collect(),
            }),
        }
    }

    fn resolve_import(
        &self,
        pkg: &str,
        module: &[String],
        imp: &Import<'_>,
        depth: usize,
    ) -> Result<Target, ResolveError> {
        self.resolve_import_inner(pkg, module, imp, depth)
            .map_err(|mut e| {
                e.module
                    .get_or_insert_with(|| Box::new(ModuleKey::new(pkg, module)));
                e
            })
    }

    fn resolve_import_inner(
        &self,
        pkg: &str,
        module: &[String],
        imp: &Import<'_>,
        depth: usize,
    ) -> Result<Target, ResolveError> {
        let mut cur = match imp.root {
            UseRoot::Package => Target::Module(pkg.to_owned(), Vec::new()),
            UseRoot::SelfMod => Target::Module(pkg.to_owned(), module.to_vec()),
            UseRoot::Super(n) => Target::Module(pkg.to_owned(), parent(module, *n, imp.span)?),
            UseRoot::Pkg(ids) => {
                let name = ids
                    .iter()
                    .map(|i| i.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                if !self.package_reachable(pkg, &name) {
                    return err("E002", imp.span, format!("unknown package `{name}`"));
                }
                Target::Module(name, Vec::new())
            }
        };
        for (k, seg) in imp.segs.iter().enumerate() {
            let Target::Module(p, m) = cur.clone() else {
                return err("E002", seg.span, "cannot descend into an item".to_owned());
            };
            let mut child = m.clone();
            child.push(seg.name.clone());
            let is_mod = self.is_module(&p, &child);
            let is_last = k + 1 == imp.segs.len();
            if is_mod && (!is_last || !self.has_item(&p, &m, &seg.name)) {
                cur = Target::Module(p, child);
                continue;
            }
            let foreign = p != pkg || m != module;
            match self.scope_lookup(&p, &m, &seg.name, foreign, depth)? {
                Some(t) => cur = t,
                None => {
                    return err(
                        "E002",
                        seg.span,
                        format!("unresolved import `{}`", seg.name),
                    );
                }
            }
        }
        Ok(cur)
    }

    /// E002 for every `use` whose target does not exist, checked eagerly (not only when used).
    // frob:ticket 01M4V13RVF8WWS2VMYE8WN9V3R
    pub fn check_use_targets(&self) -> Vec<ResolveError> {
        let mut out = Vec::new();
        for (pkg, info) in &self.pkgs {
            for (module, mi) in &info.modules {
                let (named, globs) = collect_imports(mi.file);
                for imp in named.iter().chain(globs.iter()) {
                    if let Err(e) = self.resolve_import(pkg, module, imp, 0) {
                        warn!(package = %pkg, code = e.code, "unresolved use target");
                        out.push(e);
                    }
                }
            }
        }
        out.sort_by_key(|e| (e.module.clone(), e.span));
        out
    }

    /// Every `use` whose target is an item, as the importing context, the item and the span (for E016).
    // frob:ticket 01M4YR00X3TTZGSBAKQZDNTT6T
    pub fn use_item_targets(&self) -> Vec<(Ctx, CanonPath, Span)> {
        let mut out = Vec::new();
        for (pkg, info) in &self.pkgs {
            for (module, mi) in &info.modules {
                let (named, globs) = collect_imports(mi.file);
                let ctx = Ctx {
                    package: pkg.clone(),
                    module: module.clone(),
                    component: None,
                    locals: Vec::new(),
                };
                for imp in named.iter().chain(globs.iter()) {
                    if let Ok(Target::Item(path)) = self.resolve_import(pkg, module, imp, 0) {
                        out.push((ctx.clone(), path, imp.span));
                    }
                }
            }
        }
        out
    }

    /// True when `path` is a module or the (possibly file-less) parent of one (packages.md 2).
    fn is_module(&self, pkg: &str, path: &[String]) -> bool {
        self.pkgs
            .get(pkg)
            .is_some_and(|p| p.modules.keys().any(|k| k.starts_with(path)))
    }

    fn has_item(&self, pkg: &str, module: &[String], name: &str) -> bool {
        self.pkgs
            .get(pkg)
            .and_then(|p| p.modules.get(module))
            .is_some_and(|m| m.items.contains_key(name))
    }
}

fn push_unique(hits: &mut Vec<(Target, Span)>, t: Target, span: Span) {
    if !hits.iter().any(|(h, _)| *h == t) {
        hits.push((t, span));
    }
}

fn parent(module: &[String], n: usize, span: Span) -> Result<Vec<String>, ResolveError> {
    if n > module.len() {
        return err(
            "E002",
            span,
            "`super` goes above the package root".to_owned(),
        );
    }
    Ok(module[..module.len() - n].to_vec())
}

fn index_item<'a>(it: &'a Item, add: &mut impl FnMut(&Ident, EntryKind<'a>)) {
    match &it.kind {
        ItemKind::Component(c) => add(&c.name, EntryKind::Component(c)),
        ItemKind::Interface(i) => add(&i.name, EntryKind::Interface(i)),
        ItemKind::Cite(c) => add(&c.name, EntryKind::Cite),
        ItemKind::Enum(e) => add(
            &e.name,
            EntryKind::Enum(e.variants.iter().map(|v| v.name.clone()).collect()),
        ),
        ItemKind::Fn(f) => add(&f.name, EntryKind::Fn(f)),
        // frob:ticket 01M4TRBJ7D2JTGX6XX94BJ1ZGM
        ItemKind::Table(t) => add(&t.name, EntryKind::Table),
        _ => {}
    }
}

fn collect_imports(file: &File) -> (Vec<Import<'_>>, Vec<Import<'_>>) {
    let mut named = Vec::new();
    let mut globs = Vec::new();
    for u in file.uses() {
        flatten_use(u, &mut named, &mut globs);
    }
    (named, globs)
}

fn flatten_use<'u>(u: &'u Use, named: &mut Vec<Import<'u>>, globs: &mut Vec<Import<'u>>) {
    let UsePath { root, segments, .. } = &u.path;
    let base: Vec<&Ident> = segments.iter().collect();
    match &u.kind {
        UseKind::Single { alias } => {
            let local = alias.as_ref().map(|a| a.name.clone()).or_else(|| {
                base.last().map(|i| i.name.clone()).or_else(|| match root {
                    UseRoot::Pkg(ids) => ids.last().map(|i| i.name.clone()),
                    _ => None,
                })
            });
            named.push(Import {
                local,
                root,
                segs: base,
                vis: u.vis,
                span: u.span,
            });
        }
        UseKind::Glob => globs.push(Import {
            local: None,
            root,
            segs: base,
            vis: u.vis,
            span: u.span,
        }),
        UseKind::Tree(trees) => {
            for t in trees {
                flatten_tree(root, &base, t, u.vis, named, globs);
            }
        }
    }
}

fn flatten_tree<'u>(
    root: &'u UseRoot,
    prefix: &[&'u Ident],
    t: &'u UseTree,
    vis: Vis,
    named: &mut Vec<Import<'u>>,
    globs: &mut Vec<Import<'u>>,
) {
    if matches!(t.kind, UseTreeKind::Star) {
        globs.push(Import {
            local: None,
            root,
            segs: prefix.to_vec(),
            vis,
            span: t.span,
        });
        return;
    }
    let mut segs = prefix.to_vec();
    segs.push(&t.name);
    match &t.kind {
        UseTreeKind::Leaf { alias } => named.push(Import {
            local: Some(alias.as_ref().unwrap_or(&t.name).name.clone()),
            root,
            segs,
            vis,
            span: t.span,
        }),
        UseTreeKind::Glob => globs.push(Import {
            local: None,
            root,
            segs,
            vis,
            span: t.span,
        }),
        UseTreeKind::Group(inner) => {
            for i in inner {
                flatten_tree(root, &segs, i, vis, named, globs);
            }
        }
        UseTreeKind::Star => unreachable!("handled above"),
    }
}

fn flatten(e: &Expr, out: &mut Vec<Step>) -> Result<(), ResolveError> {
    match &e.kind {
        ExprKind::Path(p) => {
            for (i, s) in p.segments.iter().enumerate() {
                let sep = if i == 0 { PathSep::Dot } else { p.seps[i - 1] };
                out.push(Step::Name(s.name.clone(), s.span, sep));
            }
            Ok(())
        }
        ExprKind::Paren(inner) => flatten(inner, out),
        ExprKind::Field(inner, id) => {
            flatten(inner, out)?;
            out.push(Step::Name(id.name.clone(), id.span, PathSep::Dot));
            Ok(())
        }
        ExprKind::Index(inner, idx) => {
            flatten(inner, out)?;
            match &idx.kind {
                ExprKind::Number(text) if text.parse::<u64>().is_ok() => {
                    out.push(Step::Index(Index::At(text.parse().unwrap_or_default())));
                    Ok(())
                }
                _ => err(
                    "E002",
                    idx.span,
                    "a canonical path index must be an integer literal".to_owned(),
                ),
            }
        }
        _ => err("E002", e.span, "not a reference".to_owned()),
    }
}

fn member_segs(steps: &[Step]) -> Result<Vec<MemberSeg>, ResolveError> {
    Ok(steps
        .iter()
        .map(|s| match s {
            Step::Name(n, _, _) => MemberSeg::Name(n.clone()),
            Step::Index(i) => MemberSeg::Index(*i),
        })
        .collect())
}

fn member_name(m: &Member) -> Option<&str> {
    Some(match &m.kind {
        MemberKind::Param(p) => &p.name.name,
        MemberKind::Let(l) => &l.name.name,
        MemberKind::Part(p) => &p.name.name,
        MemberKind::Fit(f) => &f.name.name,
        MemberKind::Check(c) => &c.name.name,
        MemberKind::Require(r) => &r.name.name,
        _ => return None,
    })
}

fn find_member<'c>(c: &'c Component, name: &str) -> Option<&'c Member> {
    c.members.iter().find(|m| member_name(m) == Some(name))
}

fn is_type_named(t: &Type, name: &str) -> bool {
    matches!(&t.kind, TypeKind::Path(p) if p.segments.last().is_some_and(|s| s.name == name))
}

/// Leading names of every reference a member's definition reads.
fn member_heads(m: &Member, out: &mut Vec<String>) {
    let mut bound = Vec::new();
    match &m.kind {
        MemberKind::Param(p) => expr_heads(&p.value, &mut bound, out),
        MemberKind::Let(l) => expr_heads(&l.value, &mut bound, out),
        MemberKind::Check(c) => {
            if let Some((v, e)) = &c.each {
                expr_heads(e, &mut bound, out);
                bound.push(v.name.clone());
            }
            expr_heads(&c.cond, &mut bound, out);
        }
        MemberKind::Require(r) => expr_heads(&r.cond, &mut bound, out),
        MemberKind::Fit(f) => out.push(f.on.segments[0].name.clone()),
        MemberKind::Part(p) => {
            if let Some(c) = &p.count {
                expr_heads(c, &mut bound, out);
            }
            let ctors = p.primary.iter().chain(p.alts.iter());
            for args in p.args.iter().chain(ctors.filter_map(|c| c.args.as_ref())) {
                for a in args {
                    expr_heads(&a.value, &mut bound, out);
                }
            }
        }
        _ => {}
    }
}

fn expr_heads(e: &Expr, bound: &mut Vec<String>, out: &mut Vec<String>) {
    let go = |x: &Expr, b: &mut Vec<String>, o: &mut Vec<String>| expr_heads(x, b, o);
    match &e.kind {
        ExprKind::Path(p) => {
            let h = &p.segments[0].name;
            if !bound.contains(h) {
                out.push(h.clone());
            }
        }
        ExprKind::Number(_) | ExprKind::Quantity(_) | ExprKind::Str(_) | ExprKind::Bool(_) => {}
        ExprKind::Paren(x)
        | ExprKind::Unary(_, x)
        | ExprKind::Field(x, _)
        | ExprKind::As(x, _)
        | ExprKind::IsoClass(x, _) => go(x, bound, out),
        ExprKind::Array(xs) => xs.iter().for_each(|x| go(x, bound, out)),
        ExprKind::Binary(_, a, b) | ExprKind::Satisfies(a, b) | ExprKind::Index(a, b) => {
            go(a, bound, out);
            go(b, bound, out);
        }
        ExprKind::Compare { first, rest } => {
            go(first, bound, out);
            rest.iter().for_each(|(_, x)| go(x, bound, out));
        }
        ExprKind::Call { callee, args, .. } => {
            go(callee, bound, out);
            args.iter().for_each(|a| go(&a.value, bound, out));
        }
        ExprKind::Tolerance(x, terms) => {
            go(x, bound, out);
            terms.iter().for_each(|t| go(&t.value, bound, out));
        }
        ExprKind::If {
            cond,
            then,
            otherwise,
        } => {
            go(cond, bound, out);
            block_heads(then, bound, out);
            if let Some(o) = otherwise {
                go(o, bound, out);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            go(scrutinee, bound, out);
            arms.iter().for_each(|a| go(&a.body, bound, out));
        }
        ExprKind::Block(b) => block_heads(b, bound, out),
    }
}

fn block_heads(b: &Block, bound: &mut Vec<String>, out: &mut Vec<String>) {
    let mark = bound.len();
    for l in &b.lets {
        expr_heads(&l.value, bound, out);
        bound.push(l.name.name.clone());
    }
    expr_heads(&b.tail, bound, out);
    bound.truncate(mark);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::parser::parse;

    fn ctx(pkg: &str, module: &[&str], comp: Option<&str>) -> Ctx {
        Ctx {
            package: pkg.to_owned(),
            module: module.iter().map(|s| (*s).to_owned()).collect(),
            component: comp.map(str::to_owned),
            locals: Vec::new(),
        }
    }

    fn pkg<'a>(
        name: &str,
        deps: &[&str],
        mods: Vec<(&[&str], &'a File)>,
        enums: &'a [EnumDecl],
    ) -> PackageSrc<'a> {
        PackageSrc {
            name: name.to_owned(),
            deps: deps.iter().map(|s| (*s).to_owned()).collect(),
            modules: mods
                .into_iter()
                .map(|(p, file)| ModuleSrc {
                    path: p.iter().map(|s| (*s).to_owned()).collect(),
                    file,
                    enums,
                })
                .collect(),
        }
    }

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

    fn path_str(r: &Resolved) -> String {
        match r {
            Resolved::Member { path, .. }
            | Resolved::Arg { path, .. }
            | Resolved::Part { path } => path.to_string(),
            Resolved::Item { path, .. } => path.to_string(),
            other => format!("{other:?}"),
        }
    }

    const LIB: &str = "model Axle(wheel: Caster, n: Count) {\n  base = 10mm\n  end_diameter = base + 2mm\n  seat = n * 1mm\n  chained = seat\n  uses_base = base\n}\n\nmodel Frame {\n  span = 300mm\n  wall = span\n}\n\nmodel Rover {\n  part axle[2]: Axle(wheel = 1, n = 2)\n  part frame: Frame\n  total = axle[0].end_diameter + frame.span\n  again = Frame.span\n  param_read = Axle.base\n}\n";

    // frob:tests bare_name_is_own_member
    #[test]
    fn bare_name_is_own_member() {
        let f = file(LIB);
        let (r, errs) = Resolver::new(vec![pkg("acme.rover", &[], vec![(&[], &f)], &[])]);
        assert!(errs.is_empty());
        let c = ctx("acme.rover", &[], Some("Axle"));
        let got = r.resolve_ref(&c, &expr_of("base"), None).unwrap();
        assert_eq!(path_str(&got), "acme.rover::Axle.base");
        let arg = r.resolve_ref(&c, &expr_of("n"), None).unwrap();
        assert!(matches!(arg, Resolved::Arg { ref name, .. } if name == "n"));
        let e = r.resolve_ref(&c, &expr_of("nothing"), None).unwrap_err();
        assert_eq!(e.code, "E002");
    }

    // frob:tests declaration_order_is_irrelevant
    #[test]
    fn declaration_order_is_irrelevant() {
        let a = file("model A { x = B.y }\nmodel B { y = 1mm }\n");
        let b = file("model B { y = 1mm }\nmodel A { x = B.y }\n");
        let run = |f: &File| {
            let (r, errs) = Resolver::new(vec![pkg("p", &[], vec![(&[], f)], &[])]);
            assert!(errs.is_empty());
            let c = ctx("p", &[], Some("A"));
            r.resolve_ref(&c, &expr_of("B.y"), None).unwrap()
        };
        assert_eq!(run(&a), run(&b));
        assert_eq!(path_str(&run(&a)), "p::B.y");
    }

    // frob:tests type_member_on_parameterized_component
    #[test]
    fn type_member_on_parameterized_component() {
        let f = file(LIB);
        let (r, _) = Resolver::new(vec![pkg("acme.rover", &[], vec![(&[], &f)], &[])]);
        let c = ctx("acme.rover", &[], Some("Rover"));
        let ok = r.resolve_ref(&c, &expr_of("Axle.base"), None).unwrap();
        assert_eq!(path_str(&ok), "acme.rover::Axle.base");
        assert!(matches!(
            ok,
            Resolved::Member {
                level: Level::Definition,
                ..
            }
        ));
        for dep in ["Axle.seat", "Axle.chained", "Axle.n"] {
            let e = r.resolve_ref(&c, &expr_of(dep), None).unwrap_err();
            assert_eq!(e.code, "E011", "{dep}");
        }
        let e = r
            .resolve_ref(&c, &expr_of("Axle.chained"), None)
            .unwrap_err();
        assert!(e.message.contains("`n`"), "{}", e.message);
        // end_diameter depends only on `base`, so it reads without an instance.
        assert!(
            r.resolve_ref(&c, &expr_of("Axle.end_diameter"), None)
                .is_ok()
        );
    }

    // frob:tests parameterless_type_read_is_implicit_instance
    #[test]
    fn parameterless_type_read_is_implicit_instance() {
        let f = file(LIB);
        let (r, _) = Resolver::new(vec![pkg("acme.rover", &[], vec![(&[], &f)], &[])]);
        let a = r
            .resolve_ref(
                &ctx("acme.rover", &[], Some("Rover")),
                &expr_of("Frame.span"),
                None,
            )
            .unwrap();
        let b = r
            .resolve_ref(
                &ctx("acme.rover", &[], Some("Axle")),
                &expr_of("Frame.span"),
                None,
            )
            .unwrap();
        for x in [&a, &b] {
            assert!(matches!(
                x,
                Resolved::Member {
                    level: Level::ImplicitInstance,
                    ..
                }
            ));
            assert_eq!(path_str(x), "acme.rover::Frame.span");
        }
        // Rover has a `part frame: Frame`, so the type read is N006 and names the part.
        assert!(matches!(&a, Resolved::Member { beside_instance: Some(p), .. } if p == "frame"));
        assert!(matches!(
            &b,
            Resolved::Member {
                beside_instance: None,
                ..
            }
        ));
        // Distinct from the part instance.
        let inst = r
            .resolve_ref(
                &ctx("acme.rover", &[], Some("Rover")),
                &expr_of("frame.span"),
                None,
            )
            .unwrap();
        assert_eq!(path_str(&inst), "acme.rover::Rover/frame.span");
    }

    // frob:tests bare_enum_variant_needs_known_type
    #[test]
    fn bare_enum_variant_needs_known_type() {
        let f = file(
            "ots Screw(thread: MetricThread, length: Length) { pitch = 1mm }\nmodel User { s = 1 }\n",
        );
        let enums = [EnumDecl {
            name: Ident {
                name: "MetricThread".into(),
                span: Span::default(),
            },
            variants: vec!["M4".into(), "M5".into()],
        }];
        let (r, errs) = Resolver::new(vec![pkg("p", &[], vec![(&[], &f)], &enums)]);
        assert!(errs.is_empty());
        let c = ctx("p", &[], Some("User"));
        let callee = match expr_of("Screw(M5, 25mm)").kind {
            ExprKind::Call { callee, args, .. } => (callee, args),
            _ => unreachable!(),
        };
        let ExprKind::Path(cp) = &callee.0.kind else {
            unreachable!()
        };
        let got = r.resolve_arg(&c, cp, 0, &callee.1[0]).unwrap();
        assert_eq!(
            got,
            Resolved::Variant {
                enum_path: CanonPath::item("p", &[], "MetricThread"),
                variant: "M5".into()
            }
        );
        // Elsewhere a bare variant is unresolved; the qualified form works.
        assert_eq!(
            r.resolve_ref(&c, &expr_of("M5"), None).unwrap_err().code,
            "E002"
        );
        let q = r
            .resolve_ref(&c, &expr_of("MetricThread::M5"), None)
            .unwrap();
        assert!(matches!(q, Resolved::Variant { ref variant, .. } if variant == "M5"));
        let bad = r
            .resolve_ref(&c, &expr_of("MetricThread::M9"), None)
            .unwrap_err();
        assert_eq!(bad.code, "E002");
    }

    // frob:tests canonical_part_index_paths
    #[test]
    fn canonical_part_index_paths() {
        let f = file(LIB);
        let (r, _) = Resolver::new(vec![pkg("acme.rover", &[], vec![(&[], &f)], &[])]);
        let c = ctx("acme.rover", &[], Some("Rover"));
        let got = r
            .resolve_ref(&c, &expr_of("axle[0].end_diameter"), None)
            .unwrap();
        assert_eq!(path_str(&got), "acme.rover::Rover/axle[0].end_diameter");
        let slot = r.resolve_ref(&c, &expr_of("axle"), None).unwrap();
        assert_eq!(path_str(&slot), "acme.rover::Rover/axle");
        let e = r
            .resolve_ref(&c, &expr_of("axle[0].nope"), None)
            .unwrap_err();
        assert_eq!(e.code, "E002");
        for text in [
            "acme.rover::Rover/wheel[2].hub_bore",
            "acme.rover::Axle.end_diameter",
            "acme.rover::Bracket.holes[1].x",
            "acme.rover::drive::motor::Gm42A.rated_torque",
            "acme.rover::Rover/bracket.place.0.x",
            "acme.rover::Rover/feet[i2].place.0.x"
                .replace("i2", "*")
                .leak(),
            "acme.rover::Rover/axle[*].end_diameter",
        ] {
            let p = CanonPath::parse(text).unwrap();
            assert_eq!(p.to_string(), text);
        }
        assert!(CanonPath::parse("nopath").is_err());
        assert!(CanonPath::parse("a::B/x[").is_err());
    }

    // frob:tests same_module_before_imports
    #[test]
    fn same_module_before_imports() {
        let main =
            file("use acme.shop::Other\nmodel Other { param = 1mm }\nmodel User { x = 1 }\n");
        let shop = file("pub model Other { param = 2mm }\n");
        let (r, errs) = Resolver::new(vec![
            pkg("acme.rover", &["acme.shop"], vec![(&[], &main)], &[]),
            pkg("acme.shop", &[], vec![(&[], &shop)], &[]),
        ]);
        assert!(errs.is_empty());
        let got = r
            .resolve_ref(
                &ctx("acme.rover", &[], Some("User")),
                &expr_of("Other.param"),
                None,
            )
            .unwrap();
        assert_eq!(path_str(&got), "acme.rover::Other.param");
        // Without a local definition the import is used.
        let main2 = file("use acme.shop::Other\nmodel User { x = 1 }\n");
        let (r2, _) = Resolver::new(vec![
            pkg("acme.rover", &["acme.shop"], vec![(&[], &main2)], &[]),
            pkg("acme.shop", &[], vec![(&[], &shop)], &[]),
        ]);
        let got = r2
            .resolve_ref(
                &ctx("acme.rover", &[], Some("User")),
                &expr_of("Other.param"),
                None,
            )
            .unwrap();
        assert_eq!(path_str(&got), "acme.shop::Other.param");
    }

    // frob:tests qualified_package_module_item_member
    #[test]
    fn qualified_package_module_item_member() {
        let root = file("model User { x = 1 }\n");
        let motor = file("model Gm42A { rated_torque = 3N*m }\n");
        let (r, _) = Resolver::new(vec![pkg(
            "acme.rover",
            &[],
            vec![(&[], &root), (&["drive", "motor"], &motor)],
            &[],
        )]);
        let c = ctx("acme.rover", &[], Some("User"));
        let was_path = |src: &str| match &file(&format!("model T {{\n  v = 1 was {src}\n}}"))
            .components()
            .next()
            .unwrap()
            .members[0]
            .kind
        {
            MemberKind::Param(p) => p.was[0].clone(),
            _ => unreachable!(),
        };
        let want = "acme.rover::drive::motor::Gm42A.rated_torque";
        let got = r.resolve_path(&c, &was_path(want), None).unwrap();
        assert_eq!(path_str(&got), want);
        let e = r
            .resolve_path(&c, &was_path("acme.rover::drive::Missing.x"), None)
            .unwrap_err();
        assert_eq!(e.code, "E002");
    }

    // frob:tests duplicates_and_ambiguous_imports
    #[test]
    fn duplicates_and_ambiguous_imports() {
        let dup = file("model A { x = 1 }\nmodel A { y = 1 }\n");
        let (_, errs) = Resolver::new(vec![pkg("p", &[], vec![(&[], &dup)], &[])]);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, "E003");
        assert_eq!(errs[0].related.len(), 1);

        let m1 = file("pub model Same { v = 1 }\n");
        let user = file("use package::a::*\nuse package::b::*\nmodel User { x = 1 }\n");
        let (r, _) = Resolver::new(vec![pkg(
            "p",
            &[],
            vec![(&[], &user), (&["a"], &m1), (&["b"], &m1)],
            &[],
        )]);
        let e = r
            .resolve_ref(&ctx("p", &[], Some("User")), &expr_of("Same.v"), None)
            .unwrap_err();
        assert_eq!(e.code, "E004");
    }
}
