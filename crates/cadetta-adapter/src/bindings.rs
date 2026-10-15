//! Bindings files, multi-locator bindings and name mangling (adapters.md 3, 3.1).
//!
//! `bindings/<id>.toml` maps canonical paths to native locators, one table per
//! path with a `locators` array. `#[native(adapter, "locator")]` pins one inline
//! and overrides the file entry. Names the adapter creates itself are derived
//! purely from the path and prefix by [`mangle`] / [`mangle_all`].
// frob:ticket 01M4SPWRYRBTGD67WTG7D60E59

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use cadetta_ir::model::{
    Binding, Implicit, Locator, LocatorKind, LocatorMode, MemberKind, Project, Sign,
    TolerancePolicy,
};
use cadetta_ir::path::{Index, Path, PathContext, Seg};
use serde::Deserialize;

use crate::{AdapterDiagnostic, Severity};

/// Diagnostic code for a binding that matches no path (adapters.md 3).
pub const R005: &str = "R005";
/// Diagnostic code for two emitted paths creating one native name (adapters.md 3.1).
pub const R013: &str = "R013";
/// Diagnostic code for a bound `Text` parameter an equations file must emit but cannot express (emitters/sw-equations.md 1).
pub const R014: &str = "R014";
/// Diagnostic code for a locator a `json`/`csv` file cannot carry (emitters/json.md 1, emitters/csv.md 1).
// frob:ticket 01M4V7WCM9TBRWY007WWS81S98
pub const R015: &str = "R015";
/// Diagnostic code for an unbound native item, including a non-bare-reference dimension (adapters.md 3.5).
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
pub const R002: &str = "R002";

/// A failure loading or building bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingsError {
    /// The file is not valid TOML of the bindings shape.
    Toml(String),
    /// A table key is not a valid binding path.
    BadPath {
        /// The offending key.
        key: String,
        /// Why it failed.
        reason: String,
    },
    /// A path table has no locators.
    NoLocators {
        /// The path key.
        path: String,
    },
    /// `sign` was neither 1 nor -1.
    BadSign {
        /// The path key.
        path: String,
        /// The offending value.
        value: i64,
    },
    /// A locator failed field validation.
    BadLocator {
        /// The path key.
        path: String,
        /// Why it failed.
        reason: String,
    },
    /// A locator with the same identity but other fields is already bound (bind refuses to guess).
    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    Conflict {
        /// The path key.
        path: String,
        /// The native locator text.
        locator: String,
    },
    /// Two paths create one native name (R013).
    NameCollision {
        /// The colliding name.
        name: String,
        /// The paths that create it.
        paths: Vec<String>,
    },
}

impl fmt::Display for BindingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(e) => write!(f, "bindings file: {e}"),
            Self::BadPath { key, reason } => write!(f, "binding path `{key}`: {reason}"),
            Self::NoLocators { path } => write!(f, "binding `{path}` has no locators"),
            Self::BadSign { path, value } => {
                write!(f, "binding `{path}`: sign must be 1 or -1, got {value}")
            }
            Self::BadLocator { path, reason } => write!(f, "binding `{path}`: {reason}"),
            Self::Conflict { path, locator } => write!(
                f,
                "binding `{path}`: `{locator}` is already bound with different fields; remove it first"
            ),
            Self::NameCollision { name, paths } => {
                write!(
                    f,
                    "{R013}: `{name}` would be created by {}",
                    paths.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for BindingsError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTable {
    locators: Vec<RawLocator>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLocator {
    locator: String,
    kind: RawKind,
    mode: Option<RawMode>,
    native_unit: Option<String>,
    ref_to: Option<String>,
    #[serde(rename = "for")]
    for_alternate: Option<String>,
    config: Option<String>,
    sign: Option<i64>,
    component: Option<String>,
    tolerance: Option<RawTolerance>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum RawTolerance {
    Drive,
    Ignore,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum RawKind {
    Global,
    Dimension,
    Property,
    Frame,
    Instance,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum RawMode {
    Value,
    Ref,
}

/// The loaded bindings of one adapter, in deterministic (path-sorted) order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BindingSet {
    adapter: String,
    entries: Vec<Binding>,
}

/// Where a `ref`-mode dimension points (adapters.md 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefTarget {
    /// An existing native global (explicit `ref_to`, or the path's first global locator).
    Global(String),
    /// The global of the path's referent, for a pure alias.
    AliasOf(String),
    /// The emitter creates this mangled global for the path.
    OwnGlobal(String),
}

impl BindingSet {
    /// Parses `bindings/<adapter>.toml`; keeps kind, mode, unit and every Locator field.
    ///
    /// `mode` defaults to `ref` for a dimension and `value` otherwise; `native_unit`
    /// defaults to the empty string, meaning the adapter's declared native unit.
    pub fn load_str(adapter: &str, text: &str) -> Result<Self, BindingsError> {
        let raw: BTreeMap<String, RawTable> =
            toml::from_str(text).map_err(|e| BindingsError::Toml(e.to_string()))?;
        let mut entries = Vec::with_capacity(raw.len());
        for (key, table) in raw {
            let path = Path::parse(&key, PathContext::Binding).map_err(|e| {
                tracing::error!(key, "bindings: bad path");
                BindingsError::BadPath {
                    key: key.clone(),
                    reason: e.to_string(),
                }
            })?;
            if table.locators.is_empty() {
                return Err(BindingsError::NoLocators { path: key });
            }
            let mut locators = Vec::with_capacity(table.locators.len());
            for l in table.locators {
                locators.push(convert(&key, l)?);
            }
            tracing::debug!(adapter, path = %key, n = locators.len(), "bindings: loaded");
            entries.push(Binding {
                path,
                adapter: adapter.to_owned(),
                locators,
            });
        }
        Ok(Self {
            adapter: adapter.to_owned(),
            entries,
        })
    }

    /// The adapter id these bindings belong to.
    pub fn adapter(&self) -> &str {
        &self.adapter
    }

    /// All bindings, file entries and implicit locators alike, sorted by path text.
    pub fn entries(&self) -> &[Binding] {
        &self.entries
    }

    /// The bindings a bindings file would hold: explicit locators only, never an implicit one.
    // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
    pub fn file_entries(&self) -> Vec<Binding> {
        self.entries
            .iter()
            .filter_map(|b| {
                let locators: Vec<Locator> = b
                    .locators
                    .iter()
                    .filter(|l| l.implicit.is_none())
                    .cloned()
                    .collect();
                (!locators.is_empty()).then(|| Binding {
                    path: b.path.clone(),
                    adapter: b.adapter.clone(),
                    locators,
                })
            })
            .collect()
    }

    /// Adds one explicit locator for `path`; `Ok(false)` when an identical one is already there.
    ///
    /// Identity is (kind, locator, for, config, component). A locator with the same identity
    /// but other fields is [`BindingsError::Conflict`]. New locators go after the path's
    /// existing ones (file order resolves refs); entries stay sorted by path text.
    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    pub fn bind(&mut self, path: &Path, loc: Locator) -> Result<bool, BindingsError> {
        let identity = |l: &Locator| {
            (
                l.kind,
                l.locator.clone(),
                l.for_alternate.clone(),
                l.config.clone(),
                l.component.clone(),
            )
        };
        let idx = self.entries.iter().position(|e| &e.path == path);
        if let Some(i) = idx {
            let same_id = self.entries[i]
                .locators
                .iter()
                .filter(|l| l.implicit.is_none())
                .find(|l| identity(l) == identity(&loc));
            if let Some(existing) = same_id {
                if *existing == loc {
                    tracing::debug!(%path, locator = %loc.locator, "bindings: bind is a no-op");
                    return Ok(false);
                }
                return Err(BindingsError::Conflict {
                    path: path.to_string(),
                    locator: loc.locator,
                });
            }
        }
        let i = idx.unwrap_or_else(|| {
            self.entries.push(Binding {
                path: path.clone(),
                adapter: self.adapter.clone(),
                locators: Vec::new(),
            });
            self.entries.len() - 1
        });
        tracing::info!(%path, locator = %loc.locator, "bindings: explicit locator added");
        self.entries[i].locators.push(loc);
        self.entries.sort_by_key(|b| b.path.to_string());
        Ok(true)
    }

    /// Removes every explicit locator of `path` whose native text is `locator`; true if any went.
    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    pub fn unbind(&mut self, path: &Path, locator: &str) -> bool {
        let Some(i) = self.entries.iter().position(|e| &e.path == path) else {
            return false;
        };
        let before = self.entries[i].locators.len();
        self.entries[i]
            .locators
            .retain(|l| l.implicit.is_some() || l.locator != locator);
        let removed = self.entries[i].locators.len() != before;
        if self.entries[i].locators.is_empty() {
            self.entries.remove(i);
        }
        if removed {
            tracing::info!(%path, locator, "bindings: explicit locator removed");
        }
        removed
    }

    /// Renders the bindings file: explicit locators only, canonical path order, spec spelling.
    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    pub fn to_toml(&self) -> String {
        let quote = |s: &str| toml::Value::String(s.to_owned()).to_string();
        let mut entries = self.file_entries();
        entries.sort_by_key(|b| b.path.to_string());
        let mut out = String::new();
        for b in &entries {
            out.push_str(&format!("[{}]\nlocators = [\n", quote(&b.path.to_string())));
            for l in &b.locators {
                let mut f = format!(
                    "locator = {}, kind = {}, mode = {}",
                    quote(&l.locator),
                    quote(kind_text(l.kind)),
                    quote(mode_text(l.mode))
                );
                if !l.native_unit.is_empty() {
                    f.push_str(&format!(", native_unit = {}", quote(&l.native_unit)));
                }
                if let Some(r) = &l.ref_to {
                    f.push_str(&format!(", ref_to = {}", quote(r)));
                }
                if let Some(a) = &l.for_alternate {
                    f.push_str(&format!(", for = {}", quote(a)));
                }
                if let Some(c) = &l.config {
                    f.push_str(&format!(", config = {}", quote(c)));
                }
                if let Some(s) = l.sign {
                    f.push_str(&format!(
                        ", sign = {}",
                        if s == Sign::Minus { -1 } else { 1 }
                    ));
                }
                if let Some(c) = &l.component {
                    f.push_str(&format!(", component = {}", quote(c)));
                }
                // frob:ticket 01M4VPEE2SE4Y96XGGN17H42KT
                if let Some(t) = l.tolerance {
                    f.push_str(&format!(", tolerance = {}", quote(tolerance_text(t))));
                }
                out.push_str(&format!("  {{ {f} }},\n"));
            }
            out.push_str("]\n\n");
        }
        out.trim_end().to_owned() + if out.is_empty() { "" } else { "\n" }
    }

    /// Adds discovered or scope-implied locators (adapters.md 3.5); never touches the file.
    ///
    /// An explicit locator of the same kind and native name replaces the implicit one (the
    /// implicit one is dropped); a different native item adds to the path's locators.
    // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
    pub fn add_implicit(&mut self, found: Vec<Binding>) {
        for b in found {
            for loc in b.locators {
                debug_assert!(
                    loc.implicit.is_some(),
                    "add_implicit takes implicit locators"
                );
                let idx = match self.entries.iter().position(|e| e.path == b.path) {
                    Some(i) => i,
                    None => {
                        self.entries.push(Binding {
                            path: b.path.clone(),
                            adapter: self.adapter.clone(),
                            locators: Vec::new(),
                        });
                        self.entries.len() - 1
                    }
                };
                let entry = &mut self.entries[idx];
                let same = |l: &Locator| l.kind == loc.kind && l.locator == loc.locator;
                if entry.locators.iter().any(same) {
                    tracing::debug!(path = %b.path, locator = %loc.locator,
                        "bindings: implicit locator already present or replaced by explicit");
                    continue;
                }
                tracing::info!(path = %b.path, locator = %loc.locator, implicit = ?loc.implicit,
                    "bindings: implicit locator added");
                entry.locators.push(loc);
            }
        }
        self.entries.sort_by_key(|b| b.path.to_string());
    }

    /// `bind = "scope"`: gives every candidate its created global as an implicit `global` locator.
    ///
    /// Candidates the caller has already restricted to the adapter's own scope; this skips
    /// a `let`, a member of another package (`package`), a `Text` path when `can_text` is
    /// false, and any path already bound to an explicit global (it creates no name, 3.1).
    /// Two candidates creating one name are R013.
    // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
    pub fn bind_scope(
        &mut self,
        candidates: &[ScopeCandidate],
        package: &str,
        prefix: &str,
        can_text: bool,
    ) -> Result<usize, BindingsError> {
        let chosen: Vec<&ScopeCandidate> = candidates
            .iter()
            .filter(|c| {
                let ok = matches!(
                    c.kind,
                    MemberKind::Param | MemberKind::Datum | MemberKind::Place
                ) && c.path.package == package
                    && (can_text || !c.text)
                    && !self.has_explicit_global(&c.path);
                if !ok {
                    tracing::trace!(path = %c.path, "bindings: not scope-bound");
                }
                ok
            })
            .collect();
        let paths: Vec<Path> = chosen.iter().map(|c| c.path.clone()).collect();
        let names = mangle_all(&paths, prefix)?;
        let found: Vec<Binding> = names
            .into_iter()
            .map(|(path, name)| Binding {
                path,
                adapter: self.adapter.clone(),
                locators: vec![implicit_locator(
                    name,
                    LocatorKind::Global,
                    Implicit::Global,
                )],
            })
            .collect();
        let n = found.len();
        self.add_implicit(found);
        tracing::info!(adapter = %self.adapter, n, "bindings: scope binding applied");
        Ok(n)
    }

    fn has_explicit_global(&self, path: &Path) -> bool {
        self.entries.iter().any(|b| {
            path_matches(&b.path, path)
                && b.locators
                    .iter()
                    .any(|l| l.kind == LocatorKind::Global && l.implicit.is_none())
        })
    }

    /// The binding for exactly `path`, if any.
    pub fn get(&self, path: &Path) -> Option<&Binding> {
        self.entries.iter().find(|b| &b.path == path)
    }

    /// Applies `#[native(adapter, "locator")]` pins: a pin replaces the file entry for its path.
    ///
    /// Pins for other adapters are ignored; several pins for one path accumulate in order.
    pub fn apply_native(&mut self, pins: &[NativePin]) {
        let mut replaced: Vec<Path> = Vec::new();
        for pin in pins.iter().filter(|p| p.adapter == self.adapter) {
            let loc = Locator {
                locator: pin.locator.clone(),
                kind: LocatorKind::Global,
                mode: LocatorMode::Value,
                native_unit: String::new(),
                ref_to: None,
                for_alternate: None,
                config: None,
                sign: None,
                component: None,
                implicit: None,
                tolerance: None,
            };
            if let Some(b) = self.entries.iter_mut().find(|b| b.path == pin.path) {
                if replaced.contains(&pin.path) {
                    b.locators.push(loc);
                } else {
                    tracing::info!(path = %pin.path, "bindings: #[native] overrides file entry");
                    b.locators = vec![loc];
                    replaced.push(pin.path.clone());
                }
            } else {
                tracing::debug!(path = %pin.path, "bindings: #[native] adds entry");
                replaced.push(pin.path.clone());
                self.entries.push(Binding {
                    path: pin.path.clone(),
                    adapter: self.adapter.clone(),
                    locators: vec![loc],
                });
            }
        }
    }

    /// Carries bindings across `was` renames, given `(old, new)` pairs in order.
    ///
    /// A binding already present at `new` wins; the stale one at `old` is dropped.
    pub fn carry_forward(&mut self, renames: &[(Path, Path)]) {
        for (old, new) in renames {
            let Some(i) = self.entries.iter().position(|b| &b.path == old) else {
                continue;
            };
            let mut b = self.entries.remove(i);
            if self.entries.iter().any(|e| &e.path == new) {
                tracing::debug!(%old, %new, "bindings: rename target already bound");
            } else {
                tracing::info!(%old, %new, "bindings: carried forward across was");
                b.path = new.clone();
                self.entries.push(b);
            }
        }
    }

    /// R005 diagnostics for bindings whose path matches no `known` path (`[*]` matches any index).
    pub fn orphans<'a>(&self, known: impl IntoIterator<Item = &'a Path>) -> Vec<AdapterDiagnostic> {
        let known: Vec<&Path> = known.into_iter().collect();
        self.entries
            .iter()
            .filter(|b| b.locators.iter().any(|l| l.implicit.is_none()))
            .filter(|b| !known.iter().any(|k| path_matches(&b.path, k)))
            .map(|b| {
                tracing::warn!(path = %b.path, "bindings: orphan binding (R005)");
                AdapterDiagnostic {
                    code: R005.to_owned(),
                    severity: Severity::Warning,
                    message: format!("binding `{}` matches no path", b.path),
                }
            })
            .collect()
    }

    /// Whether a parameter is written on emit: it must be bound and not a `let` (analysis).
    pub fn emits(&self, path: &Path, analysis: bool) -> bool {
        if analysis {
            tracing::trace!(%path, "bindings: let is never emitted");
            return false;
        }
        let bound = self.entries.iter().any(|b| path_matches(&b.path, path));
        if !bound {
            tracing::trace!(%path, "bindings: unbound, not emitted");
        }
        bound
    }

    /// Resolves a `ref`-mode dimension locator of `path` (adapters.md 3).
    ///
    /// Order: its `ref_to`; the first global locator of the same path in file order;
    /// the global of `alias_of[path]` when that referent is bound; else the path's own
    /// mangled global (`own`).
    pub fn resolve_ref(
        &self,
        path: &Path,
        loc: &Locator,
        alias_of: &HashMap<Path, Path>,
        own: &str,
    ) -> RefTarget {
        if let Some(r) = &loc.ref_to {
            return RefTarget::Global(r.clone());
        }
        if let Some(g) = self.first_global(path) {
            return RefTarget::Global(g);
        }
        if let Some(g) = alias_of.get(path).and_then(|r| self.first_global(r)) {
            return RefTarget::AliasOf(g);
        }
        RefTarget::OwnGlobal(own.to_owned())
    }

    fn first_global(&self, path: &Path) -> Option<String> {
        self.get(path)?
            .locators
            .iter()
            .find(|l| l.kind == LocatorKind::Global)
            .map(|l| l.locator.clone())
    }
}

/// An inline `#[native(adapter, "locator")]` pin from source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePin {
    /// The pinned member.
    pub path: Path,
    /// The adapter id the pin targets.
    pub adapter: String,
    /// The native locator text.
    pub locator: String,
}

fn convert(key: &str, l: RawLocator) -> Result<Locator, BindingsError> {
    let kind = match l.kind {
        RawKind::Global => LocatorKind::Global,
        RawKind::Dimension => LocatorKind::Dimension,
        RawKind::Property => LocatorKind::Property,
        RawKind::Frame => LocatorKind::Frame,
        RawKind::Instance => LocatorKind::Instance,
    };
    let mode = match l.mode {
        Some(RawMode::Value) => LocatorMode::Value,
        Some(RawMode::Ref) => LocatorMode::Ref,
        None if kind == LocatorKind::Dimension => LocatorMode::Ref,
        None => LocatorMode::Value,
    };
    let sign = match l.sign {
        None => None,
        Some(1) => Some(Sign::Plus),
        Some(-1) => Some(Sign::Minus),
        Some(v) => {
            return Err(BindingsError::BadSign {
                path: key.to_owned(),
                value: v,
            });
        }
    };
    let loc = Locator {
        locator: l.locator,
        kind,
        mode,
        native_unit: l.native_unit.unwrap_or_default(),
        ref_to: l.ref_to,
        for_alternate: l.for_alternate,
        config: l.config,
        sign,
        component: l.component,
        implicit: None,
        tolerance: l.tolerance.map(|t| match t {
            RawTolerance::Drive => TolerancePolicy::Drive,
            RawTolerance::Ignore => TolerancePolicy::Ignore,
        }),
    };
    crate::locator::validate(&loc).map_err(|e| BindingsError::BadLocator {
        path: key.to_owned(),
        reason: e.to_string(),
    })?;
    Ok(loc)
}

/// The bindings-file spelling of a locator kind.
// frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
pub fn kind_text(k: LocatorKind) -> &'static str {
    match k {
        LocatorKind::Global => "global",
        LocatorKind::Dimension => "dimension",
        LocatorKind::Property => "property",
        LocatorKind::Frame => "frame",
        LocatorKind::Instance => "instance",
    }
}

/// The bindings-file spelling of a tolerance policy.
// frob:ticket 01M4VPEE2SE4Y96XGGN17H42KT
pub fn tolerance_text(t: TolerancePolicy) -> &'static str {
    match t {
        TolerancePolicy::Drive => "drive",
        TolerancePolicy::Ignore => "ignore",
    }
}

/// The bindings-file spelling of a locator mode.
// frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
pub fn mode_text(m: LocatorMode) -> &'static str {
    match m {
        LocatorMode::Value => "value",
        LocatorMode::Ref => "ref",
    }
}

/// The spelling of an implicit-binding reason.
// frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
pub fn implicit_text(i: Implicit) -> &'static str {
    match i {
        Implicit::Global => "global",
        Implicit::Link => "link",
        Implicit::Frame => "frame",
        Implicit::Reference => "reference",
    }
}

/// Whether binding path `b` (which may hold `[*]`) addresses `k`.
fn path_matches(b: &Path, k: &Path) -> bool {
    if b == k {
        return true;
    }
    let star = |i: &Option<Index>| matches!(i, Some(Index::Star));
    if b.package != k.package
        || b.modules != k.modules
        || b.item != k.item
        || b.steps.len() != k.steps.len()
        || b.members.len() != k.members.len()
    {
        return false;
    }
    let steps = b.steps.iter().zip(&k.steps).all(|(x, y)| {
        x.name == y.name && (x.index == y.index || (star(&x.index) && y.index.is_some()))
    });
    let members = b.members.iter().zip(&k.members).all(|(x, y)| match (x, y) {
        (Seg::Index(Index::Star), Seg::Index(_)) => true,
        _ => x == y,
    });
    steps && members
}

fn index_text(i: &Index) -> Option<String> {
    match i {
        Index::Int(n) => Some(n.to_string()),
        Index::Var(v) => Some(v.clone()),
        Index::Star => None,
    }
}

fn with_index(name: &str, idx: Option<&Index>) -> String {
    match idx.and_then(index_text) {
        Some(t) => format!("{name}_{t}"),
        None => name.to_owned(),
    }
}

/// The created native name for `path`: package and modules dropped, `.` and `/` become `__`,
/// `[i]` becomes `_i`, `[*]` is dropped; `prefix` is prepended (adapters.md 3.1).
///
/// A pure function of the path and the prefix; nothing else ever changes the result.
pub fn mangle(path: &Path, prefix: &str) -> String {
    let mut parts: Vec<String> = vec![path.item.clone()];
    for s in &path.steps {
        parts.push(with_index(&s.name, s.index.as_ref()));
    }
    for seg in &path.members {
        match seg {
            Seg::Name(n) => parts.push(n.clone()),
            Seg::Num(n) => parts.push(n.to_string()),
            Seg::Index(i) => {
                if let (Some(last), Some(t)) = (parts.last_mut(), index_text(i)) {
                    last.push('_');
                    last.push_str(&t);
                }
            }
        }
    }
    format!("{prefix}{}", parts.join("__"))
}

/// Mangles every path with [`mangle`]; two distinct paths sharing a name are R013.
///
/// Callers pass only the paths whose names the adapter creates: a path bound explicitly
/// (bindings file or `#[native]` pin) is left out, which resolves a collision. Nothing is
/// qualified automatically (adapters.md 3.1). Returns [`BindingsError::NameCollision`] on
/// a shared name; the mapping is injective on success.
pub fn mangle_all(paths: &[Path], prefix: &str) -> Result<Vec<(Path, String)>, BindingsError> {
    let mut uniq: Vec<&Path> = Vec::new();
    for p in paths {
        if !uniq.contains(&p) {
            uniq.push(p);
        }
    }
    let names: Vec<String> = uniq.iter().map(|p| mangle(p, prefix)).collect();
    let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, n) in names.iter().enumerate() {
        groups.entry(n.as_str()).or_default().push(i);
    }
    if let Some((name, idxs)) = groups.iter().find(|(_, v)| v.len() > 1) {
        tracing::error!(name, "bindings: name collision (R013)");
        return Err(BindingsError::NameCollision {
            name: (*name).to_owned(),
            paths: idxs.iter().map(|&j| uniq[j].to_string()).collect(),
        });
    }
    tracing::debug!(n = names.len(), "bindings: names mangled");
    Ok(uniq.into_iter().cloned().zip(names).collect())
}

/// One in-scope member offered to scope binding (`bind = "scope"`).
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeCandidate {
    /// The member path.
    pub path: Path,
    /// The member kind (`let` is `Let`, never bound).
    pub kind: MemberKind,
    /// True for a `Text` value.
    pub text: bool,
}

fn implicit_locator(name: String, kind: LocatorKind, why: Implicit) -> Locator {
    Locator {
        locator: name,
        kind,
        mode: LocatorMode::Value,
        native_unit: String::new(),
        ref_to: None,
        for_alternate: None,
        config: None,
        sign: None,
        component: None,
        implicit: Some(why),
        tolerance: None,
    }
}

/// One in-scope path offered to the forward table of created names (adapters.md 3.5).
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameSource {
    /// The current path.
    pub path: Path,
    /// Its `was` paths, as canonical text.
    pub was: Vec<String>,
    /// The member kind (a frame only binds a datum).
    pub kind: MemberKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableHit {
    path: Path,
    kind: MemberKind,
    was: bool,
}

/// Created native name to path, including `was` names; used by discovery, never inverted.
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ForwardTable {
    by_name: BTreeMap<String, Vec<TableHit>>,
}

impl ForwardTable {
    /// Builds the table from in-scope paths; two paths creating one name are R013.
    pub fn build(sources: &[NameSource], prefix: &str) -> Result<Self, BindingsError> {
        let paths: Vec<Path> = sources.iter().map(|s| s.path.clone()).collect();
        let mut table = Self::default();
        for (path, name) in mangle_all(&paths, prefix)? {
            let kind = sources
                .iter()
                .find(|s| s.path == path)
                .map_or(MemberKind::Param, |s| s.kind);
            table.by_name.entry(name).or_default().push(TableHit {
                path,
                kind,
                was: false,
            });
        }
        for src in sources {
            for w in &src.was {
                let Ok(old) = Path::parse(w, PathContext::General) else {
                    tracing::warn!(was = %w, path = %src.path, "forward table: unparsable was path");
                    continue;
                };
                let name = mangle(&old, prefix);
                let hits = table.by_name.entry(name.clone()).or_default();
                if hits.iter().any(|h| !h.was) {
                    tracing::debug!(name, "forward table: was name shadowed by a current name");
                    continue;
                }
                hits.push(TableHit {
                    path: src.path.clone(),
                    kind: src.kind,
                    was: true,
                });
            }
        }
        tracing::debug!(names = table.by_name.len(), "forward table built");
        Ok(table)
    }

    fn lookup(&self, name: &str) -> &[TableHit] {
        self.by_name.get(name).map_or(&[], Vec::as_slice)
    }
}

/// A part's placement node and the reference text instances use to name it (`bracket`, `feet[2]`).
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The part instance path relative to its assembly.
    pub reference: String,
    /// The placement node (`Mount/bracket.place`).
    pub node: Path,
}

/// What discovery found: implicit bindings and the diagnostics it raised.
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discovery {
    /// Implicit bindings, to feed [`BindingSet::add_implicit`].
    pub bindings: Vec<Binding>,
    /// R002 and R013 findings.
    pub diagnostics: Vec<AdapterDiagnostic>,
}

fn finding(code: &str, severity: Severity, message: String) -> AdapterDiagnostic {
    tracing::warn!(code, %message, "discovery finding");
    AdapterDiagnostic {
        code: code.to_owned(),
        severity,
        message,
    }
}

/// The global a native equation names when it is exactly one quoted reference (`"G"`).
fn bare_reference(expr: &str) -> Option<&str> {
    let inner = expr.trim().strip_prefix('"')?.strip_suffix('"')?;
    (!inner.is_empty() && !inner.contains('"')).then_some(inner)
}

/// Discovers the native links of adapters.md 3.5 in an observed IR.
///
/// Looks native names up in `table` (never parses one back into a path); `placements`
/// maps instance reference text to placement nodes. Pure over its inputs.
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
pub fn discover(observed: &Project, table: &ForwardTable, placements: &[Placement]) -> Discovery {
    let mut out = Discovery::default();
    let exprs: HashMap<&Path, &str> = observed
        .components
        .iter()
        .flat_map(|c| &c.members)
        .filter_map(|m| m.expr.as_deref().map(|e| (&m.path, e)))
        .collect();
    let mut add =
        |path: &Path, loc: Locator| match out.bindings.iter_mut().find(|b| &b.path == path) {
            Some(b) => b.locators.push(loc),
            None => out.bindings.push(Binding {
                path: path.clone(),
                adapter: String::new(),
                locators: vec![loc],
            }),
        };
    let mut diagnostics = Vec::new();

    // Globals: the current name binds; a was name binds only when the current name is absent.
    let natives: Vec<&Locator> = observed.bindings.iter().flat_map(|b| &b.locators).collect();
    let mut globals: BTreeMap<String, Path> = BTreeMap::new();
    for l in natives.iter().filter(|l| l.kind == LocatorKind::Global) {
        let hits = table.lookup(&l.locator);
        let Some(first) = hits.first() else { continue };
        if first.was {
            if hits.len() > 1 {
                diagnostics.push(finding(
                    R002,
                    Severity::Warning,
                    format!(
                        "native global `{}` is a was name of several paths",
                        l.locator
                    ),
                ));
                continue;
            }
            let current_present = natives.iter().any(|o| {
                o.kind == LocatorKind::Global
                    && table
                        .lookup(&o.locator)
                        .iter()
                        .any(|h| !h.was && h.path == first.path)
            });
            if current_present {
                diagnostics.push(finding(
                    R002,
                    Severity::Warning,
                    format!(
                        "native global `{}` is an old (was) name of `{}`, whose current name also exists",
                        l.locator, first.path
                    ),
                ));
                continue;
            }
        }
        globals.insert(l.locator.clone(), first.path.clone());
        add(
            &first.path,
            Locator {
                native_unit: l.native_unit.clone(),
                ..implicit_locator(l.locator.clone(), LocatorKind::Global, Implicit::Global)
            },
        );
    }

    // Dimensions: a link only when the equation is exactly a bare reference to such a global.
    for b in &observed.bindings {
        for l in b
            .locators
            .iter()
            .filter(|l| l.kind == LocatorKind::Dimension)
        {
            let Some(expr) = exprs.get(&b.path) else {
                continue;
            };
            let bare = bare_reference(expr);
            match bare.and_then(|g| globals.get(g).map(|p| (g, p))) {
                None if bare.is_some() => tracing::debug!(
                    dimension = %l.locator,
                    "bare reference to a name that is not a bound global; not a link, silent"
                ),
                Some((g, path)) => add(
                    path,
                    Locator {
                        mode: LocatorMode::Ref,
                        ref_to: Some(g.to_owned()),
                        native_unit: l.native_unit.clone(),
                        ..implicit_locator(l.locator.clone(), LocatorKind::Dimension, Implicit::Link)
                    },
                ),
                None => diagnostics.push(finding(
                    R002,
                    Severity::Warning,
                    format!(
                        "dimension `{}` is not a link: its equation is `{expr}`, not a bare reference to a bound global",
                        l.locator
                    ),
                )),
            }
        }
    }

    // Frames named by a datum's created name.
    for f in &observed.frames {
        for h in table
            .lookup(&f.name)
            .iter()
            .filter(|h| h.kind == MemberKind::Datum)
        {
            add(
                &h.path,
                implicit_locator(f.name.clone(), LocatorKind::Frame, Implicit::Frame),
            );
        }
    }

    // Instances named by reference property; two naming one part are R013.
    let mut by_ref: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for i in &observed.instances {
        if let Some(r) = i.reference.as_deref() {
            by_ref.entry(r).or_default().push(&i.component);
        }
    }
    for (r, names) in by_ref {
        let Some(pl) = placements.iter().find(|p| p.reference == r) else {
            continue;
        };
        if names.len() > 1 {
            diagnostics.push(finding(
                R013,
                Severity::Error,
                format!("instances {} all name part `{r}`", names.join(", ")),
            ));
            continue;
        }
        add(
            &pl.node,
            implicit_locator(
                names[0].to_owned(),
                LocatorKind::Instance,
                Implicit::Reference,
            ),
        );
    }

    out.diagnostics = diagnostics;
    tracing::info!(
        bindings = out.bindings.len(),
        diagnostics = out.diagnostics.len(),
        "discovery done"
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::Binding).expect("path")
    }

    const FILE: &str = r#"
["acme.gearbox::CoverPlate.thickness"]
locators = [
  { locator = "Plate_Thk", kind = "global", mode = "value", native_unit = "mm" },
  { locator = "plate thickness", kind = "global", mode = "value", native_unit = "mm" },
  { locator = "D1@Boss-Extrude1@CoverPlate.SLDPRT", kind = "dimension", ref_to = "Plate_Thk" },
]

["acme.rover::Rover/axle[*].end_diameter"]
locators = [
  { locator = "X", kind = "frame", sign = -1, for = "Colson", config = "Default", component = "wheel<2>" },
]
"#;

    // frob:tests bindings_multi_locator_fields
    #[test]
    fn bindings_multi_locator_fields() {
        let s = BindingSet::load_str("sw", FILE).expect("load");
        let b = s.get(&p("acme.gearbox::CoverPlate.thickness")).expect("b");
        assert_eq!(b.locators.len(), 3);
        assert_eq!(b.locators[0].kind, LocatorKind::Global);
        assert_eq!(b.locators[0].native_unit, "mm");
        assert_eq!(b.locators[2].kind, LocatorKind::Dimension);
        assert_eq!(b.locators[2].mode, LocatorMode::Ref);
        let f = &s
            .get(&p("acme.rover::Rover/axle[*].end_diameter"))
            .expect("f")
            .locators[0];
        assert_eq!(f.kind, LocatorKind::Frame);
        assert_eq!(f.sign, Some(Sign::Minus));
        assert_eq!(f.for_alternate.as_deref(), Some("Colson"));
        assert_eq!(f.config.as_deref(), Some("Default"));
        assert_eq!(f.component.as_deref(), Some("wheel<2>"));
    }

    // frob:tests bindings_ref_resolution
    #[test]
    fn bindings_ref_resolution() {
        let s = BindingSet::load_str("sw", FILE).expect("load");
        let path = p("acme.gearbox::CoverPlate.thickness");
        let dim = s.get(&path).expect("b").locators[2].clone();
        let none = HashMap::new();
        assert_eq!(
            s.resolve_ref(&path, &dim, &none, "own"),
            RefTarget::Global("Plate_Thk".into())
        );
        let mut no_ref = dim.clone();
        no_ref.ref_to = None;
        assert_eq!(
            s.resolve_ref(&path, &no_ref, &none, "own"),
            RefTarget::Global("Plate_Thk".into())
        );
        let other = p("acme.gearbox::Cover.t2");
        let mut alias = HashMap::new();
        alias.insert(other.clone(), path);
        assert_eq!(
            s.resolve_ref(&other, &no_ref, &alias, "own"),
            RefTarget::AliasOf("Plate_Thk".into())
        );
        assert_eq!(
            s.resolve_ref(&other, &no_ref, &none, "Cover__t2"),
            RefTarget::OwnGlobal("Cover__t2".into())
        );
    }

    // frob:tests bindings_mangle_rules
    #[test]
    fn bindings_mangle_rules() {
        assert_eq!(
            mangle(&p("acme.axle::Axle.end_diameter"), ""),
            "Axle__end_diameter"
        );
        assert_eq!(
            mangle(&p("acme.rover::Rover/wheel[2].hub_bore"), ""),
            "Rover__wheel_2__hub_bore"
        );
        assert_eq!(
            mangle(&p("acme.rover::Rover/axle[*].end_diameter"), "p_"),
            "p_Rover__axle__end_diameter"
        );
    }

    // frob:tests bindings_mangle_pure_per_path
    #[test]
    fn bindings_mangle_pure_per_path() {
        assert_eq!(mangle(&p("acme.rover::m::A.v"), ""), "A__v");
        assert_eq!(mangle(&p("other.pkg::n::A.v"), ""), "A__v");
        assert_eq!(
            mangle(&p("acme.rail::Rail.site.offset.x"), ""),
            "Rail__site__offset__x"
        );
        assert_eq!(
            mangle(&p("acme.m::Mount/bracket.place.offset[1].x"), ""),
            "Mount__bracket__place__offset_1__x"
        );
        let a = p("acme.rover::drive::motor::Gm42A.rated_torque");
        let b = p("acme.rover::Gm42A.other");
        let alone = mangle_all(std::slice::from_ref(&a), "").expect("ok");
        let with = mangle_all(&[a.clone(), b], "").expect("ok");
        assert_eq!(alone[0].1, "Gm42A__rated_torque");
        assert_eq!(with[0].1, alone[0].1);
    }

    // frob:tests bindings_mangle_collision_r013
    #[test]
    fn bindings_mangle_collision_r013() {
        let a = p("acme.rover::drive::motor::Gm42A.rated_torque");
        let b = p("acme.rover::drive::other::Gm42A.rated_torque");
        let err = mangle_all(&[a.clone(), b.clone()], "").expect_err("collision");
        assert!(matches!(err, BindingsError::NameCollision { .. }));
        assert!(err.to_string().contains(R013));
        // Binding one explicitly leaves it out of the created names.
        let out = mangle_all(&[a], "").expect("resolved");
        assert_eq!(out[0].1, "Gm42A__rated_torque");
        // The same path twice is not a collision.
        assert!(mangle_all(&[b.clone(), b], "").is_ok());
    }

    // frob:tests bindings_was_carry_forward
    #[test]
    fn bindings_was_carry_forward() {
        let mut s = BindingSet::load_str("sw", FILE).expect("load");
        let old = p("acme.gearbox::CoverPlate.thickness");
        let new = p("acme.gearbox::CoverPlate.thk");
        s.carry_forward(&[(old.clone(), new.clone())]);
        assert!(s.get(&old).is_none());
        assert_eq!(s.get(&new).expect("moved").locators.len(), 3);
    }

    // frob:tests bindings_native_overrides
    #[test]
    fn bindings_native_overrides() {
        let mut s = BindingSet::load_str("sw", FILE).expect("load");
        let path = p("acme.gearbox::CoverPlate.thickness");
        let pin = |adapter: &str, loc: &str| NativePin {
            path: path.clone(),
            adapter: adapter.into(),
            locator: loc.into(),
        };
        s.apply_native(&[pin("other", "ignored"), pin("sw", "Pinned")]);
        let b = s.get(&path).expect("b");
        assert_eq!(b.locators.len(), 1);
        assert_eq!(b.locators[0].locator, "Pinned");
    }

    // frob:tests bindings_emits_only_bound
    #[test]
    fn bindings_emits_only_bound() {
        let s = BindingSet::load_str("sw", FILE).expect("load");
        let bound = p("acme.gearbox::CoverPlate.thickness");
        assert!(s.emits(&bound, false));
        assert!(!s.emits(&bound, true));
        assert!(!s.emits(&p("acme.gearbox::CoverPlate.other"), false));
        assert!(s.emits(&p("acme.rover::Rover/axle[1].end_diameter"), false));
    }

    // frob:tests bindings_orphans_r005
    #[test]
    fn bindings_orphans_r005() {
        let s = BindingSet::load_str("sw", FILE).expect("load");
        let known = [p("acme.gearbox::CoverPlate.thickness")];
        let d = s.orphans(known.iter());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "R005");
    }

    #[test]
    fn bindings_rejects_bad_input() {
        assert!(matches!(
            BindingSet::load_str("sw", "[\"nonsense\"]\nlocators = []"),
            Err(BindingsError::BadPath { .. })
        ));
        let bad = "[\"a.b::C.d\"]\nlocators = [{ locator = \"x\", kind = \"global\", sign = 2 }]";
        assert!(matches!(
            BindingSet::load_str("sw", bad),
            Err(BindingsError::BadSign { .. })
        ));
    }

    use cadetta_ir::model::{Exact, Nominal, ObservedFrame, ObservedInstance, PackageId};
    use cadetta_ir::observed::{ObservedBuilder, ObservedParam};
    use cadetta_units::{Dim, Magnitude, Rational};

    fn src(path: &str, kind: MemberKind, was: &[&str]) -> NameSource {
        NameSource {
            path: p(path),
            was: was.iter().map(|s| (*s).to_owned()).collect(),
            kind,
        }
    }

    fn native(name: &str, kind: LocatorKind, expr: Option<&str>) -> ObservedParam {
        ObservedParam {
            native_name: name.into(),
            locator: Locator {
                locator: name.into(),
                kind,
                mode: LocatorMode::Value,
                native_unit: "mm".into(),
                ref_to: None,
                for_alternate: None,
                config: None,
                sign: None,
                component: None,
                implicit: None,
                tolerance: None,
            },
            nominal: Nominal::Exact(Exact(Magnitude::new(Rational::new(5, 1).unwrap(), 0))),
            dim: Dim::default(),
            text: "5".into(),
            expr: expr.map(str::to_owned),
            unit_assumed: Some("mm".into()),
            inexact_reason: None,
        }
    }

    fn observed(params: Vec<ObservedParam>) -> Project {
        let pkg = PackageId {
            name: "acme.gearbox".into(),
            version: "0.0.0".into(),
        };
        let mut b = ObservedBuilder::new(pkg, "sw", "a.sldprt").expect("builder");
        for q in params {
            b.push(q).expect("push");
        }
        b.build()
    }

    fn table() -> ForwardTable {
        ForwardTable::build(
            &[
                src("acme.gearbox::CoverPlate.thickness", MemberKind::Param, &[]),
                src(
                    "acme.gearbox::CoverPlate.width",
                    MemberKind::Param,
                    &["acme.gearbox::CoverPlate.w"],
                ),
                src("acme.gearbox::CoverPlate.origin", MemberKind::Datum, &[]),
            ],
            "",
        )
        .expect("table")
    }

    fn name_of(path: &str) -> String {
        mangle(&p(path), "")
    }

    // frob:tests discover_global_by_created_name
    #[test]
    fn discover_global_by_created_name() {
        let g = name_of("acme.gearbox::CoverPlate.thickness");
        let d = discover(
            &observed(vec![native(&g, LocatorKind::Global, None)]),
            &table(),
            &[],
        );
        assert!(d.diagnostics.is_empty());
        let l = &d.bindings[0].locators[0];
        assert_eq!(d.bindings[0].path, p("acme.gearbox::CoverPlate.thickness"));
        assert_eq!(
            (l.kind, l.implicit),
            (LocatorKind::Global, Some(Implicit::Global))
        );
    }

    // frob:tests discover_global_by_was_name
    #[test]
    fn discover_global_by_was_name() {
        let old = name_of("acme.gearbox::CoverPlate.w");
        let d = discover(
            &observed(vec![native(&old, LocatorKind::Global, None)]),
            &table(),
            &[],
        );
        assert!(d.diagnostics.is_empty());
        assert_eq!(d.bindings[0].path, p("acme.gearbox::CoverPlate.width"));
    }

    // frob:tests discover_was_and_current_names_r002
    #[test]
    fn discover_was_and_current_names_r002() {
        let old = name_of("acme.gearbox::CoverPlate.w");
        let cur = name_of("acme.gearbox::CoverPlate.width");
        let d = discover(
            &observed(vec![
                native(&old, LocatorKind::Global, None),
                native(&cur, LocatorKind::Global, None),
            ]),
            &table(),
            &[],
        );
        assert_eq!(d.bindings.len(), 1);
        assert_eq!(d.bindings[0].locators[0].locator, cur);
        assert_eq!(d.diagnostics.len(), 1);
        assert_eq!(d.diagnostics[0].code, R002);
    }

    // frob:tests discover_dimension_link
    #[test]
    fn discover_dimension_link() {
        let g = name_of("acme.gearbox::CoverPlate.thickness");
        let d = discover(
            &observed(vec![
                native(&g, LocatorKind::Global, None),
                native("D1@Boss", LocatorKind::Dimension, Some(&format!("\"{g}\""))),
            ]),
            &table(),
            &[],
        );
        assert!(d.diagnostics.is_empty());
        let dim = &d.bindings[0].locators[1];
        assert_eq!(dim.kind, LocatorKind::Dimension);
        assert_eq!(dim.mode, LocatorMode::Ref);
        assert_eq!(dim.ref_to.as_deref(), Some(g.as_str()));
        assert_eq!(dim.implicit, Some(Implicit::Link));
    }

    // frob:tests discover_expression_dimension_r002
    #[test]
    fn discover_expression_dimension_r002() {
        let g = name_of("acme.gearbox::CoverPlate.thickness");
        let expr = format!("\"{g}\" * 2");
        let d = discover(
            &observed(vec![
                native(&g, LocatorKind::Global, None),
                native("D1@Boss", LocatorKind::Dimension, Some(&expr)),
            ]),
            &table(),
            &[],
        );
        assert_eq!(d.bindings[0].locators.len(), 1);
        assert_eq!(d.diagnostics[0].code, R002);
        assert!(d.diagnostics[0].message.contains(&expr));
    }

    // frob:tests discover_bare_unbound_reference_is_silent
    #[test]
    fn discover_bare_unbound_reference_is_silent() {
        let g = name_of("acme.gearbox::CoverPlate.thickness");
        let d = discover(
            &observed(vec![
                native(&g, LocatorKind::Global, None),
                native("D1@Boss", LocatorKind::Dimension, Some("\"Other\"")),
            ]),
            &table(),
            &[],
        );
        assert!(d.diagnostics.is_empty());
        assert_eq!(d.bindings[0].locators.len(), 1);
    }

    // frob:tests discover_frame_by_datum_name
    #[test]
    fn discover_frame_by_datum_name() {
        let f = name_of("acme.gearbox::CoverPlate.origin");
        let mut o = observed(vec![]);
        o.frames.push(ObservedFrame {
            name: f.clone(),
            document: "a".into(),
            transform: vec![],
        });
        o.frames.push(ObservedFrame {
            name: name_of("acme.gearbox::CoverPlate.thickness"),
            document: "a".into(),
            transform: vec![],
        });
        let d = discover(&o, &table(), &[]);
        assert_eq!(d.bindings.len(), 1, "a frame binds a datum only");
        let l = &d.bindings[0].locators[0];
        assert_eq!(
            (l.kind, l.implicit),
            (LocatorKind::Frame, Some(Implicit::Frame))
        );
        assert_eq!(l.locator, f);
    }

    fn inst(c: &str, r: &str) -> ObservedInstance {
        ObservedInstance {
            component: c.into(),
            document: "asm".into(),
            reference: Some(r.into()),
            transform: vec![],
        }
    }

    fn placements() -> Vec<Placement> {
        vec![Placement {
            reference: "bracket".into(),
            node: p("acme.gearbox::Mount/bracket.place"),
        }]
    }

    // frob:tests discover_instance_reference
    #[test]
    fn discover_instance_reference() {
        let mut o = observed(vec![]);
        o.instances.push(inst("bracket<1>", "bracket"));
        o.instances.push(inst("other<1>", "unknown"));
        let d = discover(&o, &table(), &placements());
        assert!(d.diagnostics.is_empty());
        assert_eq!(d.bindings.len(), 1);
        let l = &d.bindings[0].locators[0];
        assert_eq!(
            (l.kind, l.implicit),
            (LocatorKind::Instance, Some(Implicit::Reference))
        );
        assert_eq!(l.locator, "bracket<1>");
    }

    // frob:tests discover_two_instances_one_part_r013
    #[test]
    fn discover_two_instances_one_part_r013() {
        let mut o = observed(vec![]);
        o.instances.push(inst("bracket<1>", "bracket"));
        o.instances.push(inst("bracket<2>", "bracket"));
        let d = discover(&o, &table(), &placements());
        assert!(d.bindings.is_empty());
        assert_eq!(d.diagnostics[0].code, R013);
    }

    fn implicit_global(path: &str) -> Binding {
        Binding {
            path: p(path),
            adapter: "sw".into(),
            locators: vec![implicit_locator(
                "G".into(),
                LocatorKind::Global,
                Implicit::Global,
            )],
        }
    }

    // frob:tests implicit_never_in_file_never_r005
    #[test]
    fn implicit_never_in_file_never_r005() {
        let mut s = BindingSet::load_str("sw", "").expect("load");
        s.add_implicit(vec![implicit_global("acme.gearbox::Gone.x")]);
        assert_eq!(s.entries().len(), 1);
        assert!(s.file_entries().is_empty());
        assert!(s.orphans(&[]).is_empty());
    }

    // frob:tests explicit_replaces_implicit_other_item_adds
    #[test]
    fn explicit_replaces_implicit_other_item_adds() {
        let mut s = BindingSet::load_str("sw", FILE).expect("load");
        let path = "acme.gearbox::CoverPlate.thickness";
        let mut same = implicit_global(path);
        same.locators[0].locator = "Plate_Thk".into();
        let other = implicit_global(path);
        s.add_implicit(vec![same, other]);
        let locs = &s.get(&p(path)).expect("b").locators;
        assert_eq!(locs.len(), 4);
        assert_eq!(locs.iter().filter(|l| l.implicit.is_some()).count(), 1);
        assert_eq!(
            s.file_entries()
                .iter()
                .find(|b| b.path == p(path))
                .unwrap()
                .locators
                .len(),
            3
        );
    }

    fn cand(path: &str, kind: MemberKind, text: bool) -> ScopeCandidate {
        ScopeCandidate {
            path: p(path),
            kind,
            text,
        }
    }

    // frob:tests bind_scope_filters_candidates
    #[test]
    fn bind_scope_filters_candidates() {
        let mut s = BindingSet::load_str("sw", FILE).expect("load");
        let n = s
            .bind_scope(
                &[
                    cand("acme.gearbox::CoverPlate.width", MemberKind::Param, false),
                    cand("acme.gearbox::CoverPlate.origin", MemberKind::Datum, false),
                    cand("acme.gearbox::CoverPlate.scratch", MemberKind::Let, false),
                    cand("acme.other::Thing.size", MemberKind::Param, false),
                    cand("acme.gearbox::CoverPlate.label", MemberKind::Param, true),
                    cand(
                        "acme.gearbox::CoverPlate.thickness",
                        MemberKind::Param,
                        false,
                    ),
                ],
                "acme.gearbox",
                "",
                false,
            )
            .expect("scope");
        assert_eq!(n, 2);
        assert!(s.get(&p("acme.gearbox::CoverPlate.width")).is_some());
        assert!(s.get(&p("acme.gearbox::CoverPlate.scratch")).is_none());
        assert!(s.get(&p("acme.other::Thing.size")).is_none());
        assert!(s.get(&p("acme.gearbox::CoverPlate.label")).is_none());
        let t = s.get(&p("acme.gearbox::CoverPlate.thickness")).unwrap();
        assert!(
            t.locators.iter().all(|l| l.implicit.is_none()),
            "explicit global kept"
        );
        assert!(
            s.file_entries()
                .iter()
                .all(|b| b.locators.iter().all(|l| l.implicit.is_none()))
        );
    }

    // frob:tests bind_scope_text_when_expressible
    #[test]
    fn bind_scope_text_when_expressible() {
        let mut s = BindingSet::load_str("sw", "").expect("load");
        let n = s
            .bind_scope(
                &[cand(
                    "acme.gearbox::CoverPlate.label",
                    MemberKind::Param,
                    true,
                )],
                "acme.gearbox",
                "",
                true,
            )
            .expect("scope");
        assert_eq!(n, 1);
    }

    // frob:tests writer_round_trips_and_bind_is_idempotent
    #[test]
    fn writer_round_trips_and_bind_is_idempotent() {
        let mut s = BindingSet::load_str("sw", FILE).expect("load");
        let again = BindingSet::load_str("sw", &s.to_toml()).expect("reload");
        assert_eq!(again.entries(), s.entries());
        let path = p("acme.gearbox::CoverPlate.width");
        let loc = Locator {
            locator: "Plate_W".into(),
            kind: LocatorKind::Global,
            mode: LocatorMode::Value,
            native_unit: "mm".into(),
            ref_to: None,
            for_alternate: None,
            config: None,
            sign: Some(Sign::Minus),
            component: None,
            implicit: None,
            tolerance: None,
        };
        assert_eq!(s.bind(&path, loc.clone()), Ok(true));
        assert_eq!(s.bind(&path, loc.clone()), Ok(false));
        let mut other = loc.clone();
        other.native_unit = "in".into();
        assert!(matches!(
            s.bind(&path, other),
            Err(BindingsError::Conflict { .. })
        ));
        let text = s.to_toml();
        assert!(text.find("CoverPlate.thickness") < text.find("CoverPlate.width"));
        let back = BindingSet::load_str("sw", &text).expect("reload");
        assert_eq!(back.get(&path).expect("w").locators, vec![loc]);
    }

    // frob:ticket 01M4VPEE2SE4Y96XGGN17H42KT
    // frob:tests writer_writes_tolerance_canonically
    #[test]
    fn writer_writes_tolerance_canonically() {
        let mut s = BindingSet::load_str("sw", "").expect("load");
        let path = p("acme.gearbox::CoverPlate.width");
        let loc = Locator {
            locator: "D1@Sketch1".into(),
            kind: LocatorKind::Dimension,
            mode: LocatorMode::Ref,
            native_unit: String::new(),
            ref_to: None,
            for_alternate: None,
            config: None,
            sign: None,
            component: None,
            implicit: None,
            tolerance: Some(TolerancePolicy::Ignore),
        };
        assert_eq!(s.bind(&path, loc.clone()), Ok(true));
        assert_eq!(s.bind(&path, loc.clone()), Ok(false));
        let text = s.to_toml();
        assert!(text.contains(", tolerance = \"ignore\" }"), "{text}");
        let back = BindingSet::load_str("sw", &text).expect("reload");
        assert_eq!(back.get(&path).expect("w").locators, vec![loc.clone()]);
        let mut other = loc;
        other.tolerance = Some(TolerancePolicy::Drive);
        assert!(matches!(
            s.bind(&path, other),
            Err(BindingsError::Conflict { .. })
        ));
    }

    // frob:tests unbind_removes_and_drops_empty_tables
    #[test]
    fn unbind_removes_and_drops_empty_tables() {
        let mut s = BindingSet::load_str("sw", FILE).expect("load");
        let rover = p("acme.rover::Rover/axle[*].end_diameter");
        assert!(!s.unbind(&rover, "nope"));
        assert!(s.unbind(&rover, "X"));
        assert!(s.get(&rover).is_none());
        assert!(!s.to_toml().contains("rover"));
    }
}
