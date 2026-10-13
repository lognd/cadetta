//! Bindings files, multi-locator bindings and name mangling (adapters.md 3, 3.1).
//!
//! `bindings/<id>.toml` maps canonical paths to native locators, one table per
//! path with a `locators` array. `#[native(adapter, "locator")]` pins one inline
//! and overrides the file entry. Names the adapter creates itself are derived
//! deterministically from the path by [`mangle`] / [`mangle_all`].
// frob:ticket 01M4SPWRYRBTGD67WTG7D60E59

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use cadetta_ir::model::{Binding, Locator, LocatorKind, LocatorMode, Sign};
use cadetta_ir::path::{Index, Path, PathContext, Seg};
use serde::Deserialize;

use crate::{AdapterDiagnostic, Severity};

/// Diagnostic code for a binding that matches no path (adapters.md 3).
pub const R005: &str = "R005";
/// Diagnostic code for two emitted paths creating one native name (adapters.md 3.1).
pub const R013: &str = "R013";

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
    /// Two paths still create one native name after full qualification (R013).
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
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum RawKind {
    Global,
    Dimension,
    Property,
    Frame,
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

    /// All bindings, sorted by path text.
    pub fn entries(&self) -> &[Binding] {
        &self.entries
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
    };
    crate::locator::validate(&loc).map_err(|e| BindingsError::BadLocator {
        path: key.to_owned(),
        reason: e.to_string(),
    })?;
    Ok(loc)
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

/// Qualification level of a mangled name (adapters.md 3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Plain,
    Module,
    Package,
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

fn mangle_at(path: &Path, prefix: &str, level: Level) -> String {
    let mut parts: Vec<String> = Vec::new();
    if level >= Level::Package {
        parts.push(path.package.replace('.', "_"));
    }
    if level >= Level::Module {
        parts.extend(path.modules.iter().cloned());
    }
    parts.push(path.item.clone());
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

/// The default native name for `path`: package and modules dropped, `.` and `/` become `__`,
/// `[i]` becomes `_i`, `[*]` is dropped; `prefix` is prepended (adapters.md 3.1).
pub fn mangle(path: &Path, prefix: &str) -> String {
    mangle_at(path, prefix, Level::Plain)
}

/// Mangles every path, qualifying colliding names with the module path and then the package.
///
/// Returns [`BindingsError::NameCollision`] (R013) if a collision survives; the mapping is
/// injective on success.
pub fn mangle_all(paths: &[Path], prefix: &str) -> Result<Vec<(Path, String)>, BindingsError> {
    let mut uniq: Vec<&Path> = Vec::new();
    for p in paths {
        if !uniq.contains(&p) {
            uniq.push(p);
        }
    }
    let mut levels = vec![Level::Plain; uniq.len()];
    loop {
        let names: Vec<String> = uniq
            .iter()
            .zip(&levels)
            .map(|(p, l)| mangle_at(p, prefix, *l))
            .collect();
        let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, n) in names.iter().enumerate() {
            groups.entry(n.as_str()).or_default().push(i);
        }
        let mut changed = false;
        for (name, idxs) in groups.iter().filter(|(_, v)| v.len() > 1) {
            for &i in idxs {
                match levels[i] {
                    Level::Plain => levels[i] = Level::Module,
                    Level::Module => levels[i] = Level::Package,
                    Level::Package => {
                        tracing::error!(name, "bindings: name collision (R013)");
                        return Err(BindingsError::NameCollision {
                            name: (*name).to_owned(),
                            paths: idxs.iter().map(|&j| uniq[j].to_string()).collect(),
                        });
                    }
                }
                changed = true;
            }
        }
        if !changed {
            tracing::debug!(n = names.len(), "bindings: names mangled");
            return Ok(uniq.into_iter().cloned().zip(names).collect());
        }
    }
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

    // frob:tests bindings_mangle_injective
    #[test]
    fn bindings_mangle_injective() {
        let a = p("acme.rover::drive::motor::Gm42A.rated_torque");
        let b = p("acme.rover::drive::other::Gm42A.rated_torque");
        let out = mangle_all(&[a, b], "").expect("ok");
        assert_eq!(out[0].1, "drive__motor__Gm42A__rated_torque");
        assert_eq!(out[1].1, "drive__other__Gm42A__rated_torque");
        assert_ne!(out[0].1, out[1].1);
        let x = p("acme.rover::m::A.v");
        let y = p("other.pkg::m::A.v");
        let out = mangle_all(&[x, y], "").expect("ok");
        assert_eq!(out[0].1, "acme_rover__m__A__v");
        assert_eq!(out[1].1, "other_pkg__m__A__v");
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
}
