//! Renames and moves across modules and packages, and the semver-major rule
//! (packages.md 5, lints.md N004).
//!
//! A `was` path is a full canonical path, so it names the old module, file
//! or package of a moved item. [`Moves`] indexes the renames collected by
//! [`Renames`] so that anything keyed by the old path (bindings, waivers,
//! attestations, obligation records) can be carried to the current path, and
//! a use of the old path still resolves, with N004. [`ApiSurface`] captures a
//! library's interface so [`semver_changes`] can list what a new version
//! changes that packages.md 5 calls a major change.
//!
//! Decisions written into packages.md 5: a rename chain (`a` was `b`, `b` was
//! `c`) is followed to its end with a cycle guard; a carried path keeps the
//! suffix below the renamed prefix (`old.hub_bore` -> `new.hub_bore`); a glob
//! carries when its literal prefix (up to the first wildcard) names an old
//! path; the semver comparison treats a removed `pub` item or non-`priv`
//! member as covered when a `was` maps it onto a member that exists in the
//! new version; cadetta lists the changes but does not compare version
//! numbers.

// frob:ticket 01M4SXS3P4EGP92DE0KZ81R7HK

use std::collections::BTreeMap;
use std::fmt;

use cadetta_syntax::ast::{Expr, ItemKind, MemberKind, Type, Vis};
use tracing::{debug, trace};

use crate::deps::{DepDiagnostic, Renames};
use crate::resolve::{CanonPath, Ctx, MemberSeg, PackageSrc, ResolveError, Resolved, Resolver};
use crate::visibility::{item_name, member_name};

/// Longest rename chain followed before it is taken for a cycle.
const MAX_CHAIN: usize = 64;

/// The renames of a project, indexed for carrying old paths to current ones.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Moves {
    map: BTreeMap<CanonPath, CanonPath>,
}

impl Moves {
    /// Indexes the renames collected from the project's `was` clauses.
    pub fn new(renames: &Renames) -> Self {
        let map = renames
            .entries()
            .map(|(o, n)| (o.clone(), n.clone()))
            .collect::<BTreeMap<_, _>>();
        debug!(moves = map.len(), "indexed moves");
        Self { map }
    }

    /// Number of recorded moves.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True when nothing has moved.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The current path of `path`, following a rename chain; `None` when `path` has not moved.
    pub fn current(&self, path: &CanonPath) -> Option<CanonPath> {
        let mut cur = path.clone();
        let mut moved = false;
        for _ in 0..MAX_CHAIN {
            let Some(next) = self.step(&cur) else {
                return moved.then_some(cur);
            };
            if next == cur {
                return moved.then_some(cur);
            }
            cur = next;
            moved = true;
        }
        debug!(%path, "rename chain too long; stopping");
        moved.then_some(cur)
    }

    /// One redirect: the recorded old path that prefixes `path`, longest first.
    fn step(&self, path: &CanonPath) -> Option<CanonPath> {
        let (key, new) = self
            .map
            .iter()
            .filter(|(k, _)| {
                k.item_only() == path.item_only()
                    && k.parts == path.parts
                    && path.members.starts_with(&k.members)
            })
            .max_by_key(|(k, _)| k.members.len())?;
        let mut out = new.clone();
        out.members
            .extend(path.members[key.members.len()..].iter().cloned());
        Some(out)
    }

    /// Re-keys `records` (bindings, waivers, attestations) from old paths to current ones.
    pub fn carry_over<V>(&self, records: BTreeMap<CanonPath, V>) -> BTreeMap<CanonPath, V> {
        let mut out = BTreeMap::new();
        for (path, value) in records {
            let key = match self.current(&path) {
                Some(new) => {
                    debug!(old = %path, %new, "record carried over");
                    new
                }
                None => path,
            };
            out.insert(key, value);
        }
        out
    }

    /// Rewrites a waiver or attestation glob whose literal prefix names an old path.
    pub fn carry_glob(&self, glob: &str) -> String {
        let cut = glob.find(['*', '?', '[', '{']).unwrap_or(glob.len());
        let (literal, wild) = glob.split_at(cut);
        let Ok(path) = CanonPath::parse(literal) else {
            trace!(glob, "glob prefix is not a canonical path; kept");
            return glob.to_owned();
        };
        match self.current(&path) {
            Some(new) => {
                debug!(old = glob, %new, "glob carried over");
                format!("{new}{wild}")
            }
            None => glob.to_owned(),
        }
    }

    /// Resolves a reference; one that names an old path resolves to the
    /// current path and carries N004 (also across modules and packages).
    pub fn resolve_ref(
        &self,
        renames: &Renames,
        resolver: &Resolver<'_>,
        ctx: &Ctx,
        expr: &Expr,
        expected: Option<&Type>,
    ) -> Result<(Resolved, Option<DepDiagnostic>), ResolveError> {
        renames.resolve_ref(resolver, ctx, expr, expected)
    }
}

/// The kind of one exported thing, as far as semver cares.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ApiKind(pub String);

/// What a library exposes: `pub` items, non-`priv` members, and the values dependents read.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ApiSurface {
    /// `pub` items by path, with their kind.
    pub items: BTreeMap<CanonPath, ApiKind>,
    /// Non-`priv` members of `pub` items, with their kind.
    pub members: BTreeMap<CanonPath, ApiKind>,
    /// Digests of values a dependent's verdict reads, supplied from evaluation.
    pub values: BTreeMap<CanonPath, String>,
}

impl ApiSurface {
    /// Captures the exported items and members of one package.
    pub fn of(package: &PackageSrc<'_>) -> Self {
        let mut out = Self::default();
        for m in &package.modules {
            for it in m.file.items() {
                if it.vis != Vis::Pub {
                    continue;
                }
                let Some(name) = item_name(it) else { continue };
                let id = CanonPath::item(&package.name, &m.path, name);
                out.items.insert(id.clone(), ApiKind(item_kind(&it.kind)));
                if let ItemKind::Component(c) = &it.kind {
                    for mem in c.members.iter().filter(|x| !x.private) {
                        if let Some(n) = member_name(mem) {
                            let mut path = id.clone();
                            path.members = vec![MemberSeg::Name(n.to_owned())];
                            out.members.insert(path, ApiKind(member_kind(&mem.kind)));
                        }
                    }
                }
            }
        }
        debug!(
            package = %package.name,
            items = out.items.len(),
            members = out.members.len(),
            "captured api surface"
        );
        out
    }

    /// Records the digest of a value dependents read (from evaluation).
    pub fn with_value(mut self, path: CanonPath, digest: impl Into<String>) -> Self {
        self.values.insert(path, digest.into());
        self
    }
}

fn item_kind(k: &ItemKind) -> String {
    match k {
        ItemKind::Component(c) => format!("{:?}", c.class).to_lowercase(),
        ItemKind::Interface(_) => "interface".to_owned(),
        ItemKind::Cite(_) => "cite".to_owned(),
        ItemKind::Enum(_) => "enum".to_owned(),
        ItemKind::Fn(_) => "fn".to_owned(),
        _ => "other".to_owned(),
    }
}

fn member_kind(k: &MemberKind) -> String {
    match k {
        MemberKind::Param(_) => "param",
        MemberKind::Let(_) => "let",
        MemberKind::Part(_) => "part",
        MemberKind::Fit(_) => "fit",
        MemberKind::Check(_) => "check",
        MemberKind::Datum(_) => "datum",
        _ => "other",
    }
    .to_owned()
}

/// One change between two versions of a library that packages.md 5 makes a semver-major change.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SemverChange {
    /// A `pub` item or non-`priv` member was removed or renamed without `was`.
    Removed(CanonPath),
    /// The kind of an item or member changed.
    KindChanged {
        /// The path.
        path: CanonPath,
        /// Its old kind.
        from: String,
        /// Its new kind.
        to: String,
    },
    /// A value a dependent's verdict reads changed.
    ValueChanged(CanonPath),
}

impl fmt::Display for SemverChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Removed(p) => write!(f, "`{p}` was removed or renamed without `was`"),
            Self::KindChanged { path, from, to } => {
                write!(f, "`{path}` changed kind from {from} to {to}")
            }
            Self::ValueChanged(p) => write!(f, "value of `{p}` changed"),
        }
    }
}

/// Lists the semver-major changes from `old` to `new`, given the `was` moves of the new version.
pub fn semver_changes(old: &ApiSurface, new: &ApiSurface, moves: &Moves) -> Vec<SemverChange> {
    let mut out = Vec::new();
    for (name, o, n) in [
        ("item", &old.items, &new.items),
        ("member", &old.members, &new.members),
    ] {
        for (path, kind) in o {
            let target = moves.current(path).unwrap_or_else(|| path.clone());
            match n.get(&target) {
                None => {
                    debug!(%path, what = name, "removed without was");
                    out.push(SemverChange::Removed(path.clone()));
                }
                Some(k) if k != kind => out.push(SemverChange::KindChanged {
                    path: path.clone(),
                    from: kind.0.clone(),
                    to: k.0.clone(),
                }),
                Some(_) => {}
            }
        }
    }
    for (path, digest) in &old.values {
        let target = moves.current(path).unwrap_or_else(|| path.clone());
        if new.values.get(&target).is_some_and(|d| d != digest) {
            out.push(SemverChange::ValueChanged(path.clone()));
        }
    }
    debug!(changes = out.len(), "semver comparison done");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::ModuleSrc;
    use cadetta_syntax::ast::File;
    use cadetta_syntax::parser::parse;

    fn file(src: &str) -> File {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        p.file
    }

    fn pkg<'a>(name: &str, mods: &[(&[&str], &'a File)]) -> PackageSrc<'a> {
        PackageSrc {
            defaults: Default::default(),
            name: name.to_owned(),
            deps: Vec::new(),
            modules: mods
                .iter()
                .map(|(p, f)| ModuleSrc {
                    path: p.iter().map(|s| (*s).to_owned()).collect(),
                    file: f,
                    enums: &[],
                })
                .collect(),
        }
    }

    fn cp(s: &str) -> CanonPath {
        CanonPath::parse(s).unwrap()
    }

    fn moves_of(pkgs: &[PackageSrc<'_>]) -> (Moves, Renames) {
        let (r, _) = Resolver::new(pkgs.to_vec());
        let renames = Renames::collect(&r, pkgs);
        (Moves::new(&renames), renames)
    }

    // frob:tests records_carry_over_to_the_moved_path
    #[test]
    fn records_carry_over_to_the_moved_path() {
        let f = file("model Bracket {\n  thickness = 2mm was q::old::Bracket.t\n}\n");
        let pkgs = vec![pkg("q", &[(&["frame"], &f)])];
        let (m, _) = moves_of(&pkgs);
        assert_eq!(m.len(), 1);
        let mut recs = BTreeMap::new();
        recs.insert(cp("q::old::Bracket.t"), "binding");
        recs.insert(cp("q::old::Bracket.t.sub"), "waiver");
        recs.insert(cp("q::other::X.y"), "kept");
        let out = m.carry_over(recs);
        assert_eq!(out[&cp("q::frame::Bracket.thickness")], "binding");
        assert_eq!(out[&cp("q::frame::Bracket.thickness.sub")], "waiver");
        assert_eq!(out[&cp("q::other::X.y")], "kept");
        assert_eq!(
            m.carry_glob("q::old::Bracket.t*"),
            "q::frame::Bracket.thickness*"
        );
        assert_eq!(m.carry_glob("q::z::*"), "q::z::*");
    }

    // frob:tests move_across_packages_and_chains_carry
    #[test]
    fn move_across_packages_and_chains_carry() {
        let a = file("model B {\n  t = 1mm was p::m::B.t\n}\n");
        let b = file("model C {\n  t = 1mm was q::frame::B.t\n}\n");
        let pkgs = vec![pkg("q", &[(&["frame"], &a)]), pkg("r", &[(&[], &b)])];
        let (m, _) = moves_of(&pkgs);
        assert_eq!(m.current(&cp("p::m::B.t")), Some(cp("r::C.t")));
        assert_eq!(m.current(&cp("q::frame::B.t")), Some(cp("r::C.t")));
        assert_eq!(m.current(&cp("zz::Q.t")), None);
    }

    // frob:tests old_path_use_resolves_with_n004
    #[test]
    fn old_path_use_resolves_with_n004() {
        let moved = file("model Bracket {\n  t = 2mm was q::old::Bracket.thickness\n}\n");
        let user = file("model U {\n  v = q::old::Bracket.thickness\n}\n");
        let pkgs = vec![pkg("q", &[(&["frame"], &moved), (&["use_it"], &user)])];
        let (m, renames) = moves_of(&pkgs);
        let (r, _) = Resolver::new(pkgs.clone());
        let comp = user.components().next().unwrap();
        let MemberKind::Param(p) = &comp.members[0].kind else {
            panic!("param expected")
        };
        let ctx = Ctx {
            package: "q".to_owned(),
            module: vec!["use_it".to_owned()],
            component: Some("U".to_owned()),
            locals: Vec::new(),
        };
        let (res, diag) = m.resolve_ref(&renames, &r, &ctx, &p.value, None).unwrap();
        assert_eq!(diag.unwrap().code, "N004");
        let Resolved::Member { path, .. } = res else {
            panic!("member expected")
        };
        assert_eq!(path, cp("q::frame::Bracket.t"));
    }

    // frob:tests semver_major_changes_are_reported
    #[test]
    fn semver_major_changes_are_reported() {
        let v1 = file(
            "pub model A {\n  x = 1mm\n  y = 2mm\n  z = 3mm\n  priv h = 4mm\n}\npub model B {\n  k = 1mm\n}\n",
        );
        let v2 = file(
            "pub model A {\n  x2 = 1mm was q::A.x\n  z = 3mm\n}\npub assembly B {\n  k = 1mm\n}\n",
        );
        let p1 = [pkg("q", &[(&[], &v1)])];
        let p2 = vec![pkg("q", &[(&[], &v2)])];
        let (m, _) = moves_of(&p2);
        let old = ApiSurface::of(&p1[0]).with_value(cp("q::A.z"), "d1");
        let new = ApiSurface::of(&p2[0]).with_value(cp("q::A.z"), "d2");
        let ch = semver_changes(&old, &new, &m);
        assert!(ch.contains(&SemverChange::Removed(cp("q::A.y"))), "{ch:?}");
        assert!(!ch.contains(&SemverChange::Removed(cp("q::A.x"))), "{ch:?}");
        assert!(!ch.contains(&SemverChange::Removed(cp("q::A.h"))), "{ch:?}");
        assert!(ch.contains(&SemverChange::ValueChanged(cp("q::A.z"))));
        assert!(
            ch.iter().any(
                |c| matches!(c, SemverChange::KindChanged { path, .. } if *path == cp("q::B"))
            )
        );
        assert_eq!(ch.len(), 3, "{ch:?}");
        assert!(semver_changes(&old, &old, &Moves::default()).is_empty());
    }
}
