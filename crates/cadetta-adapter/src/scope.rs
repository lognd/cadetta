//! Adapter identity and scope (adapters.md 3.2).
//!
//! An `[[adapter]]` has an `id` (required when `name` repeats, else defaults
//! to `name`) and optional `modules` / `paths` filters that restrict which
//! canonical paths reach the adapter for emit, read and sync. Per-run
//! `--module` and path-glob flags narrow further via [`Scope::and`].

use std::collections::BTreeSet;
use std::fmt;

/// Configuration error raised while resolving adapter ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeError {
    /// A name repeats and at least one of its blocks has no `id`.
    MissingId {
        /// The repeated adapter name.
        name: String,
    },
    /// Two adapters resolve to the same id.
    DuplicateId {
        /// The clashing id.
        id: String,
    },
}

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingId { name } => {
                write!(
                    f,
                    "adapter `{name}` repeats; every block needs an explicit `id`"
                )
            }
            Self::DuplicateId { id } => write!(f, "adapter id `{id}` is used twice"),
        }
    }
}

impl std::error::Error for ScopeError {}

/// Resolves instance ids for `(name, id)` pairs in file order.
///
/// `id` defaults to `name`; a repeated name without an explicit `id` is a
/// configuration error (config-cli.md 2).
pub fn resolve_ids(adapters: &[(&str, Option<&str>)]) -> Result<Vec<String>, ScopeError> {
    let mut ids = Vec::with_capacity(adapters.len());
    let mut seen = BTreeSet::new();
    for (name, id) in adapters {
        let repeats = adapters.iter().filter(|(n, _)| n == name).count() > 1;
        if repeats && id.is_none() {
            tracing::error!(name, "repeated adapter name without id");
            return Err(ScopeError::MissingId {
                name: (*name).to_string(),
            });
        }
        let resolved = id.unwrap_or(name).to_string();
        if !seen.insert(resolved.clone()) {
            tracing::error!(id = %resolved, "duplicate adapter id");
            return Err(ScopeError::DuplicateId { id: resolved });
        }
        ids.push(resolved);
    }
    tracing::debug!(count = ids.len(), "adapter ids resolved");
    Ok(ids)
}

/// Which canonical paths an adapter sees; empty lists mean "all".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    modules: Vec<String>,
    paths: Vec<String>,
}

impl Scope {
    /// Builds a scope from module prefixes and canonical-path globs.
    pub fn new(modules: Vec<String>, paths: Vec<String>) -> Self {
        Self { modules, paths }
    }

    /// Reports whether `canonical` (`module::Item.member`) is in scope.
    pub fn accepts(&self, canonical: &str) -> bool {
        let module_ok = self.modules.is_empty()
            || self
                .modules
                .iter()
                .any(|m| module_prefix_matches(m, canonical));
        let path_ok =
            self.paths.is_empty() || self.paths.iter().any(|g| glob_matches(g, canonical));
        let ok = module_ok && path_ok;
        tracing::trace!(canonical, ok, "scope check");
        ok
    }

    /// Filters `paths` down to the in-scope ones, keeping order.
    pub fn filter<'a>(&self, paths: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        paths.into_iter().filter(|p| self.accepts(p)).collect()
    }
}

/// A run-level narrowing: a path must pass the adapter scope and the run scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Narrowed<'a> {
    adapter: &'a Scope,
    run: Scope,
}

impl Scope {
    /// Narrows this scope with per-run `--module` / glob flags.
    pub fn and(&self, run: Scope) -> Narrowed<'_> {
        Narrowed { adapter: self, run }
    }
}

impl Narrowed<'_> {
    /// Reports whether `canonical` passes both scopes.
    pub fn accepts(&self, canonical: &str) -> bool {
        self.adapter.accepts(canonical) && self.run.accepts(canonical)
    }
}

/// A module prefix matches on a segment boundary (`.` or `::`) of the path.
fn module_prefix_matches(prefix: &str, canonical: &str) -> bool {
    match canonical.strip_prefix(prefix) {
        Some("") => true,
        Some(rest) => rest.starts_with('.') || rest.starts_with("::"),
        None => false,
    }
}

/// Glob match: `*` matches any run without `:`; `**` matches any run.
pub fn glob_matches(glob: &str, text: &str) -> bool {
    fn go(g: &[u8], t: &[u8]) -> bool {
        match g.split_first() {
            None => t.is_empty(),
            Some((b'*', rest)) => {
                let (deep, rest) = match rest.split_first() {
                    Some((b'*', r)) => (true, r),
                    _ => (false, rest),
                };
                (0..=t.len()).any(|i| (deep || !t[..i].contains(&b':')) && go(rest, &t[i..]))
            }
            Some((c, rest)) => t.first() == Some(c) && go(rest, &t[1..]),
        }
    }
    go(glob.as_bytes(), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn repeated_name_without_id_is_error() {
        let r = resolve_ids(&[("solidworks", None), ("solidworks", Some("b"))]);
        assert_eq!(
            r,
            Err(ScopeError::MissingId {
                name: "solidworks".into()
            })
        );
    }

    #[test]
    fn id_defaults_to_name() {
        let r = resolve_ids(&[
            ("freecad", None),
            ("sw", Some("sw-a")),
            ("sw", Some("sw-b")),
        ]);
        assert_eq!(r.unwrap(), vec!["freecad", "sw-a", "sw-b"]);
    }

    #[test]
    fn duplicate_explicit_id_is_error() {
        let r = resolve_ids(&[("a", Some("x")), ("b", Some("x"))]);
        assert_eq!(r, Err(ScopeError::DuplicateId { id: "x".into() }));
    }

    #[test]
    fn modules_and_paths_restrict() {
        let sc = Scope::new(s(&["acme.gearbox"]), s(&["**.thickness"]));
        assert!(sc.accepts("acme.gearbox::CoverPlate.thickness"));
        assert!(!sc.accepts("acme.gearbox::CoverPlate.width"));
        assert!(!sc.accepts("acme.gearboxes::CoverPlate.thickness"));
        assert!(!sc.accepts("acme.other::CoverPlate.thickness"));
        assert!(Scope::default().accepts("anything::At.all"));
    }

    #[test]
    fn single_star_stays_in_segment() {
        assert!(glob_matches(
            "acme.gearbox::*.thickness",
            "acme.gearbox::Cover.thickness"
        ));
        assert!(!glob_matches(
            "*.thickness",
            "acme.gearbox::Cover.thickness"
        ));
        assert!(glob_matches(
            "**.thickness",
            "acme.gearbox::Cover.thickness"
        ));
    }

    #[test]
    fn run_scope_narrows() {
        let ad = Scope::new(s(&["acme"]), vec![]);
        let n = ad.and(Scope::new(s(&["acme.gearbox"]), vec![]));
        assert!(n.accepts("acme.gearbox::A.x"));
        assert!(!n.accepts("acme.shop::A.x"));
        assert_eq!(ad.filter(["acme.a::X.y", "zed::X.y"]), vec!["acme.a::X.y"]);
    }
}
