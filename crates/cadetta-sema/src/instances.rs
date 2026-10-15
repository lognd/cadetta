//! Parameterized component instances (language.md 3.3, 3.12; stdlib.md 3, 4).
//!
//! A `part j: ScrewJoint(thread = M6, ...)` inside a parameterless component is an
//! instance. Its arguments and the callee's members become definitions keyed under the
//! instance prefix `Parent/j.<name>`, so the path-keyed evaluator evaluates them like any
//! member: a given argument in the parent's scope, a default or a callee member in the
//! callee's scope with callee names mapped to the prefix. An omitted interface-typed
//! argument (`policy`) resolves to the scope default (stdlib.md 4).

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::OnceLock;

use cadetta_ir::canon::PartSeg;
use cadetta_syntax::ast::{self, AttrArg, Component, Expr, ExprKind, MemberKind, TypeKind};
use cadetta_syntax::span::Span;
use tracing::{debug, trace, warn};

use super::{Def, EvalDiagnostic, Evaluator, Fail, R, Scope, Val, diag};
use crate::defaults::{Layers, Source, resolve_default};
use crate::deps::Severity;
use crate::parts::{ArgSource, bind_args_injecting};
use crate::resolve::{
    CanonPath, Ctx, ItemClass, MemberSeg, ModuleKey, PackageSrc, Resolved, Resolver,
};

/// The package whose components supply the default policies (stdlib.md 4).
const STD: &str = "std";

/// The default policy implementation `std` offers for an interface.
const STD_DEFAULT: &str = "IsoDefault";

/// Where an instance lives and what it instantiates.
#[derive(Debug)]
pub(super) struct InstInfo {
    /// `Parent/part`, the key prefix of every instance member.
    pub prefix: CanonPath,
    /// The instantiated component.
    pub callee: CanonPath,
    /// The `part` declaration that creates the instance (where callee failures are reported).
    pub span: Span,
}

/// One materialised instance.
struct Instance<'p> {
    info: Rc<InstInfo>,
    component: &'p Component,
    callee_ctx: Ctx,
    span: Span,
}

/// Every instance of the project, plus what indexing them found.
#[derive(Default)]
pub(super) struct Instances<'p> {
    list: Vec<Instance<'p>>,
    /// Binding failures found while indexing (E006).
    pub diagnostics: Vec<EvalDiagnostic>,
    /// Instance members declared `priv`.
    pub private: BTreeSet<CanonPath>,
    /// Given arguments with no definition (a `Part` argument, evaluated by a later ticket).
    nodef: BTreeSet<CanonPath>,
}

impl<'p> Instances<'p> {
    /// Each instance as (info, component, callee context, span).
    pub fn members(&self) -> impl Iterator<Item = (Rc<InstInfo>, &'p Component, Ctx, Span)> + '_ {
        self.list
            .iter()
            .map(|i| (i.info.clone(), i.component, i.callee_ctx.clone(), i.span))
    }

    /// True when `path` is the prefix of an instance.
    pub fn is_instance(&self, path: &CanonPath) -> bool {
        self.list.iter().any(|i| i.info.prefix == *path)
    }

    /// True when `key` is a supplied argument that has no definition.
    pub fn is_nodef_arg(&self, key: &CanonPath) -> bool {
        self.nodef.contains(key)
    }
}

/// A placeholder expression for definitions whose value is fixed.
fn placeholder() -> &'static Expr {
    static P: OnceLock<Expr> = OnceLock::new();
    P.get_or_init(|| Expr {
        kind: ExprKind::Bool(false),
        span: Span { start: 0, end: 0 },
    })
}

fn bind_error(module: ModuleKey, span: Span, message: String) -> EvalDiagnostic {
    warn!(%message, "instances: argument binding failed");
    EvalDiagnostic {
        module: Some(module),
        code: "E006",
        severity: Severity::Error,
        message,
        span,
    }
}

/// Indexes the instances of every `part` of a parameterless component into `defs`.
// frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
pub(super) fn materialise<'p>(
    resolver: &Resolver<'p>,
    packages: &[PackageSrc<'p>],
    defs: &mut BTreeMap<CanonPath, Def<'p>>,
) -> Instances<'p> {
    let mut out = Instances::default();
    for pkg in packages {
        for module in &pkg.modules {
            for c in module.file.components() {
                if !c.params.is_empty() {
                    continue;
                }
                for m in &c.members {
                    let MemberKind::Part(part) = &m.kind else {
                        continue;
                    };
                    if part.primary.is_some() {
                        continue;
                    }
                    let Some((callee_path, callee)) =
                        resolver.part_type(&pkg.name, &module.path, &part.ty)
                    else {
                        continue;
                    };
                    if callee.params.is_empty() {
                        continue;
                    }
                    if part.count.is_some() {
                        if let Some(name) = pattern_arg(callee, part.args.as_deref().unwrap_or(&[]))
                        {
                            out.diagnostics.push(bind_error(
                                ModuleKey::new(&pkg.name, &module.path),
                                m.span,
                                format!(
                                    "`{}` is one part per pattern: `{}[N]` repeats it, so each \
                                     instance would take the whole `{name}` pattern",
                                    callee.name.name, part.name.name
                                ),
                            ));
                        }
                        continue;
                    }
                    let parent_ctx = Ctx {
                        package: pkg.name.clone(),
                        module: module.path.clone(),
                        component: Some(c.name.name.clone()),
                        locals: Vec::new(),
                    };
                    let mut prefix = CanonPath::item(&pkg.name, &module.path, &c.name.name);
                    prefix.parts.push(PartSeg {
                        name: part.name.name.clone(),
                        index: None,
                    });
                    let site = Site {
                        resolver,
                        packages,
                        project_defaults: &pkg.defaults,
                        parent: c,
                        parent_ctx: &parent_ctx,
                        parent_file: module.file,
                        callee_path: &callee_path,
                        callee,
                        prefix: &prefix,
                        args: part.args.as_deref().unwrap_or(&[]),
                        span: m.span,
                    };
                    site.index(defs, &mut out);
                }
            }
        }
    }
    debug!(instances = out.list.len(), "instances: materialised");
    out
}

/// One `part` site being materialised.
struct Site<'a, 'p> {
    resolver: &'a Resolver<'p>,
    packages: &'a [PackageSrc<'p>],
    /// The parent package's `[defaults]` table.
    project_defaults: &'a BTreeMap<String, String>,
    parent: &'p Component,
    parent_ctx: &'a Ctx,
    parent_file: &'p ast::File,
    callee_path: &'a CanonPath,
    callee: &'p Component,
    prefix: &'a CanonPath,
    args: &'p [ast::Arg],
    span: Span,
}

impl<'p> Site<'_, 'p> {
    fn key(&self, name: &str) -> CanonPath {
        let mut k = self.prefix.clone();
        k.members = vec![MemberSeg::Name(name.to_owned())];
        k
    }

    fn callee_ctx(&self) -> Ctx {
        Ctx {
            package: self.callee_path.package.clone(),
            module: self.callee_path.modules.clone(),
            component: Some(self.callee.name.name.clone()),
            locals: Vec::new(),
        }
    }

    /// The interface a parameter's type names, resolved in the callee's module.
    fn interface_of(&self, ty: &ast::Type) -> Option<CanonPath> {
        let TypeKind::Path(tp) = &ty.kind else {
            return None;
        };
        let scope = Ctx {
            component: None,
            ..self.callee_ctx()
        };
        match self.resolver.resolve_path(&scope, tp, None) {
            Ok(Resolved::Item {
                path,
                class: ItemClass::Interface,
            }) => Some(path),
            _ => None,
        }
    }

    /// The scope default implementation of `iface`, resolved by `defaults::resolve_default`.
    // frob:ticket 01M4WGAD346CJ141PB5JK240ZV
    fn default_impl(&self, iface: &CanonPath) -> Option<CanonPath> {
        let name = iface.item.as_str();
        let component = default_entries(&self.parent.inner_attrs);
        let file = default_entries(&self.parent_file.inner_attrs);
        let std_impl = self.std_default(name);
        let layers = Layers {
            component: Some(&component.names),
            file: Some(&file.names),
            project: Some(self.project_defaults),
            std: std_impl.as_ref().map(|_| STD_DEFAULT),
        };
        let resolution = resolve_default(name, &layers)?;
        let value = match resolution.source {
            Source::Component => component.exprs.get(name),
            Source::File => file.exprs.get(name),
            Source::Project => return self.project_default(name),
            Source::Std => None,
        };
        let Some(value) = value else {
            debug!(iface = name, source = ?resolution.source, "instances: default from non-scope layer");
            return std_impl;
        };
        match self.resolver.resolve_ref(self.parent_ctx, value, None) {
            Ok(Resolved::Item {
                path,
                class: ItemClass::Component,
            }) => Some(path),
            _ => {
                warn!(
                    iface = name,
                    "instances: scope default does not name a component"
                );
                None
            }
        }
    }

    /// The component the project `[defaults]` entry for `iface` names, from its qualified path.
    // frob:ticket 01M4W5J6FACWMX38X4FTGY21E7
    fn project_default(&self, iface: &str) -> Option<CanonPath> {
        let written = self.project_defaults.get(iface)?;
        let mut segs: Vec<&str> = written.split("::").collect();
        let item = segs.pop().filter(|i| !i.is_empty());
        let package = (!segs.is_empty()).then(|| segs.remove(0));
        let found = item.zip(package).and_then(|(item, package)| {
            self.packages
                .iter()
                .filter(|p| p.name == package)
                .flat_map(|p| p.modules.iter().map(move |m| (p, m)))
                .filter(|(_, m)| m.path.iter().map(String::as_str).eq(segs.iter().copied()))
                .find(|(_, m)| m.file.components().any(|c| c.name.name == item))
                .map(|(p, m)| CanonPath::item(&p.name, &m.path, item))
        });
        match &found {
            Some(path) => debug!(iface, %written, ?path, "instances: project default resolved"),
            None => warn!(iface, %written, "instances: project default does not name a component"),
        }
        found
    }

    /// The `std` default component implementing the interface named `name`, if any.
    fn std_default(&self, name: &str) -> Option<CanonPath> {
        self.packages
            .iter()
            .filter(|p| p.name == STD)
            .flat_map(|p| p.modules.iter().map(move |m| (p, m)))
            .flat_map(|(p, m)| m.file.components().map(move |c| (p, m, c)))
            .find(|(_, _, c)| {
                c.name.name == STD_DEFAULT
                    && c.implements
                        .iter()
                        .any(|i| i.segments.last().is_some_and(|s| s.name == name))
            })
            .map(|(p, m, c)| CanonPath::item(&p.name, &m.path, &c.name.name))
    }

    fn index(&self, defs: &mut BTreeMap<CanonPath, Def<'p>>, out: &mut Instances<'p>) {
        let injectable = |p: &ast::ParamDecl| self.interface_of(&p.ty).is_some();
        let bound = match bind_args_injecting(
            &self.callee.name.name,
            &self.callee.params,
            self.args,
            injectable,
        ) {
            Ok(b) => b,
            Err(e) => {
                out.diagnostics.push(bind_error(
                    ModuleKey::of_ctx(self.parent_ctx),
                    self.span,
                    format!("`{}`: {e}", self.callee.name.name),
                ));
                return;
            }
        };
        let info = Rc::new(InstInfo {
            prefix: self.prefix.clone(),
            callee: self.callee_path.clone(),
            span: self.span,
        });
        let callee_ctx = self.callee_ctx();
        for (param, b) in self.callee.params.iter().zip(&bound) {
            let key = self.key(b.name);
            let mut def = Def {
                expr: placeholder(),
                ty: None,
                is_let: false,
                ctx: self.parent_ctx.clone(),
                span: self.span,
                inst: None,
                fixed: None,
                when: None,
                role: None,
            };
            match &b.source {
                ArgSource::Given(expr, span) => {
                    if is_part_type(&param.ty) {
                        trace!(%key, "instances: Part argument kept without a value");
                        out.nodef.insert(key);
                        continue;
                    }
                    def.expr = expr;
                    def.span = *span;
                    def.fixed = self.fixed_value(expr, param);
                }
                ArgSource::Default(expr) => {
                    def.expr = expr;
                    def.ctx = callee_ctx.clone();
                    def.inst = Some(info.clone());
                    def.span = param.span;
                }
                ArgSource::Absent => continue,
                ArgSource::Injected => {
                    let Some(iface) = self.interface_of(&param.ty) else {
                        continue;
                    };
                    match self.default_impl(&iface) {
                        Some(imp) => def.fixed = Some(Val::Component(imp)),
                        None => {
                            out.diagnostics.push(bind_error(
                                ModuleKey::of_ctx(self.parent_ctx),
                                self.span,
                                format!(
                                    "`{}`: no argument and no default for `{}` ({})",
                                    self.callee.name.name, b.name, iface.item
                                ),
                            ));
                            continue;
                        }
                    }
                }
            }
            defs.insert(key, def);
        }
        for m in &self.callee.members {
            let (name, expr, is_let, span, ty) = match &m.kind {
                MemberKind::Param(p) => (&p.name.name, &p.value, false, p.name.span, p.ty.as_ref()),
                MemberKind::Let(l) => (&l.name.name, &l.value, true, l.span, l.ty.as_ref()),
                _ => continue,
            };
            let when = m
                .attrs
                .iter()
                .chain(m.trailing.iter())
                .find(|a| a.name.name == "when")
                .and_then(|a| match a.args.first() {
                    Some(AttrArg::Expr(e)) => Some(e),
                    _ => None,
                });
            let key = self.key(name);
            if m.private {
                out.private.insert(key.clone());
            }
            defs.insert(
                key,
                Def {
                    expr,
                    ty,
                    is_let,
                    ctx: callee_ctx.clone(),
                    span,
                    inst: Some(info.clone()),
                    fixed: None,
                    when,
                    role: crate::attrs::explicit_role(&m.attrs, &m.trailing),
                },
            );
        }
        out.list.push(Instance {
            info,
            component: self.callee,
            callee_ctx,
            span: self.span,
        });
    }

    /// A given argument whose value needs the parameter's type: a bare variant or an `ots` name.
    fn fixed_value(&self, expr: &Expr, param: &ast::ParamDecl) -> Option<Val> {
        match self.resolver.resolve_ref_in_param_type(
            self.parent_ctx,
            expr,
            &param.ty,
            self.callee_path,
        ) {
            Ok(Resolved::Variant { enum_path, variant }) => {
                Some(Val::Variant { enum_path, variant })
            }
            Ok(Resolved::Item {
                path,
                class: ItemClass::Component,
            }) => Some(Val::Component(path)),
            _ => None,
        }
    }
}

/// The first supplied argument whose parameter is a `Pattern2` or `Pattern3` (stdlib.md 3: one joint per pattern).
// frob:ticket 01M4SWP814HCC088QRKJ7QGBNV
fn pattern_arg(callee: &Component, args: &[ast::Arg]) -> Option<String> {
    callee
        .params
        .iter()
        .filter(|p| {
            matches!(&p.ty.kind, TypeKind::Path(tp)
                if tp.segments.last().is_some_and(|s| s.name.starts_with("Pattern")))
        })
        .map(|p| p.name.name.clone())
        .find(|n| {
            args.iter()
                .any(|a| a.name.as_ref().is_some_and(|an| an.name == *n))
        })
}

fn is_part_type(ty: &ast::Type) -> bool {
    matches!(&ty.kind, TypeKind::Path(p) if p.segments.len() == 1 && p.segments[0].name == "Part")
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// Reports E006 for each `#![requires(arg, when = cond)]` whose condition holds with `arg` absent,
    /// and for each `#![forbids(arg, when = cond)]` whose condition holds with `arg` given.
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    pub(super) fn check_requires(&mut self) {
        let work: Vec<(Rc<InstInfo>, &'p Component, Ctx, Span)> = self
            .instances
            .list
            .iter()
            .map(|i| (i.info.clone(), i.component, i.callee_ctx.clone(), i.span))
            .collect();
        for (info, comp, ctx, span) in work {
            for attr in comp
                .inner_attrs
                .iter()
                .filter(|a| matches!(a.name.name.as_str(), "requires" | "forbids"))
            {
                let forbids = attr.name.name == "forbids";
                let [AttrArg::Expr(arg), AttrArg::Named { name, value }] = attr.args.as_slice()
                else {
                    continue;
                };
                let ExprKind::Path(p) = &arg.kind else {
                    continue;
                };
                if name.name != "when" || p.segments.len() != 1 {
                    continue;
                }
                let arg_name = &p.segments[0].name;
                let mut key = info.prefix.clone();
                key.members = vec![MemberSeg::Name(arg_name.clone())];
                let present = self.defs.contains_key(&key) || self.instances.is_nodef_arg(&key);
                if present != forbids {
                    continue;
                }
                let saved = self.cur_inst.replace(info.clone());
                let mut sc = Scope {
                    ctx: ctx.clone(),
                    locals: Vec::new(),
                };
                let r: R<Val> = self.eval(&mut sc, value);
                self.cur_inst = saved;
                match r {
                    Ok(Val::Bool(true)) => {
                        let verb = if forbids { "forbids" } else { "requires" };
                        let d = diag(
                            "E006",
                            span,
                            format!(
                                "`{}` {verb} `{arg_name}` when its `{verb}` condition holds",
                                comp.name.name
                            ),
                        );
                        if let Fail::Diag(d) = d {
                            self.diagnostics
                                .push(d.in_module(ModuleKey::of_path(&info.prefix)));
                        }
                    }
                    Ok(_) => {}
                    Err(Fail::Diag(d)) => {
                        self.diagnostics.push(d.in_module(ModuleKey::of_ctx(&ctx)))
                    }
                    Err(Fail::Silent) => {}
                }
            }
        }
    }
}

/// The `#![default(Iface = Impl)]` entries of one attribute layer (first entry per interface wins).
struct DefaultEntries<'a> {
    /// Interface name to the implementation as written, for `resolve_default`.
    names: BTreeMap<String, String>,
    /// Interface name to the implementation expression, for resolution.
    exprs: BTreeMap<String, &'a Expr>,
}

/// Collect the `default` attribute entries from `attrs` into a `DefaultEntries`.
fn default_entries(attrs: &[ast::Attr]) -> DefaultEntries<'_> {
    let mut out = DefaultEntries {
        names: BTreeMap::new(),
        exprs: BTreeMap::new(),
    };
    for g in attrs
        .iter()
        .filter(|a| a.name.name == "default")
        .flat_map(|a| a.args.iter())
    {
        if let AttrArg::Named { name, value } = g {
            if out.exprs.contains_key(&name.name) {
                continue;
            }
            let written = match &value.kind {
                ExprKind::Path(p) => p.text(),
                _ => String::from("<expr>"),
            };
            out.names.insert(name.name.clone(), written);
            out.exprs.insert(name.name.clone(), value);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::tests::{file, run};
    use super::super::{Evaluation, Val};
    use crate::resolve::{CanonPath, MemberSeg};
    use cadetta_ir::canon::PartSeg;
    use cadetta_units::compare::Verdict;
    use cadetta_units::{Magnitude, Rational};

    fn key(comp: &str, part: &str, member: &str) -> CanonPath {
        let mut p = CanonPath::item("p", &[], comp);
        p.parts.push(PartSeg {
            name: part.to_owned(),
            index: None,
        });
        p.members.push(MemberSeg::Name(member.to_owned()));
        p
    }

    fn mm(ev: &Evaluation, member: &str) -> Option<Magnitude> {
        match &ev.get(&key("Top", "j", member))?.value {
            Val::Quantity(q) => Some(q.nominal.clone()),
            _ => None,
        }
    }

    fn m(n: i64, d: i64) -> Option<Magnitude> {
        Some(Magnitude::new(Rational::new(n, d).expect("denominator"), 0))
    }

    const BASE: &str = "interface Pol { gap: Length }\n\
        ots Std: Pol { gap = 2mm }\n\
        ots Loose: Pol { gap = 5mm }\n\
        assembly Joint(a: Length, pol: Pol, extra?: Length, b: Length = 3mm) {\n\
          #![requires(extra, when = a > 10mm)]\n\
          sum = a + b\n\
          priv hidden = a * 2\n\
          #[when(given(extra))]\n\
          with_extra = extra + a\n\
          gapped = pol.gap + a\n\
        }\n";

    // frob:tests arguments_defaults_and_members_evaluate
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    #[test]
    fn arguments_defaults_and_members_evaluate() {
        let f = file(&format!(
            "{BASE}#![default(Pol = Std)]\nassembly Top {{ part j: Joint(a = 4mm) }}\n"
        ));
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(mm(&ev, "a"), m(1, 250));
        assert_eq!(mm(&ev, "b"), m(3, 1000));
        assert_eq!(mm(&ev, "sum"), m(7, 1000));
        assert_eq!(mm(&ev, "gapped"), m(3, 500));
        assert!(ev.instance_private.contains(&key("Top", "j", "hidden")));
    }

    // frob:tests optional_argument_and_when_follow_given
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    #[test]
    fn optional_argument_and_when_follow_given() {
        let src = |args: &str| {
            format!("{BASE}#![default(Pol = Std)]\nassembly Top {{ part j: Joint({args}) }}\n")
        };
        let absent = file(&src("a = 4mm"));
        let ev = run(&absent);
        assert!(ev.get(&key("Top", "j", "with_extra")).is_none());
        let given = file(&src("a = 4mm, extra = 1mm"));
        let ev = run(&given);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(mm(&ev, "with_extra"), m(1, 200));
    }

    // frob:tests requires_reports_e006_when_the_argument_is_missing
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    #[test]
    fn requires_reports_e006_when_the_argument_is_missing() {
        let f = file(&format!(
            "{BASE}#![default(Pol = Std)]\nassembly Top {{ part j: Joint(a = 20mm) }}\n"
        ));
        let ev = run(&f);
        assert!(
            ev.diagnostics.iter().any(|d| d.code == "E006"),
            "{:?}",
            ev.diagnostics
        );
    }

    // frob:tests absent_member_read_is_e002
    // frob:ticket 01M4VSSD47V9VAAPW8B34630AM
    #[test]
    fn absent_member_read_is_e002() {
        let f = file(&format!(
            "{BASE}#![default(Pol = Std)]\nassembly Top {{\n  part j: Joint(a = 4mm)\n  \
             bad = j.with_extra\n}}\n"
        ));
        let ev = run(&f);
        assert!(
            ev.diagnostics
                .iter()
                .any(|d| d.code == "E002" && d.message.contains("absent")),
            "{:?}",
            ev.diagnostics
        );
    }

    // frob:tests requires_is_silent_when_the_argument_is_given
    // frob:ticket 01M4VSSDCJ82KNXA8MGEVK71FT
    #[test]
    fn requires_is_silent_when_the_argument_is_given() {
        let f = file(&format!(
            "{BASE}#![default(Pol = Std)]\nassembly Top {{ part j: Joint(a = 20mm, extra = 1mm) }}\n"
        ));
        let ev = run(&f);
        assert!(
            ev.diagnostics.iter().all(|d| d.code != "E006"),
            "{:?}",
            ev.diagnostics
        );
    }

    /// Evaluate `top` (package `p`) with `defaults` as its `[defaults]`, beside dependency `q` holding `dep`.
    fn run_with_defaults(top: &str, dep: &str, defaults: &[(&str, &str)]) -> Evaluation {
        use super::super::Evaluator;
        use crate::resolve::{ModuleSrc, PackageSrc, Resolver};
        use cadetta_units::UnitTable;
        let (top, dep) = (file(top), file(dep));
        let module = |f| {
            vec![ModuleSrc {
                path: Vec::new(),
                file: f,
                enums: &[],
            }]
        };
        let pkgs = vec![
            PackageSrc {
                name: "p".to_owned(),
                deps: vec!["q".to_owned()],
                modules: module(&top),
                defaults: defaults
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
            },
            PackageSrc {
                name: "q".to_owned(),
                deps: Vec::new(),
                modules: module(&dep),
                defaults: Default::default(),
            },
        ];
        let (resolver, errs) = Resolver::new(pkgs.clone());
        assert!(errs.is_empty(), "{errs:?}");
        let units = UnitTable::builtin();
        Evaluator::new(&resolver, &units, &pkgs).evaluate()
    }

    const DEP: &str = "pub interface Pol { gap: Length }\n\
        pub ots Std: Pol { gap = 2mm }\n\
        pub ots Loose: Pol { gap = 5mm }\n";

    const TOP: &str = "use q::{Pol, Std, Loose}\n\
        assembly Joint(a: Length, pol: Pol) { gapped = pol.gap + a }\n";

    // frob:tests project_defaults_name_a_dependency_implementation
    // frob:ticket 01M4W5J6FACWMX38X4FTGY21E7
    #[test]
    fn project_defaults_name_a_dependency_implementation() {
        let top = format!("{TOP}assembly Top {{ part j: Joint(a = 4mm) }}\n");
        let ev = run_with_defaults(&top, DEP, &[("Pol", "q::Loose")]);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(mm(&ev, "gapped"), m(9, 1000));
    }

    // frob:tests file_default_beats_project_default
    // frob:ticket 01M4W5J6FACWMX38X4FTGY21E7
    #[test]
    fn file_default_beats_project_default() {
        let top =
            format!("{TOP}#![default(Pol = Std)]\nassembly Top {{ part j: Joint(a = 4mm) }}\n");
        let ev = run_with_defaults(&top, DEP, &[("Pol", "q::Loose")]);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(mm(&ev, "gapped"), m(3, 500));
    }

    // frob:tests assembly_default_beats_file_default
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    #[test]
    fn assembly_default_beats_file_default() {
        let f = file(&format!(
            "{BASE}#![default(Pol = Std)]\nassembly Top {{\n  #![default(Pol = Loose)]\n  \
             part j: Joint(a = 4mm)\n}}\n"
        ));
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(mm(&ev, "gapped"), m(9, 1000));
    }

    // frob:tests crates/cadetta-sema/src/instances.rs::default_impl kind=unit
    // frob:ticket 01M4WGAD346CJ141PB5JK240ZV
    #[test]
    fn default_impl_delegates_file_layer_and_reports_missing() {
        let f = file(&format!(
            "{BASE}#![default(Pol = Loose)]\nassembly Top {{ part j: Joint(a = 4mm) }}\n"
        ));
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(mm(&ev, "gapped"), m(9, 1000));
        let none = file(&format!(
            "{BASE}assembly Top {{ part j: Joint(a = 4mm) }}\n"
        ));
        let ev = run(&none);
        assert!(!ev.diagnostics.is_empty());
    }

    // frob:tests missing_required_argument_is_e006
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    #[test]
    fn missing_required_argument_is_e006() {
        let f = file(&format!(
            "{BASE}#![default(Pol = Std)]\nassembly Top {{ part j: Joint(b = 1mm) }}\n"
        ));
        let ev = run(&f);
        assert!(
            ev.diagnostics.iter().any(|d| d.code == "E006"),
            "{:?}",
            ev.diagnostics
        );
    }

    const CHECKED: &str = "assembly Joint(a: Length, extra?: Length) {\n\
          check small: a < 10mm \"a stays small\"\n\
          #[when(given(extra))]\n\
          check extra_ok: extra < a \"extra is below a\"\n\
        }\n";

    fn verdicts(ev: &Evaluation) -> Vec<(String, Verdict)> {
        ev.checks
            .iter()
            .map(|c| (c.path.to_string(), c.verdict))
            .collect()
    }

    // frob:tests instance_checks_report_under_the_instance_path
    // frob:ticket 01M4W5HM6SAY12P1J3R3G80Q7F
    #[test]
    fn instance_checks_report_under_the_instance_path() {
        let f = file(&format!(
            "{CHECKED}assembly Top {{ part j: Joint(a = 4mm); part k: Joint(a = 12mm) }}\n"
        ));
        let ev = run(&f);
        let got = verdicts(&ev);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got.contains(&("p::Top/j.small".to_owned(), Verdict::Holds)));
        assert!(got.contains(&("p::Top/k.small".to_owned(), Verdict::Violates)));
        assert!(ev.diagnostics.iter().any(|d| d.code == "C001"));
    }

    // frob:tests instance_check_when_gates_the_verdict
    // frob:ticket 01M4W5HM6SAY12P1J3R3G80Q7F
    #[test]
    fn instance_check_when_gates_the_verdict() {
        let f = file(&format!(
            "{CHECKED}assembly Top {{ part j: Joint(a = 4mm, extra = 1mm); part k: Joint(a = 4mm) }}\n"
        ));
        let ev = run(&f);
        let got = verdicts(&ev);
        assert!(
            got.contains(&("p::Top/j.extra_ok".to_owned(), Verdict::Holds)),
            "{got:?}"
        );
        assert!(
            !got.iter().any(|(p, _)| p == "p::Top/k.extra_ok"),
            "{got:?}"
        );
    }

    // frob:tests pattern_argument_on_a_part_array_is_e006
    // frob:ticket 01M4SWP814HCC088QRKJ7QGBNV
    #[test]
    fn pattern_argument_on_a_part_array_is_e006() {
        let src = |slot: &str| {
            format!(
                "assembly Joint(at: Pattern2) {{ sum = 1mm }}\n\
                 assembly Top {{ holes: [Point2 on origin; 2] = rect_pattern(2, 1, 30mm, 0mm)\n\
                 part {slot}: Joint(at = holes) }}\n"
            )
        };
        let ev = run(&file(&src("j[4]")));
        assert!(
            ev.diagnostics.iter().any(|d| d.code == "E006"),
            "{:?}",
            ev.diagnostics
        );
        let ev = run(&file(&src("j")));
        assert!(
            !ev.diagnostics.iter().any(|d| d.code == "E006"),
            "{:?}",
            ev.diagnostics
        );
    }

    // frob:tests forbids_reports_e006_when_the_argument_is_given
    // frob:ticket 01M4SWP814HCC088QRKJ7QGBNV
    #[test]
    fn forbids_reports_e006_when_the_argument_is_given() {
        let src = |args: &str| {
            format!(
                "assembly Joint(a: Length, count?: Count) {{\n\
                 #![forbids(count, when = a > 1mm)]\n sum = a }}\n\
                 assembly Top {{ part j: Joint({args}) }}\n"
            )
        };
        let ev = run(&file(&src("a = 4mm, count = 2")));
        assert!(
            ev.diagnostics.iter().any(|d| d.code == "E006"),
            "{:?}",
            ev.diagnostics
        );
        let ev = run(&file(&src("a = 4mm")));
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
    }
}
