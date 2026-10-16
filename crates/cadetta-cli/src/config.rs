//! frob:ticket 01M4SPWSTRG1FFAJ0XM89ETBBN
//! The typed model of `cadetta.toml` (config-cli.md 2, packages.md 1).
//!
//! Loading is strict: unknown keys are errors that name the key (so the
//! retired `[project]` table and its `sources` and `roots` keys are
//! rejected), and every value is validated. Numeric quantities such as
//! `common_values` stay strings in the model, so no inexact number is ever
//! stored here; [`Lint::d_config`] parses them exactly when a lint needs them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cadetta_ir::model::TolerancePolicy;
use cadetta_lint::rules::d::{DConfig, Key, NamedValue};
use cadetta_lint::rules::m::MConfig;
use cadetta_units::{Dim, KindTable, Magnitude, Rational, UnitTable};
use serde::Deserialize;

/// Why a `cadetta.toml` could not be loaded (config-cli.md exit code 2).
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Read {
        /// The file that failed to read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// Malformed TOML, a wrong type, or an unknown key (named in the message).
    #[error("invalid cadetta.toml: {0}")]
    Parse(#[from] toml::de::Error),
    /// Neither a `[package]` nor a `[workspace]` table is present.
    #[error("cadetta.toml has neither a [package] nor a [workspace] table")]
    NoProject,
    /// Both a `[package]` and a `[workspace]` table are present.
    #[error("cadetta.toml has both a [package] and a [workspace] table; use one")]
    BothProjectKinds,
    /// A key parsed but its value is not acceptable.
    #[error("invalid value for `{key}`: {reason}")]
    Invalid {
        /// The dotted key path of the offending value.
        key: String,
        /// What is wrong with it.
        reason: String,
    },
}

/// Build an [`ConfigError::Invalid`] result.
fn invalid<T>(key: impl Into<String>, reason: impl Into<String>) -> Result<T, ConfigError> {
    let (key, reason) = (key.into(), reason.into());
    tracing::warn!(%key, %reason, "rejecting configuration value");
    Err(ConfigError::Invalid { key, reason })
}

/// `[lint] fail_on`: the lowest severity that makes a command exit 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailOn {
    /// Fail on errors only (the default).
    #[default]
    Error,
    /// Fail on warnings and errors.
    Warning,
    /// Fail on any diagnostic.
    Info,
    /// Never fail on diagnostics.
    Never,
}

/// A diagnostic severity (lints.md 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational.
    Info,
    /// A warning.
    Warning,
    /// An error.
    Error,
}

/// The tolerance point written to CAD (quantities.md 5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmitPoint {
    /// The nominal value (the default).
    #[default]
    Nominal,
    /// The middle of the tolerance band.
    Mid,
    /// Maximum material condition.
    Mmc,
    /// Least material condition.
    Lmc,
}

/// The gate an obligation role or kind is required for (obligations.md 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Gate {
    /// Required by `cadetta check`.
    Check,
    /// Required by `cadetta check --release`.
    Release,
}

/// How a solver exchanges data (config-cli.md 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// Over the child's standard streams (the default).
    #[default]
    Stdio,
    /// Through files.
    File,
}

/// When an external adapter saves CAD files (adapters.md 2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SaveMode {
    /// Save after each sync (the default).
    #[default]
    AfterSync,
    /// Never save.
    Never,
}

/// Which in-scope paths an adapter binds without a bindings entry (adapters.md 3.5).
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BindMode {
    /// Only bindings-file entries, pins and discovered links (the default).
    #[default]
    Explicit,
    /// Also every in-scope expressible path, to its created global.
    Scope,
}

/// `[package]` (packages.md 1).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    /// Lower dotted package name; `None` means the normalised directory name.
    #[serde(default)]
    pub name: Option<String>,
    /// Semver version.
    #[serde(default)]
    pub version: Option<String>,
    /// Module root, relative to the package directory.
    #[serde(default = "default_src")]
    pub src: String,
    /// Fixed date (`YYYY-MM-DD`) for expiry; `None` means today.
    #[serde(default)]
    pub as_of: Option<String>,
}

/// The default module root.
fn default_src() -> String {
    "src".to_owned()
}

/// One `[dependencies]` entry (packages.md 1, 6).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    /// Local source path.
    #[serde(default)]
    pub path: Option<String>,
    /// Git source URL.
    #[serde(default)]
    pub git: Option<String>,
    /// Git tag (with `git`).
    #[serde(default)]
    pub tag: Option<String>,
    /// Git revision (with `git`).
    #[serde(default)]
    pub rev: Option<String>,
    /// Git branch (with `git`).
    #[serde(default)]
    pub branch: Option<String>,
    /// Semver requirement the resolved package must meet.
    #[serde(default)]
    pub version: Option<String>,
    /// Roles whose upstream attestations discharge dependent obligations.
    #[serde(default)]
    pub trust: Vec<String>,
}

/// `[workspace]` (packages.md 1).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    /// Member package directories.
    pub members: Vec<String>,
}

/// `[units]`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Units {
    /// Display unit per kind (quantities.md 6).
    #[serde(default)]
    pub display: BTreeMap<String, String>,
}

/// `[emit]`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Emit {
    /// Tolerance point written to CAD.
    #[serde(default)]
    pub point: EmitPoint,
}

/// `[lint]` (lints.md 1, config-cli.md 2).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lint {
    /// Lowest severity that makes a command exit 1.
    #[serde(default)]
    pub fail_on: FailOn,
    /// Max significant decimals per kind before M001 fires.
    #[serde(default)]
    pub precision: BTreeMap<String, u8>,
    /// D001 full severity from this many significant digits.
    #[serde(default = "default_d001_min_digits")]
    pub d001_min_digits: u32,
    /// Round numbers (quantity literals, kept as text) D001 never reports.
    #[serde(default)]
    pub common_values: Vec<String>,
    /// Per-code severity overrides.
    #[serde(default)]
    pub severity: BTreeMap<String, Severity>,
    /// W005 fires this many days before expiry.
    #[serde(default = "default_expiry_warn_days")]
    pub expiry_warn_days: u32,
}

/// The default `d001_min_digits`.
fn default_d001_min_digits() -> u32 {
    3
}

/// The default `expiry_warn_days`.
fn default_expiry_warn_days() -> u32 {
    30
}

impl Default for Lint {
    fn default() -> Self {
        Self {
            fail_on: FailOn::default(),
            precision: BTreeMap::new(),
            d001_min_digits: default_d001_min_digits(),
            common_values: Vec::new(),
            severity: BTreeMap::new(),
            expiry_warn_days: default_expiry_warn_days(),
        }
    }
}

/// The currencies named by quantities.md 3 (`Money`); each is its own base dimension.
const MONEY_CODES: [&str; 5] = ["USD", "EUR", "GBP", "CNY", "JPY"];

/// The dimension of every documented currency, for a `Money` precision entry.
fn money_dims() -> Vec<Dim> {
    let units = UnitTable::builtin();
    MONEY_CODES
        .iter()
        .filter_map(|code| units.lookup(code))
        .map(|u| u.dim)
        .collect()
}

impl Lint {
    /// The M001 settings: `precision` by kind name, resolved against `kinds` (built-ins plus
    /// the package's own `kind` declarations); empty keeps the documented defaults.
    pub fn m_config(&self, kinds: &KindTable) -> Result<MConfig, ConfigError> {
        self.resolve_precision(kinds, true)
    }

    /// Resolve `precision`; an unknown kind is an error when `strict`, else logged and skipped.
    fn resolve_precision(&self, kinds: &KindTable, strict: bool) -> Result<MConfig, ConfigError> {
        if self.precision.is_empty() {
            return Ok(MConfig::default());
        }
        let mut precision = Vec::with_capacity(self.precision.len());
        for (kind, decimals) in &self.precision {
            if kind == "Money" {
                // Money has no single dimension: each currency is its own base dimension.
                for dim in money_dims() {
                    precision.push((dim, u32::from(*decimals)));
                }
                continue;
            }
            let Some(dim) = kinds.dim_of(kind) else {
                if strict {
                    return invalid(format!("lint.precision.{kind}"), "unknown quantity kind");
                }
                tracing::warn!(kind = %kind, "lint: precision names an unknown quantity kind; ignored");
                continue;
            };
            precision.push((dim.clone(), u32::from(*decimals)));
        }
        tracing::debug!(kinds = precision.len(), "lint: M001 precision from config");
        Ok(MConfig { precision })
    }

    /// The M001 settings for analysis: like `m_config` but an unknown kind is skipped, since the
    /// configuration loads before the package's kinds are known.
    pub fn m_config_lenient(&self, kinds: &KindTable) -> MConfig {
        self.resolve_precision(kinds, false)
            .unwrap_or_else(|_| MConfig::default())
    }

    /// The D001/D002 settings: `d001_min_digits`, parsed `common_values` and the caller's
    /// table cells.
    pub fn d_config(&self, table_cells: Vec<NamedValue>) -> Result<DConfig, ConfigError> {
        let units = UnitTable::builtin();
        let mut common_values = Vec::with_capacity(self.common_values.len());
        for (i, text) in self.common_values.iter().enumerate() {
            match parse_quantity(text, &units) {
                Some((dim, mag)) => common_values.push(Key::new(&dim, &mag)),
                None => {
                    return invalid(
                        format!("lint.common_values[{i}]"),
                        format!("`{text}` is not a quantity literal such as `0.5in`"),
                    );
                }
            }
        }
        Ok(DConfig {
            min_digits: self.d001_min_digits,
            common_values,
            table_cells,
        })
    }
}

/// Parse a quantity literal (`0.5in`, `10mm`) into its dimension and exact SI magnitude.
fn parse_quantity(text: &str, units: &UnitTable) -> Option<(Dim, Magnitude)> {
    let text = text.trim();
    let split = text
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')))
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let value = Rational::parse_decimal(number).ok()?;
    let resolved = units.parse(unit).ok()?;
    Some((resolved.dim, Magnitude::new(value, 0).mul(&resolved.factor)))
}

/// `[obligations.roles.NAME]` (obligations.md 4).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    /// Obligation selectors the role covers.
    #[serde(default)]
    pub covers: Vec<String>,
    /// Gate the role is required for; `None` means not gating.
    #[serde(default)]
    pub required_for: Option<Gate>,
    /// Who may attest; `None` means anyone.
    #[serde(default)]
    pub members: Option<Vec<String>>,
    /// The attester must differ from the author of the newest covered change.
    #[serde(default)]
    pub distinct_from_author: bool,
}

/// `[obligations.kinds.KIND]` (obligations.md 4).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindPolicy {
    /// Gate the kind is required for; `None` means not gating.
    #[serde(default)]
    pub required_for: Option<Gate>,
}

/// `[obligations]`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obligations {
    /// Roles by name.
    #[serde(default)]
    pub roles: BTreeMap<String, Role>,
    /// Per-kind policy by kind name.
    #[serde(default)]
    pub kinds: BTreeMap<String, KindPolicy>,
}

/// `[solvers."qualified::path"]` (solvers.md 4).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Solver {
    /// Argv; `{script}` is substituted by the host.
    pub command: Vec<String>,
    /// Data exchange mode.
    #[serde(default)]
    pub transport: Transport,
    /// Seconds before the solver is killed; unset means no limit (config-cli.md 2).
    // frob:ticket 01M4STAN8QG3QWZQCH5XN3QHMT
    #[serde(default)]
    pub timeout: Option<u64>,
}

/// One `[[adapter]]` table (adapters.md 2, 3, 5).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adapter {
    /// Built-in adapter name, or external adapter label.
    pub name: String,
    /// Instance id; defaults to `name`, required if a name repeats.
    #[serde(default)]
    pub id: Option<String>,
    /// Built-in output path.
    #[serde(default)]
    pub out: Option<String>,
    /// Bindings file; defaults to `bindings/<id>.toml`.
    #[serde(default)]
    pub bindings: Option<String>,
    /// Module path prefixes; empty means all.
    #[serde(default)]
    pub modules: Vec<String>,
    /// Canonical-path globs; empty means all.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Prefix for created native names.
    #[serde(default)]
    pub prefix: String,
    /// `explicit` (default) or `scope`: also bind every in-scope path to its created global.
    // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
    #[serde(default)]
    pub bind: BindMode,
    /// Unit for unitless native values.
    #[serde(default)]
    pub native_unit: Option<String>,
    /// Decimals; `None` means the emitter spec's default.
    #[serde(default)]
    pub precision: Option<u8>,
    /// External adapter argv.
    #[serde(default)]
    pub command: Option<Vec<String>>,
    /// Protocol version an external adapter must report.
    #[serde(default)]
    pub version: Option<String>,
    /// File globs an external adapter owns.
    #[serde(default)]
    pub files: Vec<String>,
    /// External adapter save policy.
    #[serde(default)]
    pub save: SaveMode,
    /// `drive` (default) or `ignore`: write dimension tolerances (adapters.md 3.6).
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    #[serde(default = "default_tolerances")]
    pub tolerances: TolerancePolicy,
    /// `verify` (default) or `create`: create and drive datum coordinate systems (adapters.md 3.4).
    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    #[serde(default)]
    pub datums: cadetta_adapter::geometry_bind::DatumPolicy,
    /// Seconds one adapter call may stay unanswered before the adapter is killed (adapters.md 5).
    // frob:ticket 01M4YPDTD1977TMY4M2PPPNSDD
    #[serde(default)]
    pub timeout: Option<u64>,
}

/// Seconds an external adapter call may stay unanswered when `[[adapter]] timeout` is unset.
pub const DEFAULT_ADAPTER_TIMEOUT_SECS: u64 = 60;

/// The `[[adapter]] tolerances` default, `drive` (adapters.md 3.6).
fn default_tolerances() -> TolerancePolicy {
    TolerancePolicy::Drive
}

impl Adapter {
    /// The instance id: `id`, else `name`.
    pub fn id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }

    /// The per-call adapter timeout: `timeout` seconds, else [`DEFAULT_ADAPTER_TIMEOUT_SECS`].
    // frob:ticket 01M4YPDTD1977TMY4M2PPPNSDD
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout.unwrap_or(DEFAULT_ADAPTER_TIMEOUT_SECS))
    }

    /// The bindings file: `bindings`, else `bindings/<id>.toml`.
    pub fn bindings_path(&self) -> String {
        self.bindings
            .clone()
            .unwrap_or_else(|| format!("bindings/{}.toml", self.id()))
    }
}

/// A whole `cadetta.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// `[package]`, for a package root.
    #[serde(default)]
    pub package: Option<Package>,
    /// `[workspace]`, for a workspace root.
    #[serde(default)]
    pub workspace: Option<Workspace>,
    /// `[dependencies]` by package name.
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
    /// `[defaults]`: qualified path to value, lowest precedence.
    #[serde(default)]
    pub defaults: BTreeMap<String, String>,
    /// `[units]`.
    #[serde(default)]
    pub units: Units,
    /// `[emit]`.
    #[serde(default)]
    pub emit: Emit,
    /// `[lint]`.
    #[serde(default)]
    pub lint: Lint,
    /// `[obligations]`.
    #[serde(default)]
    pub obligations: Obligations,
    /// `[solvers]` by qualified path.
    #[serde(default)]
    pub solvers: BTreeMap<String, Solver>,
    /// `[[adapter]]` tables in file order.
    #[serde(default, rename = "adapter")]
    pub adapters: Vec<Adapter>,
}

impl Config {
    /// Parse and validate `text` as a `cadetta.toml`.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(text).inspect_err(|error| {
            tracing::warn!(%error, "cadetta.toml failed to parse");
        })?;
        config.validate()?;
        tracing::debug!(
            package = config.package.is_some(),
            workspace = config.workspace.is_some(),
            dependencies = config.dependencies.len(),
            adapters = config.adapters.len(),
            "loaded cadetta.toml"
        );
        Ok(config)
    }

    /// Read, parse and validate the file at `path`; an absent package
    /// `name` becomes the normalised directory name (config-cli.md 2).
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        tracing::info!(path = %path.display(), "loading configuration");
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_owned(),
            source,
        })?;
        let mut config = Self::parse(&text)?;
        config.default_package_name(path);
        Ok(config)
    }

    /// Fill an absent package `name` from the directory holding `path`.
    fn default_package_name(&mut self, path: &Path) {
        let Some(package) = self.package.as_mut().filter(|p| p.name.is_none()) else {
            return;
        };
        let dir = path.parent().unwrap_or(Path::new("."));
        let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_owned());
        if let Some(stem) = dir.file_name().and_then(|n| n.to_str()) {
            let name = normalise_package_name(stem);
            tracing::info!(%name, "package name defaults to the directory name");
            package.name = Some(name);
        }
    }

    /// Check every cross-field and value rule not expressible in types.
    fn validate(&self) -> Result<(), ConfigError> {
        match (&self.package, &self.workspace) {
            (None, None) => return Err(ConfigError::NoProject),
            (Some(_), Some(_)) => return Err(ConfigError::BothProjectKinds),
            _ => {}
        }
        if let Some(package) = &self.package {
            validate_package(package)?;
        }
        if let Some(workspace) = &self.workspace
            && workspace.members.is_empty()
        {
            return invalid("workspace.members", "must list at least one member");
        }
        for (name, dependency) in &self.dependencies {
            validate_dependency(name, dependency)?;
        }
        if self.lint.d001_min_digits == 0 {
            return invalid("lint.d001_min_digits", "must be at least 1");
        }
        // Package-declared kinds are unknown at load; `pipeline` resolves `precision` later.
        self.lint.d_config(Vec::new())?;
        for (name, role) in &self.obligations.roles {
            if role.covers.is_empty() {
                return invalid(
                    format!("obligations.roles.{name}.covers"),
                    "must list at least one selector",
                );
            }
        }
        for (path, solver) in &self.solvers {
            if solver.command.is_empty() {
                return invalid(format!("solvers.{path}.command"), "must not be empty");
            }
        }
        self.validate_adapters()
    }

    /// Adapter rules: unique ids, non-empty commands.
    fn validate_adapters(&self) -> Result<(), ConfigError> {
        let mut seen = BTreeMap::new();
        for adapter in &self.adapters {
            if adapter.command.as_ref().is_some_and(Vec::is_empty) {
                return invalid(
                    format!("adapter.{}.command", adapter.id()),
                    "must not be empty",
                );
            }
            if seen.insert(adapter.id(), ()).is_some() {
                return invalid(
                    "adapter.id",
                    format!(
                        "`{}` is used twice; give repeated names distinct ids",
                        adapter.id()
                    ),
                );
            }
        }
        Ok(())
    }
}

/// Validate a `[package]` table.
fn validate_package(package: &Package) -> Result<(), ConfigError> {
    if let Some(name) = &package.name
        && !is_valid_package_name(name)
    {
        return invalid(
            "package.name",
            format!("`{name}` is not lower.dotted (N001) or is reserved"),
        );
    }
    if let Some(version) = &package.version
        && !is_semver(version)
    {
        return invalid(
            "package.version",
            format!("`{version}` is not a semver version"),
        );
    }
    if package.src.is_empty() {
        return invalid("package.src", "must not be empty");
    }
    if let Some(date) = &package.as_of
        && !is_iso_date(date)
    {
        return invalid(
            "package.as_of",
            format!("`{date}` is not a YYYY-MM-DD date"),
        );
    }
    Ok(())
}

/// Validate one `[dependencies]` entry.
fn validate_dependency(name: &str, dep: &Dependency) -> Result<(), ConfigError> {
    let key = format!("dependencies.{name}");
    if !is_valid_package_name(name) {
        return invalid(&key, "the key is not a valid package name or is reserved");
    }
    match (&dep.path, &dep.git) {
        (None, None) => return invalid(&key, "needs a `path` or a `git` source"),
        (Some(_), Some(_)) => return invalid(&key, "`path` and `git` are exclusive"),
        _ => {}
    }
    let refs = [&dep.tag, &dep.rev, &dep.branch]
        .iter()
        .filter(|r| r.is_some())
        .count();
    if dep.git.is_none() && refs > 0 {
        return invalid(&key, "`tag`, `rev` and `branch` need `git`");
    }
    if refs > 1 {
        return invalid(&key, "give at most one of `tag`, `rev` and `branch`");
    }
    Ok(())
}

/// Whether `name` is `lower.dotted` (segments of `[a-z][a-z0-9_]*`) and not `std`.
pub fn is_valid_package_name(name: &str) -> bool {
    name != "std"
        && !name.is_empty()
        && name.split('.').all(|seg| {
            let mut chars = seg.chars();
            chars.next().is_some_and(|c| c.is_ascii_lowercase())
                && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
}

/// Normalise a directory name into a package name: lowercase, runs of
/// other characters become `_`, a leading non-letter gets a `p` prefix.
pub fn normalise_package_name(dir_name: &str) -> String {
    let mut out = String::new();
    for c in dir_name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches(|c| c == '_' || c == '.').to_owned();
    if out.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        out
    } else {
        format!("p{out}")
    }
}

/// Whether `v` is `MAJOR.MINOR.PATCH` with optional `-pre` and `+build`.
fn is_semver(v: &str) -> bool {
    let core = v.split(['-', '+']).next().unwrap_or("");
    let parts: Vec<&str> = core.split('.').collect();
    let rest = &v[core.len()..];
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        && (rest.is_empty()
            || (rest[1..]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
                && rest.len() > 1))
}

/// Whether `d` is a real calendar date `YYYY-MM-DD`.
fn is_iso_date(d: &str) -> bool {
    let parts: Vec<&str> = d.split('-').collect();
    let [y, m, day] = parts[..] else {
        return false;
    };
    if y.len() != 4 || m.len() != 2 || day.len() != 2 {
        return false;
    }
    let (Ok(y), Ok(m), Ok(day)) = (y.parse::<u32>(), m.parse::<u32>(), day.parse::<u32>()) else {
        return false;
    };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The config-cli.md 2 sample (the workspace comment block omitted).
    const SAMPLE: &str = r#"
[package]
name = "acme.rover"
version = "0.1.0"
src = "src"
as_of = "2026-10-13"

[dependencies]
"acme.shop" = { path = "../shop", version = "1" }
"acme.parts" = { git = "https://example.com/acme/parts.git", tag = "v2.1.0", trust = ["source"] }

[defaults]
FastenerPolicy = "acme.shop::AcmeShop"

[units]
display = { Length = "mm", Angle = "deg", Mass = "kg" }

[emit]
point = "nominal"

[lint]
fail_on = "error"
precision = { Length = 2, Angle = 1 }
d001_min_digits = 3
common_values = ["1mm", "2mm", "5mm", "10mm", "0.5in", "1in"]
severity = { N002 = "info" }
expiry_warn_days = 30

[obligations.roles.source]
covers = ["ots:*"]
required_for = "check"
members = ["a@x.org"]
distinct_from_author = false

[obligations.kinds.sync]
required_for = "release"

[solvers."acme.rover::wall_sizing"]
command = ["python", "{script}"]
transport = "stdio"

[[adapter]]
name = "sw-equations"
id = "rover-sw"
out = "build/solidworks/rover-sw.txt"
bindings = "bindings/rover-sw.toml"
modules = ["drive"]
paths = []
prefix = ""
native_unit = "mm"
precision = 6

[[adapter]]
name = "solidworks"
command = ["cadetta-solidworks"]
version = "0.1"
files = ["**/*.SLDPRT", "**/*.SLDASM"]
save = "after-sync"
native_unit = "mm"
"#;

    fn pkg(extra: &str) -> String {
        format!("[package]\nname = \"a.b\"\n{extra}")
    }

    fn err_text(text: &str) -> String {
        Config::parse(text)
            .expect_err("must be rejected")
            .to_string()
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Config
    #[test]
    fn sample_parses_in_every_section() {
        let c = Config::parse(SAMPLE).expect("sample parses");
        assert_eq!(
            c.package.as_ref().unwrap().name.as_deref(),
            Some("acme.rover")
        );
        assert_eq!(c.dependencies.len(), 2);
        assert_eq!(c.defaults["FastenerPolicy"], "acme.shop::AcmeShop");
        assert_eq!(c.obligations.roles.len(), 1);
        assert_eq!(c.solvers.len(), 1);
        assert_eq!(c.adapters.len(), 2);
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Config
    #[test]
    fn unknown_keys_are_named() {
        assert!(err_text(&pkg("bogus = 1\n")).contains("bogus"));
        assert!(err_text(&pkg("[lint]\nfail_onn = \"error\"\n")).contains("fail_onn"));
        assert!(err_text(&pkg("[[adapter]]\nname = \"x\"\nwat = 1\n")).contains("wat"));
    }

    // frob:ticket 01M4TRKD3ZG6X9SKKC8XN48PSV
    // frob:tests crates/cadetta-cli/src/config.rs::Lint
    #[test]
    fn lint_precision_kinds_and_common_values_are_checked() {
        let typo = Config::parse(&pkg("[lint]\nprecision = { Lenth = 2 }\n")).unwrap();
        let err = typo.lint.m_config(&KindTable::builtin()).unwrap_err();
        assert!(err.to_string().contains("Lenth"));
        assert!(err_text(&pkg("[lint]\ncommon_values = [\"fast\"]\n")).contains("fast"));
        let c = Config::parse(&pkg(
            "[lint]\nprecision = { Length = 4 }\ncommon_values = [\"0.5in\"]\n",
        ))
        .unwrap();
        assert_eq!(
            c.lint
                .m_config(&KindTable::builtin())
                .unwrap()
                .precision
                .len(),
            1
        );
        assert_eq!(c.lint.d_config(Vec::new()).unwrap().common_values.len(), 1);
    }

    // frob:ticket 01M4Z7SF3KHPF3NA2YB3P4PXDX
    // frob:ticket 01M4YM3CKT4FBV4VXJD76GMV4N
    // frob:tests crates/cadetta-cli/src/config.rs::Lint
    #[test]
    fn lint_precision_accepts_custom_kinds_from_table() {
        let c = Config::parse(&pkg("[lint]\nprecision = { Widget = 3 }\n")).unwrap();
        assert!(c.lint.m_config(&KindTable::builtin()).is_err());
        let mut kinds = KindTable::builtin();
        kinds.register("Widget", "Length").unwrap();
        let m = c.lint.m_config(&kinds).unwrap();
        assert_eq!(
            m.precision,
            vec![(kinds.dim_of("Length").unwrap().clone(), 3)]
        );
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Lint
    #[test]
    fn lint_precision_accepts_money_for_every_currency() {
        let c = Config::parse(&pkg("[lint]\nprecision = { Money = 2 }\n")).unwrap();
        let m = c.lint.m_config(&KindTable::builtin()).unwrap();
        assert_eq!(m.precision.len(), 5);
        assert!(m.precision.iter().all(|(_, n)| *n == 2));
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Lint
    #[test]
    fn lint_keys_are_typed_and_validated() {
        let c = Config::parse(SAMPLE).unwrap();
        assert_eq!(c.lint.fail_on, FailOn::Error);
        assert_eq!(c.lint.precision["Length"], 2);
        assert_eq!(c.lint.d001_min_digits, 3);
        assert_eq!(c.lint.common_values[4], "0.5in");
        assert_eq!(c.lint.severity["N002"], Severity::Info);
        assert!(err_text(&pkg("[lint]\nd001_min_digits = 0\n")).contains("d001_min_digits"));
        assert!(err_text(&pkg("[lint]\nseverity = { N002 = \"loud\" }\n")).contains("loud"));
        assert!(err_text(&pkg("[lint]\nprecision = { Length = -1 }\n")).contains("invalid"));
        assert!(err_text(&pkg("[lint]\nfail_on = \"sometimes\"\n")).contains("sometimes"));
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Role
    #[test]
    fn roles_are_modelled() {
        let c = Config::parse(SAMPLE).unwrap();
        let role = &c.obligations.roles["source"];
        assert_eq!(role.covers, ["ots:*"]);
        assert_eq!(role.required_for, Some(Gate::Check));
        assert_eq!(role.members.as_deref(), Some(&["a@x.org".to_owned()][..]));
        assert!(!role.distinct_from_author);
        assert_eq!(
            c.obligations.kinds["sync"].required_for,
            Some(Gate::Release)
        );
        assert!(
            err_text(&pkg("[obligations.roles.r]\nrequired_for = \"never\"\n")).contains("never")
        );
        assert!(err_text(&pkg("[obligations.roles.r]\n")).contains("covers"));
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Adapter
    #[test]
    fn solvers_and_adapters_parse() {
        let c = Config::parse(SAMPLE).unwrap();
        let s = &c.solvers["acme.rover::wall_sizing"];
        assert_eq!(s.command, ["python", "{script}"]);
        assert_eq!(s.transport, Transport::Stdio);
        let (builtin, ext) = (&c.adapters[0], &c.adapters[1]);
        assert_eq!(builtin.id(), "rover-sw");
        assert_eq!(builtin.precision, Some(6));
        assert_eq!(ext.id(), "solidworks");
        assert_eq!(ext.bindings_path(), "bindings/solidworks.toml");
        assert_eq!(
            ext.command.as_deref(),
            Some(&["cadetta-solidworks".to_owned()][..])
        );
        assert_eq!(ext.version.as_deref(), Some("0.1"));
        assert_eq!(ext.files.len(), 2);
        assert_eq!(ext.native_unit.as_deref(), Some("mm"));
        assert_eq!(ext.save, SaveMode::AfterSync);
        let dup = pkg("[[adapter]]\nname = \"x\"\n[[adapter]]\nname = \"x\"\n");
        assert!(err_text(&dup).contains("used twice"));
        let file = pkg("[solvers.\"a::b\"]\ncommand = [\"x\"]\ntransport = \"file\"\n");
        assert_eq!(
            Config::parse(&file).unwrap().solvers["a::b"].transport,
            Transport::File
        );
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Emit
    #[test]
    fn emit_point_and_display_units() {
        let c = Config::parse(SAMPLE).unwrap();
        assert_eq!(c.emit.point, EmitPoint::Nominal);
        assert_eq!(c.units.display["Length"], "mm");
        let c = Config::parse(&pkg("[emit]\npoint = \"mmc\"\n")).unwrap();
        assert_eq!(c.emit.point, EmitPoint::Mmc);
        assert!(err_text(&pkg("[emit]\npoint = \"max\"\n")).contains("max"));
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Config
    #[test]
    fn defaults_match_the_spec() {
        let c = Config::parse("[package]\n").unwrap();
        let p = c.package.unwrap();
        assert_eq!((p.name, p.version, p.as_of), (None, None, None));
        assert_eq!(p.src, "src");
        assert!(c.dependencies.is_empty() && c.defaults.is_empty() && c.solvers.is_empty());
        assert!(c.adapters.is_empty() && c.units.display.is_empty());
        assert_eq!(c.emit.point, EmitPoint::Nominal);
        assert_eq!(c.lint, Lint::default());
        assert_eq!(c.lint.fail_on, FailOn::Error);
        assert_eq!(c.lint.d001_min_digits, 3);
        assert_eq!(c.lint.expiry_warn_days, 30);
        let a = Config::parse(&pkg("[[adapter]]\nname = \"freecad\"\n"))
            .unwrap()
            .adapters;
        assert_eq!((a[0].prefix.as_str(), a[0].save), ("", SaveMode::AfterSync));
        assert!(a[0].modules.is_empty() && a[0].paths.is_empty());
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Package
    #[test]
    fn package_dependencies_and_workspace() {
        let c = Config::parse(SAMPLE).unwrap();
        let shop = &c.dependencies["acme.shop"];
        assert_eq!(
            (shop.path.as_deref(), shop.version.as_deref()),
            (Some("../shop"), Some("1"))
        );
        let parts = &c.dependencies["acme.parts"];
        assert_eq!(parts.tag.as_deref(), Some("v2.1.0"));
        assert_eq!(parts.trust, ["source"]);

        let w = Config::parse("[workspace]\nmembers = [\"rover\", \"shop\"]\n").unwrap();
        assert_eq!(w.workspace.unwrap().members, ["rover", "shop"]);
        assert!(w.package.is_none());

        assert!(matches!(Config::parse(""), Err(ConfigError::NoProject)));
        assert!(matches!(
            Config::parse("[package]\n[workspace]\nmembers = [\"a\"]\n"),
            Err(ConfigError::BothProjectKinds)
        ));
        assert!(err_text("[workspace]\nmembers = []\n").contains("members"));
        assert!(Config::parse("[package]\nname = \"Acme\"\n").is_err());
        assert!(Config::parse("[package]\nname = \"std\"\n").is_err());
        assert!(Config::parse("[package]\nversion = \"1.0\"\n").is_err());
        assert!(Config::parse("[package]\nas_of = \"2026-02-30\"\n").is_err());
        assert!(
            Config::parse("[package]\nversion = \"1.0.0-rc.1\"\nas_of = \"2028-02-29\"\n").is_ok()
        );
        for bad in [
            "\"x.y\" = {}",
            "\"x.y\" = { path = \"a\", git = \"b\" }",
            "\"x.y\" = { path = \"a\", tag = \"t\" }",
            "\"x.y\" = { git = \"a\", tag = \"t\", rev = \"r\" }",
            "std = { path = \"a\" }",
        ] {
            assert!(
                Config::parse(&format!("[package]\n[dependencies]\n{bad}\n")).is_err(),
                "{bad}"
            );
        }
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Config
    #[test]
    fn old_project_namespace_is_rejected() {
        assert!(err_text("[project]\nname = \"a\"\n").contains("project"));
        assert!(err_text(&pkg("sources = [\"src\"]\n")).contains("sources"));
        assert!(err_text(&pkg("roots = [\"a\"]\n")).contains("roots"));
    }

    // frob:tests crates/cadetta-cli/src/config.rs::normalise_package_name
    #[test]
    fn directory_names_normalise() {
        assert_eq!(normalise_package_name("My Rover-2"), "my_rover_2");
        assert_eq!(normalise_package_name("9lives"), "p9lives");
        assert!(is_valid_package_name(&normalise_package_name(
            "--Weird  Dir--"
        )));
    }

    // frob:tests crates/cadetta-cli/src/config.rs::Config
    #[test]
    fn load_reports_io_and_defaults_the_name() {
        let missing = Path::new("/nonexistent-cadetta-dir/cadetta.toml");
        assert!(matches!(
            Config::load(missing),
            Err(ConfigError::Read { .. })
        ));
        let mut c = Config::parse("[package]\n").unwrap();
        c.default_package_name(Path::new("/nonexistent-cadetta-dir/My Rover/cadetta.toml"));
        assert_eq!(c.package.unwrap().name.as_deref(), Some("my_rover"));
    }

    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    // frob:tests tolerances_key_defaults_drive_and_is_strict
    #[test]
    fn tolerances_key_defaults_drive_and_is_strict() {
        let c = Config::parse(SAMPLE).unwrap();
        assert_eq!(c.adapters[1].tolerances, TolerancePolicy::Drive);
        let ign =
            Config::parse(&pkg("[[adapter]]\nname = \"x\"\ntolerances = \"ignore\"\n")).unwrap();
        assert_eq!(ign.adapters[0].tolerances, TolerancePolicy::Ignore);
        assert!(err_text(&pkg("[[adapter]]\nname = \"x\"\ntolerances = \"on\"\n")).contains("on"));
    }

    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    // frob:tests datums_key_defaults_verify_and_is_strict
    #[test]
    fn datums_key_defaults_verify_and_is_strict() {
        use cadetta_adapter::geometry_bind::DatumPolicy;
        let c = Config::parse(SAMPLE).unwrap();
        assert_eq!(c.adapters[1].datums, DatumPolicy::Verify);
        let cr = Config::parse(&pkg("[[adapter]]\nname = \"x\"\ndatums = \"create\"\n")).unwrap();
        assert_eq!(cr.adapters[0].datums, DatumPolicy::Create);
        assert!(
            err_text(&pkg("[[adapter]]\nname = \"x\"\ndatums = \"drive\"\n")).contains("drive")
        );
    }

    // frob:tests crates/cadetta-cli/src/config.rs::BindMode
    #[test]
    fn bind_mode_defaults_explicit_and_is_strict() {
        let c = Config::parse(SAMPLE).unwrap();
        assert_eq!(c.adapters[1].bind, BindMode::Explicit);
        let scope = Config::parse(&pkg(
            "[[adapter]]\nname = \"sw-equations\"\nbind = \"scope\"\n",
        ))
        .unwrap();
        assert_eq!(scope.adapters[0].bind, BindMode::Scope);
        assert!(err_text(&pkg("[[adapter]]\nname = \"x\"\nbind = \"all\"\n")).contains("all"));
    }
}
