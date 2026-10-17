//! frob:ticket 01M4SWN8BMEE0S29Y4N4EZTYJT
//! The project model: root discovery, workspace members, packages and
//! their modules (config-cli.md 1, packages.md 1-2).

use std::path::{Path, PathBuf};

use cadetta_syntax::lexer::Span;

use crate::cmd::update::checkout_dir;
use crate::config::{Config, ConfigError};
use crate::pkg::manifest::{self, GitRef, Manifest, PackageManifest, Source};
use crate::pkg::resolve::{self, ResolveError, ResolvedPackage};

/// The configuration file name.
pub const CONFIG_FILE: &str = "cadetta.toml";

/// The source file extension.
pub const SOURCE_EXT: &str = "cdt";

/// Why a project could not be discovered or loaded.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// No `cadetta.toml` with `[package]` or `[workspace]` above the start (exit 2).
    #[error("no cadetta.toml with a [package] or [workspace] table found from {}", start.display())]
    NotFound {
        /// Where the walk began.
        start: PathBuf,
    },
    /// A `cadetta.toml` could not be loaded (exit 2).
    #[error("{}: {source}", path.display())]
    Config {
        /// The offending file.
        path: PathBuf,
        /// What was wrong.
        source: ConfigError,
    },
    /// A directory could not be read (exit 2).
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// The path that failed.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// A project diagnostic (E005).
    #[error("{0}")]
    Diag(ProjectDiag),
}

/// A project-level diagnostic with its code and spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiag {
    /// `E005`.
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
    /// The file or directory concerned.
    pub path: PathBuf,
    /// Source spans in `path` (empty for E005).
    pub spans: Vec<Span>,
}

impl std::fmt::Display for ProjectDiag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} ({})",
            self.code,
            self.message,
            self.path.display()
        )
    }
}

/// Build an E005 (missing-package) error.
fn e005(path: &Path, message: impl Into<String>) -> ProjectError {
    let message = message.into();
    tracing::warn!(code = "E005", path = %path.display(), %message, "project diagnostic");
    ProjectError::Diag(ProjectDiag {
        code: "E005",
        message,
        path: path.to_owned(),
        spans: Vec::new(),
    })
}

/// One `.cdt` file of a package.
#[derive(Debug, Clone)]
pub struct Module {
    /// Path under the package `src`, `/`-separated (the sort key).
    pub rel_path: String,
    /// The file on disk.
    pub file: PathBuf,
    /// The file text.
    pub source: String,
}

/// A loaded package: its configuration and modules sorted by path.
#[derive(Debug, Clone)]
pub struct PackageDir {
    /// The package directory.
    pub dir: PathBuf,
    /// Its parsed `cadetta.toml`.
    pub config: Config,
    /// Its modules, sorted by `rel_path`.
    pub modules: Vec<Module>,
}

/// A discovered project: a package or a workspace and its members.
#[derive(Debug, Clone)]
pub struct Project {
    /// The directory holding the project's `cadetta.toml`.
    pub root: PathBuf,
    /// The root configuration.
    pub config: Config,
    /// Loaded packages: the package itself, or each workspace member in order.
    pub packages: Vec<PackageDir>,
    /// The resolved dependency closure (path sources), one entry per package name.
    pub dependencies: Vec<Dependency>,
}

/// One resolved dependency: how it was resolved and its loaded files.
#[derive(Debug, Clone)]
pub struct Dependency {
    /// The resolution record (name, version, source, directory).
    pub resolved: ResolvedPackage,
    /// The loaded package.
    pub package: PackageDir,
}

/// Walk up from `start` to the nearest `cadetta.toml` with a `[package]`
/// or `[workspace]` table; returns the file path (config-cli.md 1.6).
pub fn find_config(start: &Path) -> Result<PathBuf, ProjectError> {
    let start = absolute(start);
    for dir in start.ancestors() {
        let candidate = dir.join(CONFIG_FILE);
        if !candidate.is_file() {
            continue;
        }
        match Config::load(&candidate) {
            Ok(_) => {
                tracing::debug!(path = %candidate.display(), "root discovered");
                return Ok(candidate);
            }
            Err(ConfigError::NoProject) => {
                tracing::debug!(path = %candidate.display(), "skipping cadetta.toml without a project table");
            }
            Err(source) => {
                return Err(config_error(candidate, source));
            }
        }
    }
    tracing::warn!(start = %start.display(), "no project root found");
    Err(ProjectError::NotFound { start })
}

/// Make `path` absolute against the current directory (no symlink resolution).
fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir().map_or_else(|_| path.to_owned(), |cwd| cwd.join(path))
    }
}

impl Project {
    /// Open the project for `cwd` (default `.`) or an explicit `config`
    /// file (config-cli.md 1.6). Returns the project and the indices into
    /// `packages` a command scopes to: the member containing `cwd` when
    /// run inside a workspace member, otherwise every package.
    pub fn open(
        cwd: Option<&Path>,
        config: Option<&Path>,
    ) -> Result<(Self, Vec<usize>), ProjectError> {
        let start = absolute(cwd.unwrap_or(Path::new(".")));
        let config_path = match config {
            Some(file) => find_config(absolute(file).parent().unwrap_or(Path::new(".")))?,
            None => find_config(&start)?,
        };
        let project = Self::load(&config_path)?;
        let scope = project.scope(&start);
        tracing::info!(root = %project.root.display(), scope = ?scope, "project opened");
        Ok((project, scope))
    }

    /// Load the project whose `cadetta.toml` is `config_path`.
    pub fn load(config_path: &Path) -> Result<Self, ProjectError> {
        let root = config_path.parent().unwrap_or(Path::new("."));
        let cache = root.join("build").join("deps");
        Self::load_with(config_path, &|url, r| cached_checkout(&cache, url, r))
    }

    /// Load the project with `fetch` supplying git dependency checkouts (injectable, so
    /// tests need no network).
    // frob:ticket 01M4YRTECA4XNC784V88ZC83KP
    pub fn load_with(config_path: &Path, fetch: &GitFetch<'_>) -> Result<Self, ProjectError> {
        let root = config_path.parent().unwrap_or(Path::new(".")).to_owned();
        let config = load_config(config_path)?;
        let mut packages = Vec::new();
        if let Some(workspace) = &config.workspace {
            for member in &workspace.members {
                let dir = root.join(member);
                if !dir.join(CONFIG_FILE).is_file() {
                    return Err(e005(
                        &dir,
                        format!("workspace member `{member}` has no {CONFIG_FILE}"),
                    ));
                }
                packages.push(PackageDir::load(&dir)?);
            }
        } else {
            packages.push(PackageDir::load(&root)?);
        }
        for (name, dep) in &config.dependencies {
            if let Some(path) = &dep.path {
                let dir = root.join(path);
                if !dir.join(CONFIG_FILE).is_file() {
                    return Err(e005(
                        &dir,
                        format!("path dependency `{name}` has no {CONFIG_FILE}"),
                    ));
                }
            }
        }
        let dependencies = load_dependencies(&packages, fetch)?;
        Ok(Self {
            root,
            config,
            packages,
            dependencies,
        })
    }

    /// Indices of the packages a command run in `cwd` applies to: the
    /// member containing `cwd`, unless `cwd` is the workspace root (or
    /// outside every member), which scopes to all packages.
    pub fn scope(&self, cwd: &Path) -> Vec<usize> {
        let all: Vec<usize> = (0..self.packages.len()).collect();
        if self.config.workspace.is_none() || cwd == self.root {
            return all;
        }
        self.packages
            .iter()
            .position(|p| cwd.starts_with(&p.dir))
            .map_or(all, |i| vec![i])
    }
}

/// Build a project diagnostic (E012, E013) for a resolution failure.
fn dep_diag(code: &'static str, path: &Path, message: String) -> ProjectError {
    tracing::warn!(code, path = %path.display(), %message, "project diagnostic");
    ProjectError::Diag(ProjectDiag {
        code,
        message,
        path: path.to_owned(),
        spans: Vec::new(),
    })
}

/// Materialises a git dependency checkout for a url and reference.
pub type GitFetch<'a> = dyn Fn(&str, &GitRef) -> Result<PathBuf, String> + 'a;

/// The already-fetched checkout of `url` at `reference` under `cache` (the `cadetta update`
/// cache); `check` does no network access, so a missing checkout is an error.
// frob:ticket 01M4YRTECA4XNC784V88ZC83KP
fn cached_checkout(cache: &Path, url: &str, reference: &GitRef) -> Result<PathBuf, String> {
    let dir = checkout_dir(cache, url, reference);
    if dir.join(CONFIG_FILE).is_file() {
        tracing::debug!(%url, dir = %dir.display(), "using cached git checkout");
        Ok(dir)
    } else {
        Err(format!("no checkout of {url} in {}", cache.display()))
    }
}

/// Resolve and load the dependency closure of `packages` (packages.md 1).
///
/// Path sources resolve in place; git sources resolve through `fetch`. A git dependency whose
/// checkout is unavailable is skipped with a warning and stays unresolved. E012 and E013 are returned as
/// project diagnostics; a dependency that cannot be found is E005.
fn load_dependencies(
    packages: &[PackageDir],
    fetch: &GitFetch<'_>,
) -> Result<Vec<Dependency>, ProjectError> {
    let mut roots: Vec<(PathBuf, PackageManifest)> = Vec::with_capacity(packages.len());
    for pkg in packages {
        match manifest::load(&pkg.dir) {
            Ok(Manifest::Package(mut m)) => {
                m.dependencies.retain(|d| match &d.source {
                    Source::Path(_) => true,
                    Source::Git { url, reference } => match fetch(url, reference) {
                        Ok(_) => true,
                        Err(reason) => {
                            tracing::warn!(name = %d.name, %reason, "git dependency is not available; run `cadetta update`");
                            false
                        }
                    },
                });
                roots.push((pkg.dir.clone(), m));
            }
            Ok(Manifest::Workspace(_)) => {
                return Err(e005(&pkg.dir, "not a package: found a workspace"));
            }
            Err(e) => return Err(e005(&pkg.dir, e.to_string())),
        }
    }
    if roots.iter().all(|(_, m)| m.dependencies.is_empty()) {
        return Ok(Vec::new());
    }
    let read = |p: &Path| std::fs::read_to_string(p).ok();
    let io = resolve::Io { read: &read, fetch };
    let resolution = match resolve::resolve(&roots, &io) {
        Ok(r) => r,
        Err(e) => {
            let dir = &packages[0].dir;
            return Err(match &e {
                ResolveError::VersionUnsatisfied { .. } => dep_diag("E012", dir, e.to_string()),
                ResolveError::VersionConflict { .. } => dep_diag("E013", dir, e.to_string()),
                _ => e005(dir, e.to_string()),
            });
        }
    };
    let mut out = Vec::with_capacity(resolution.packages.len());
    for resolved in resolution.packages {
        let package = PackageDir::load(&resolved.dir)?;
        tracing::debug!(name = %resolved.name, version = %resolved.version, "dependency loaded");
        out.push(Dependency { resolved, package });
    }
    Ok(out)
}

/// Load a `cadetta.toml`, mapping the error with its path.
fn load_config(path: &Path) -> Result<Config, ProjectError> {
    Config::load(path).map_err(|source| config_error(path.to_owned(), source))
}

/// Map a config failure to a project error: an unloadable package or dependency is E005
/// (packages.md 1), any other config problem is a usage error.
// frob:ticket 01M4YRFMRM5PTVK9TJ9ZRN2H8S
fn config_error(path: PathBuf, source: ConfigError) -> ProjectError {
    match source {
        ConfigError::Unloadable { .. } => e005(&path, source.to_string()),
        source => ProjectError::Config { path, source },
    }
}

impl PackageDir {
    /// Load the package in `dir`: its config and its `.cdt` modules
    /// sorted by path (config-cli.md 1.3); E005 for a missing or empty
    /// `src` (E003 is the resolver's, not the loader's).
    pub fn load(dir: &Path) -> Result<Self, ProjectError> {
        let config = load_config(&dir.join(CONFIG_FILE))?;
        let Some(package) = &config.package else {
            return Err(e005(dir, "not a package: no [package] table"));
        };
        let src = dir.join(&package.src);
        if !src.is_dir() {
            return Err(e005(&src, "package `src` directory is missing"));
        }
        let mut files = Vec::new();
        collect_sources(&src, &src, &mut files)?;
        if files.is_empty() {
            return Err(e005(&src, "package `src` holds no .cdt file"));
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        let mut modules = Vec::with_capacity(files.len());
        for (rel_path, file) in files {
            let source = std::fs::read_to_string(&file).map_err(|source| ProjectError::Io {
                path: file.clone(),
                source,
            })?;
            modules.push(Module {
                rel_path,
                file,
                source,
            });
        }
        tracing::debug!(dir = %dir.display(), modules = modules.len(), "package loaded");
        Ok(Self {
            dir: dir.to_owned(),
            config,
            modules,
        })
    }
}

/// Recursively collect `(relative path, file)` for every `.cdt` under `dir`.
fn collect_sources(
    base: &Path,
    dir: &Path,
    out: &mut Vec<(String, PathBuf)>,
) -> Result<(), ProjectError> {
    let io = |source| ProjectError::Io {
        path: dir.to_owned(),
        source,
    };
    for entry in std::fs::read_dir(dir).map_err(io)? {
        let path = entry.map_err(io)?.path();
        if path.is_dir() {
            collect_sources(base, &path, out)?;
        } else if path.extension().is_some_and(|e| e == SOURCE_EXT) {
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.push((rel, path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    /// A fresh scratch directory under the system temp dir.
    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cadetta-project-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Write `text` to `root/rel`, creating parents.
    fn put(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    const PKG: &str = "[package]\nname = \"acme.a\"\n";

    fn code(err: ProjectError) -> &'static str {
        match err {
            ProjectError::Diag(d) => d.code,
            other => panic!("not a diagnostic: {other}"),
        }
    }

    // frob:tests crates/cadetta-cli/src/project.rs::PackageDir
    #[test]
    fn modules_sorted_by_path() {
        let root = scratch();
        put(&root, "cadetta.toml", PKG);
        put(&root, "src/z.cdt", "");
        put(&root, "src/drive/motor.cdt", "");
        put(&root, "src/a.cdt", "");
        let pkg = PackageDir::load(&root).unwrap();
        let rels: Vec<_> = pkg.modules.iter().map(|m| m.rel_path.as_str()).collect();
        assert_eq!(rels, ["a.cdt", "drive/motor.cdt", "z.cdt"]);
    }

    // frob:tests crates/cadetta-cli/src/project.rs::config_error
    #[test]
    fn reserved_name_and_registry_dependency_are_e005() {
        let root = scratch();
        put(&root, "cadetta.toml", "[package]\nname = \"std\"\n");
        put(&root, "src/a.cdt", "");
        assert_eq!(code(PackageDir::load(&root).unwrap_err()), "E005");
        put(
            &root,
            "cadetta.toml",
            "[package]\nname = \"a.b\"\n[dependencies]\n\"x.y\" = \"1\"\n",
        );
        assert_eq!(code(PackageDir::load(&root).unwrap_err()), "E005");
    }

    // frob:tests crates/cadetta-cli/src/project.rs::PackageDir
    #[test]
    fn missing_or_empty_src_is_e005() {
        let root = scratch();
        put(&root, "cadetta.toml", PKG);
        assert_eq!(code(PackageDir::load(&root).unwrap_err()), "E005");
        put(&root, "src/readme.txt", "");
        assert_eq!(code(PackageDir::load(&root).unwrap_err()), "E005");
    }

    // frob:tests crates/cadetta-cli/src/project.rs::Project
    #[test]
    fn member_and_path_dependency_without_config_is_e005() {
        let root = scratch();
        put(&root, "cadetta.toml", "[workspace]\nmembers = [\"m\"]\n");
        std::fs::create_dir_all(root.join("m")).unwrap();
        assert_eq!(
            code(Project::load(&root.join("cadetta.toml")).unwrap_err()),
            "E005"
        );

        let root = scratch();
        put(
            &root,
            "cadetta.toml",
            "[package]\nname = \"acme.a\"\n[dependencies]\n\"acme.b\" = { path = \"../nowhere\" }\n",
        );
        put(&root, "src/lib.cdt", "");
        assert_eq!(
            code(Project::load(&root.join("cadetta.toml")).unwrap_err()),
            "E005"
        );
    }

    // frob:tests crates/cadetta-cli/src/project.rs::PackageDir
    #[test]
    fn duplicate_component_loads_and_is_left_to_the_resolver() {
        let root = scratch();
        put(&root, "cadetta.toml", PKG);
        put(&root, "src/lib.cdt", "model A { }\nmodel A { }\n");
        assert!(PackageDir::load(&root).is_ok());
    }

    // frob:tests crates/cadetta-cli/src/project.rs::Project
    #[test]
    fn member_scope_unless_at_workspace_root() {
        let root = scratch();
        put(
            &root,
            "cadetta.toml",
            "[workspace]\nmembers = [\"a\", \"b\"]\n",
        );
        for m in ["a", "b"] {
            put(&root, &format!("{m}/cadetta.toml"), PKG);
            put(&root, &format!("{m}/src/lib.cdt"), "");
        }
        // Inside a member: discovery stops at the member's own cadetta.toml.
        let (inner, scope) = Project::open(Some(&root.join("b/src")), None).unwrap();
        assert_eq!(inner.root, root.join("b"));
        assert_eq!(scope, vec![0]);
        // At the workspace root: all members.
        let (ws, scope) = Project::open(Some(&root), None).unwrap();
        assert_eq!(scope, vec![0, 1]);
        // Workspace scoping by cwd inside a member directory.
        assert_eq!(ws.scope(&root.join("b/src")), vec![1]);
    }

    // frob:tests crates/cadetta-cli/src/project.rs::find_config
    #[test]
    fn discovery_walks_up_skips_tableless_and_fails_when_none() {
        let root = scratch();
        put(&root, "cadetta.toml", PKG);
        put(&root, "src/lib.cdt", "");
        put(&root, "src/sub/cadetta.toml", "[lint]\n");
        assert_eq!(
            find_config(&root.join("src/sub")).unwrap(),
            root.join("cadetta.toml")
        );
        let (project, _) = Project::open(None, Some(&root.join("src/sub/cadetta.toml"))).unwrap();
        assert_eq!(project.root, root);
        let empty = scratch();
        assert!(matches!(
            find_config(&empty),
            Err(ProjectError::NotFound { .. })
        ));
    }

    /// A root package depending on `acme.shop` (version `shop_version`) with requirement `req`.
    fn with_shop(shop_version: &str, req: &str) -> PathBuf {
        let root = scratch();
        put(
            &root,
            "cadetta.toml",
            &format!(
                "[package]\nname = \"acme.a\"\n[dependencies]\n\"acme.shop\" = {{ path = \"shop\", version = \"{req}\" }}\n"
            ),
        );
        put(&root, "src/lib.cdt", "");
        put(
            &root,
            "shop/cadetta.toml",
            &format!("[package]\nname = \"acme.shop\"\nversion = \"{shop_version}\"\n"),
        );
        put(&root, "shop/src/lib.cdt", "pub model Thing { }\n");
        root
    }

    // frob:ticket 01M4V6BZ0MPYS6XXSC1992PZ41
    // frob:tests crates/cadetta-cli/src/project.rs::Project
    #[test]
    fn path_dependency_is_resolved_and_loaded() {
        let root = with_shop("1.0.0", "1");
        let project = Project::load(&root.join("cadetta.toml")).unwrap();
        assert_eq!(project.dependencies.len(), 1);
        assert_eq!(project.dependencies[0].resolved.name, "acme.shop");
        assert_eq!(project.dependencies[0].package.modules.len(), 1);
    }

    /// A project depending on a git package `acme.shop`.
    fn with_git_shop() -> PathBuf {
        let root = scratch();
        put(
            &root,
            "cadetta.toml",
            "[package]\nname = \"acme.a\"\n[dependencies]\n\"acme.shop\" = { git = \"https://x/shop.git\", tag = \"v1\" }\n",
        );
        put(&root, "src/lib.cdt", "");
        root
    }

    /// Write the shop package into `dir`.
    fn put_shop(dir: &Path) {
        put(
            dir,
            "cadetta.toml",
            "[package]\nname = \"acme.shop\"\nversion = \"1.0.0\"\n",
        );
        put(dir, "src/lib.cdt", "pub model Thing { }\n");
    }

    // frob:ticket 01M4YRTECA4XNC784V88ZC83KP
    // frob:tests crates/cadetta-cli/src/project.rs::Project
    #[test]
    fn git_dependency_loads_through_the_injected_fetch() {
        let root = with_git_shop();
        let shop = scratch();
        put_shop(&shop);
        let project = Project::load_with(&root.join("cadetta.toml"), &|url, _| {
            assert_eq!(url, "https://x/shop.git");
            Ok(shop.clone())
        })
        .unwrap();
        assert_eq!(project.dependencies.len(), 1);
        assert_eq!(project.dependencies[0].resolved.name, "acme.shop");
        assert_eq!(project.dependencies[0].package.modules.len(), 1);
    }

    // frob:ticket 01M4YRTECA4XNC784V88ZC83KP
    // frob:tests crates/cadetta-cli/src/project.rs::Project
    #[test]
    fn check_loads_a_git_dependency_from_the_update_cache() {
        let root = with_git_shop();
        let reference = GitRef::Tag("v1".to_owned());
        let cache = root.join("build").join("deps");
        let dir = checkout_dir(&cache, "https://x/shop.git", &reference);
        put_shop(&dir);
        let project = Project::load(&root.join("cadetta.toml")).unwrap();
        assert_eq!(project.dependencies.len(), 1);
        assert_eq!(project.dependencies[0].resolved.dir, dir);
    }

    // frob:ticket 01M4YRTECA4XNC784V88ZC83KP
    // frob:tests crates/cadetta-cli/src/project.rs::Project
    #[test]
    fn git_dependency_without_a_checkout_is_skipped() {
        let root = with_git_shop();
        let project = Project::load(&root.join("cadetta.toml")).unwrap();
        assert!(project.dependencies.is_empty());
    }

    // frob:ticket 01M4V6BZ0MPYS6XXSC1992PZ41
    // frob:tests crates/cadetta-cli/src/project.rs::Project
    #[test]
    fn unsatisfied_version_requirement_is_e012() {
        let root = with_shop("1.3.0", "1.2.*");
        assert_eq!(
            code(Project::load(&root.join("cadetta.toml")).unwrap_err()),
            "E012"
        );
    }

    // frob:ticket 01M4V6BZ0MPYS6XXSC1992PZ41
    // frob:tests crates/cadetta-cli/src/project.rs::Project
    #[test]
    fn conflicting_requirements_on_one_package_are_e013() {
        let root = scratch();
        put(
            &root,
            "cadetta.toml",
            "[workspace]\nmembers = [\"a\", \"b\"]\n",
        );
        for (m, req) in [("a", "1"), ("b", "2")] {
            put(
                &root,
                &format!("{m}/cadetta.toml"),
                &format!(
                    "[package]\nname = \"acme.{m}\"\n[dependencies]\n\"acme.shop\" = {{ path = \"../shop\", version = \"{req}\" }}\n"
                ),
            );
            put(&root, &format!("{m}/src/lib.cdt"), "");
        }
        put(
            &root,
            "shop/cadetta.toml",
            "[package]\nname = \"acme.shop\"\nversion = \"1.0.0\"\n",
        );
        put(&root, "shop/src/lib.cdt", "pub model Thing { }\n");
        assert_eq!(
            code(Project::load(&root.join("cadetta.toml")).unwrap_err()),
            "E013"
        );
    }
}
