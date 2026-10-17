//! frob:ticket 01M4SXQWGAYJDGMGKNBMHASNBA
//! The package manifest and workspace model (packages.md 1, 2, 6;
//! config-cli.md 1.3, 2).
//!
//! A [`Manifest`] is a validated `cadetta.toml` re-expressed in typed
//! form: dependency sources are an enum, trust roles are parsed, `std`
//! is the implicit dependency, and a workspace knows its members. The
//! schema checks live in [`crate::config`]; this layer adds what needs
//! the file system (members, package roots) and the rules that make a
//! registry source unsupported (OQ-19).

use std::path::{Path, PathBuf};

use crate::config::{Config, ConfigError, Dependency, Workspace, normalise_package_name};

/// The module root file inside `src` (packages.md 2).
pub const ROOT_MODULE: &str = "lib.cdt";

/// The implicit standard-library dependency name (packages.md 1).
pub const STD_NAME: &str = "std";

/// Why a manifest could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The `cadetta.toml` itself is invalid (config-cli.md exit code 2).
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// A dependency names a registry source, which is OQ-19.
    #[error(
        "dependency `{name}` has no `path` or `git` source; registry sources are unsupported (OQ-19)"
    )]
    RegistryUnsupported {
        /// The dependency name.
        name: String,
    },
    /// A workspace member has no `cadetta.toml` (E005).
    #[error("E005 missing-package: workspace member `{member}` has no cadetta.toml at {}", path.display())]
    MissingMember {
        /// The member as written in `[workspace] members`.
        member: String,
        /// Where its `cadetta.toml` was expected.
        path: PathBuf,
    },
    /// A workspace member is itself a workspace, or has no `[package]`.
    #[error("workspace member `{member}` must be a package, not a workspace")]
    MemberNotPackage {
        /// The member as written in `[workspace] members`.
        member: String,
    },
}

/// Where a dependency comes from (packages.md 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A local directory, relative to the declaring package.
    Path(PathBuf),
    /// A git repository at a pinned or moving reference.
    Git {
        /// Repository URL.
        url: String,
        /// Which reference to check out.
        reference: GitRef,
    },
}

/// The git reference of a [`Source::Git`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitRef {
    /// A tag.
    Tag(String),
    /// A commit.
    Rev(String),
    /// A branch.
    Branch(String),
    /// None given: the remote's default branch.
    Default,
}

/// One typed dependency of a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dep {
    /// The dependency's package name.
    pub name: String,
    /// Where it comes from.
    pub source: Source,
    /// Semver requirement the resolved package must meet (E012).
    pub version: Option<String>,
    /// Roles whose upstream attestations are trusted (packages.md 6).
    pub trust: Vec<String>,
}

/// A package manifest: `[package]` plus `[dependencies]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifest {
    /// Lower dotted package name.
    pub name: String,
    /// Semver version, when declared.
    pub version: Option<String>,
    /// Module root directory, relative to the package directory.
    pub src: PathBuf,
    /// Explicit dependencies in name order.
    pub dependencies: Vec<Dep>,
    /// Version of the implicit `std` dependency: the compiler's own.
    pub std_version: &'static str,
}

impl PackageManifest {
    /// The package root module file, `<src>/lib.cdt`, under `dir`.
    pub fn root_module(&self, dir: &Path) -> PathBuf {
        dir.join(&self.src).join(ROOT_MODULE)
    }
}

/// A workspace root and its members (packages.md 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceManifest {
    /// The workspace root directory.
    pub root: PathBuf,
    /// Members in declared order, each loaded from its own `cadetta.toml`.
    pub members: Vec<Member>,
}

impl WorkspaceManifest {
    /// The one lock all members share.
    pub fn lock_path(&self) -> PathBuf {
        self.root.join("cadetta.lock")
    }

    /// The one build directory all members share.
    pub fn build_dir(&self) -> PathBuf {
        self.root.join("build")
    }
}

/// One loaded workspace member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The member directory as written in `members`.
    pub dir: String,
    /// Its manifest.
    pub package: PackageManifest,
}

/// A loaded `cadetta.toml`: a package or a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Manifest {
    /// A package root.
    Package(PackageManifest),
    /// A workspace root.
    Workspace(WorkspaceManifest),
}

/// Convert one validated [`Dependency`] to its typed form.
fn typed_dep(name: &str, dep: &Dependency) -> Dep {
    let source = match (&dep.path, &dep.git) {
        (Some(path), _) => Source::Path(PathBuf::from(path)),
        (None, Some(url)) => Source::Git {
            url: url.clone(),
            reference: match (&dep.tag, &dep.rev, &dep.branch) {
                (Some(t), _, _) => GitRef::Tag(t.clone()),
                (_, Some(r), _) => GitRef::Rev(r.clone()),
                (_, _, Some(b)) => GitRef::Branch(b.clone()),
                _ => GitRef::Default,
            },
        },
        // Config validation guarantees one of path/git; unreachable in practice.
        (None, None) => unreachable!("config validation requires a source"),
    };
    Dep {
        name: name.to_owned(),
        source,
        version: dep.version.clone(),
        trust: dep.trust.clone(),
    }
}

/// Reject registry dependencies (no `path` or `git`) before schema validation.
fn reject_registry(text: &str) -> Result<(), ManifestError> {
    let Ok(table) = text.parse::<toml::Table>() else {
        return Ok(()); // the config parser reports the syntax error
    };
    let Some(deps) = table.get("dependencies").and_then(|d| d.as_table()) else {
        return Ok(());
    };
    for (name, value) in deps {
        let sourced = value
            .as_table()
            .is_some_and(|t| t.contains_key("path") || t.contains_key("git"));
        if !sourced {
            tracing::warn!(%name, "registry dependency source is unsupported (OQ-19)");
            return Err(ManifestError::RegistryUnsupported { name: name.clone() });
        }
    }
    Ok(())
}

impl PackageManifest {
    /// Build from a validated config whose `[package]` table is present.
    fn from_config(config: &Config) -> Option<Self> {
        let package = config.package.as_ref()?;
        Some(Self {
            name: package.name.clone().unwrap_or_default(),
            version: package.version.clone(),
            src: PathBuf::from(&package.src),
            dependencies: config
                .dependencies
                .iter()
                .map(|(n, d)| typed_dep(n, d))
                .collect(),
            std_version: env!("CARGO_PKG_VERSION"),
        })
    }
}

/// Parse manifest `text` without touching the file system (members are
/// not checked); an absent package name stays empty.
pub fn parse(text: &str) -> Result<Manifest, ManifestError> {
    reject_registry(text)?;
    let config = Config::parse(text)?;
    if let Some(package) = PackageManifest::from_config(&config) {
        tracing::debug!(name = %package.name, deps = package.dependencies.len(), "parsed package manifest");
        return Ok(Manifest::Package(package));
    }
    let Workspace { members } = config.workspace.unwrap_or(Workspace { members: vec![] });
    tracing::debug!(members = members.len(), "parsed workspace manifest");
    Ok(Manifest::Workspace(WorkspaceManifest {
        root: PathBuf::new(),
        members: members
            .into_iter()
            .map(|dir| Member {
                dir,
                package: PackageManifest {
                    name: String::new(),
                    version: None,
                    src: PathBuf::from("src"),
                    dependencies: vec![],
                    std_version: env!("CARGO_PKG_VERSION"),
                },
            })
            .collect(),
    }))
}

/// Load `<dir>/cadetta.toml`; for a workspace, also load every member's
/// manifest, where a member without one is E005.
pub fn load(dir: &Path) -> Result<Manifest, ManifestError> {
    load_with(dir, &|path| std::fs::read_to_string(path).ok())
}

/// [`load`] over an injected reader: `read` returns a file's text, or
/// `None` when it is absent or unreadable.
pub fn load_with(
    dir: &Path,
    read: &dyn Fn(&Path) -> Option<String>,
) -> Result<Manifest, ManifestError> {
    let path = dir.join("cadetta.toml");
    tracing::info!(path = %path.display(), "loading manifest");
    let text = read(&path).ok_or_else(|| ConfigError::Read {
        path: path.clone(),
        source: std::io::Error::from(std::io::ErrorKind::NotFound),
    })?;
    reject_registry(&text)?;
    let mut config = Config::parse(&text)?;
    if let Some(ws) = config.workspace.take() {
        let mut members = Vec::with_capacity(ws.members.len());
        for member in ws.members {
            let member_dir = dir.join(&member);
            let member_toml = member_dir.join("cadetta.toml");
            if read(&member_toml).is_none() {
                tracing::error!(%member, path = %member_toml.display(), "E005 workspace member has no cadetta.toml");
                return Err(ManifestError::MissingMember {
                    member,
                    path: member_toml,
                });
            }
            match load_with(&member_dir, read)? {
                Manifest::Package(package) => members.push(Member {
                    dir: member,
                    package,
                }),
                Manifest::Workspace(_) => {
                    return Err(ManifestError::MemberNotPackage { member });
                }
            }
        }
        tracing::info!(members = members.len(), "loaded workspace");
        return Ok(Manifest::Workspace(WorkspaceManifest {
            root: dir.to_owned(),
            members,
        }));
    }
    let mut package = PackageManifest::from_config(&config).ok_or(ConfigError::NoProject)?;
    if package.name.is_empty() {
        let name = default_name(dir);
        tracing::info!(%name, "package name defaults to the directory name");
        package.name = name;
    }
    Ok(Manifest::Package(package))
}

/// The normalised directory name used when `[package] name` is absent.
fn default_name(dir: &Path) -> String {
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_owned());
    normalise_package_name(dir.file_name().and_then(|n| n.to_str()).unwrap_or(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(text: &str) -> PackageManifest {
        match parse(text).expect("valid") {
            Manifest::Package(p) => p,
            Manifest::Workspace(_) => panic!("expected a package"),
        }
    }

    // frob:tests crates/cadetta-cli/src/pkg/manifest.rs::parse
    #[test]
    fn package_fields_default_and_root_module() {
        let p = pkg("[package]\nname = \"acme.rover\"\nversion = \"0.3.0\"\n");
        assert_eq!(p.name, "acme.rover");
        assert_eq!(p.version.as_deref(), Some("0.3.0"));
        assert_eq!(p.src, PathBuf::from("src"));
        assert_eq!(
            p.root_module(Path::new("/x")),
            PathBuf::from("/x/src/lib.cdt")
        );
        for bad in [
            "[package]\nname = \"Acme.Rover\"\n",
            "[package]\nname = \"acme\"\nversion = \"1.0\"\n",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }

    // frob:tests crates/cadetta-cli/src/pkg/manifest.rs::reject_registry
    #[test]
    fn dependency_sources_are_typed_and_registry_rejected() {
        let p = pkg(
            "[package]\nname = \"a\"\n[dependencies]\n\"b.p\" = { path = \"../p\", version = \"1\" }\n\"b.g\" = { git = \"https://x/y.git\", tag = \"v2\" }\n\"b.r\" = { git = \"https://x/z.git\", rev = \"abc\" }\n",
        );
        assert_eq!(p.dependencies.len(), 3);
        assert_eq!(
            p.dependencies[0].source,
            Source::Git {
                url: "https://x/y.git".into(),
                reference: GitRef::Tag("v2".into())
            }
        );
        assert_eq!(p.dependencies[1].source, Source::Path("../p".into()));
        assert_eq!(p.dependencies[1].version.as_deref(), Some("1"));
        let err = parse("[package]\nname = \"a\"\n[dependencies]\n\"b.p\" = \"1\"\n").unwrap_err();
        assert!(matches!(err, ManifestError::RegistryUnsupported { .. }));
        let err = parse("[package]\nname = \"a\"\n[dependencies]\n\"b.p\" = { version = \"1\" }\n")
            .unwrap_err();
        assert!(matches!(err, ManifestError::RegistryUnsupported { .. }));
    }

    // frob:ticket 01M4STANV9T55FV8529E2RPFB1
    // frob:tests crates/cadetta-cli/src/pkg/manifest.rs::PackageManifest
    #[test]
    fn std_ships_as_an_ordinary_package_at_0_1_0() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
        let text = std::fs::read_to_string(root.join("cadetta.toml")).unwrap();
        let value: toml::Value = toml::from_str(&text).unwrap();
        let package = value.get("package").and_then(|p| p.as_table()).unwrap();
        assert_eq!(package["name"].as_str(), Some(STD_NAME));
        // 0.2.0 added the inch fasteners (~HP6PTJH); the test name keeps its bound evidence ref.
        assert_eq!(package["version"].as_str(), Some("0.2.0"));
        let src = package["src"].as_str().unwrap();
        assert!(root.join(src).join("lib.cdt").is_file());
        assert!(value.get("dependencies").is_none());
        // The same name stays reserved for user packages.
        assert!(parse(&text).is_err());
    }

    // frob:tests crates/cadetta-cli/src/pkg/manifest.rs::PackageManifest
    #[test]
    fn std_is_reserved_and_implicit() {
        assert!(parse("[package]\nname = \"std\"\n").is_err());
        assert!(
            parse("[package]\nname = \"a\"\n[dependencies]\nstd = { path = \"x\" }\n").is_err()
        );
        assert_eq!(
            pkg("[package]\nname = \"a\"\n").std_version,
            env!("CARGO_PKG_VERSION")
        );
    }

    // frob:tests crates/cadetta-cli/src/pkg/manifest.rs::load_with
    #[test]
    fn workspace_members_load_and_missing_member_is_e005() {
        let mut files: std::collections::BTreeMap<PathBuf, String> = [
            ("ws/cadetta.toml", "[workspace]\nmembers = [\"rover\", \"shop\"]\n"),
            (
                "ws/rover/cadetta.toml",
                "[package]\nname = \"acme.rover\"\n[dependencies]\n\"acme.shop\" = { path = \"../shop\" }\n",
            ),
            ("ws/shop/cadetta.toml", "[package]\nname = \"acme.shop\"\n"),
        ]
        .into_iter()
        .map(|(p, t)| (PathBuf::from(p), t.to_owned()))
        .collect();
        let dir = Path::new("ws");
        let Manifest::Workspace(ws) = load_with(dir, &|p| files.get(p).cloned()).expect("loads")
        else {
            panic!("expected workspace");
        };
        assert_eq!(ws.members.len(), 2);
        assert_eq!(ws.members[0].package.dependencies[0].name, "acme.shop");
        assert_eq!(ws.lock_path(), dir.join("cadetta.lock"));
        assert_eq!(ws.build_dir(), dir.join("build"));

        files.remove(Path::new("ws/shop/cadetta.toml"));
        let err = load_with(dir, &|p| files.get(p).cloned()).unwrap_err();
        assert!(matches!(err, ManifestError::MissingMember { .. }));
        assert!(err.to_string().contains("E005"));
    }

    // frob:tests crates/cadetta-cli/src/pkg/manifest.rs::Dep
    #[test]
    fn trust_roles_are_parsed() {
        let p = pkg(
            "[package]\nname = \"a\"\n[dependencies.\"acme.shop\"]\npath = \"../shop\"\ntrust = [\"source\", \"tolerance_review\"]\n",
        );
        assert_eq!(p.dependencies[0].trust, vec!["source", "tolerance_review"]);
    }

    // frob:tests crates/cadetta-cli/src/pkg/manifest.rs::load
    #[test]
    fn load_reports_an_absent_manifest() {
        let err = load(Path::new("/nonexistent/cadetta-dir")).unwrap_err();
        assert!(matches!(
            err,
            ManifestError::Config(ConfigError::Read { .. })
        ));
    }
}
