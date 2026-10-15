//! frob:ticket 01M4SXR2FQZ6DZDY01VF34J1C5
//! Dependency resolution from path and git sources (packages.md 1, 7;
//! config-cli.md 1.3).
//!
//! Resolution walks the dependency graph breadth first from the root
//! packages, in declared (name) order, and yields exactly one version of
//! each package. The first requirement that reaches a package selects
//! its source; later requirements are checked against that selection
//! (E012 when it is the only requirement, E013 when several cannot be
//! met together). No second copy is ever loaded into the result.
//!
//! All file and git access is injected through [`Io`] so the resolver
//! does no I/O of its own (CAP001) and tests need no network.

use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};
use std::path::{Component, Path, PathBuf};

use crate::config::ConfigError;
use crate::pkg::manifest::{self, Dep, GitRef, Manifest, ManifestError, PackageManifest, Source};

/// Injected effects: reading files and materialising a git checkout.
pub struct Io<'a> {
    /// Returns a file's text, or `None` when absent or unreadable.
    pub read: &'a dyn Fn(&Path) -> Option<String>,
    /// Checks out `url` at a reference and returns the checkout directory.
    pub fetch: &'a dyn Fn(&str, &GitRef) -> Result<PathBuf, String>,
}

/// Why resolution failed.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// A dependency has no package directory with a `cadetta.toml` (E005).
    #[error("E005 missing-package: dependency `{name}` ({wanted}) has no cadetta.toml: {reason}")]
    MissingPackage {
        /// Dependency name.
        name: String,
        /// The source as written.
        wanted: String,
        /// What went wrong.
        reason: String,
    },
    /// A resolved package does not meet its `version` requirement (E012).
    #[error("E012 version-unsatisfied: `{name}` resolved to {found}, which does not meet `{req}`")]
    VersionUnsatisfied {
        /// Dependency name.
        name: String,
        /// The resolved version (or `none`).
        found: String,
        /// The requirement.
        req: String,
    },
    /// No single version meets every requirement on a package (E013).
    #[error(
        "E013 version-conflict: `{name}` resolved to {found}, which fails `{req}` required by `{by}`"
    )]
    VersionConflict {
        /// Dependency name.
        name: String,
        /// The one selected version.
        found: String,
        /// The requirement that cannot be met alongside the others.
        req: String,
        /// The package that imposed it.
        by: String,
    },
    /// A version or requirement is not valid semver.
    #[error("invalid semver {text:?} for `{name}`: {reason}")]
    BadSemver {
        /// Package or dependency name.
        name: String,
        /// The offending text.
        text: String,
        /// What is wrong with it.
        reason: String,
    },
    /// A dependency's manifest is invalid.
    #[error("dependency `{name}`: {source}")]
    Manifest {
        /// Dependency name.
        name: String,
        /// The manifest error.
        source: ManifestError,
    },
}

/// A resolved package: one per name for the whole project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackage {
    /// Package name.
    pub name: String,
    /// Declared version (`0.0.0` when the manifest declares none).
    pub version: String,
    /// Where it came from, with relative paths normalised.
    pub source: Source,
    /// Its directory.
    pub dir: PathBuf,
    /// Its manifest.
    pub manifest: PackageManifest,
}

/// The one-version-per-package result, ordered by package name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolution {
    /// Every non-root package in the project's dependency closure.
    pub packages: Vec<ResolvedPackage>,
}

impl Resolution {
    /// The resolved package called `name`, if any.
    pub fn get(&self, name: &str) -> Option<&ResolvedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }
}

/// A parsed `major.minor.patch[-pre]` version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// Major.
    pub major: u64,
    /// Minor.
    pub minor: u64,
    /// Patch.
    pub patch: u64,
    /// Pre-release tag, if any.
    pub pre: Option<String>,
}

impl Version {
    /// Parse a full semver version; build metadata is ignored.
    pub fn parse(text: &str) -> Result<Self, String> {
        let core = text.split('+').next().unwrap_or("");
        let (nums, pre) = match core.split_once('-') {
            Some((n, p)) if !p.is_empty() => (n, Some(p.to_owned())),
            Some(_) => return Err("empty pre-release".into()),
            None => (core, None),
        };
        let parts: Vec<&str> = nums.split('.').collect();
        let [a, b, c] = parts[..] else {
            return Err("expected major.minor.patch".into());
        };
        let num = |s: &str| s.parse::<u64>().map_err(|_| format!("bad number {s:?}"));
        Ok(Self {
            major: num(a)?,
            minor: num(b)?,
            patch: num(c)?,
            pre,
        })
    }

    fn triple(&self) -> (u64, u64, u64) {
        (self.major, self.minor, self.patch)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.triple()
            .cmp(&other.triple())
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (Some(a), Some(b)) => a.cmp(b),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// One bound pair: `lower <= v < upper` (either side optional).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Range {
    lower: Option<(u64, u64, u64)>,
    upper: Option<(u64, u64, u64)>,
}

/// A semver requirement: comparators joined by commas, all of which
/// must hold (`^`, `~`, `=`, `>`, `>=`, `<`, `<=`, wildcards; a bare
/// version is caret).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq {
    ranges: Vec<Range>,
}

/// Parse a possibly partial version: `1`, `1.2`, `1.2.3`, `1.*`, `*`.
fn partial(text: &str) -> Result<Vec<u64>, String> {
    let mut out = Vec::new();
    let mut wild = false;
    for (i, part) in text.split('.').enumerate() {
        if i > 2 {
            return Err("too many components".into());
        }
        if matches!(part, "*" | "x" | "X") {
            wild = true;
        } else if wild {
            return Err("number after a wildcard".into());
        } else {
            out.push(
                part.parse::<u64>()
                    .map_err(|_| format!("bad number {part:?}"))?,
            );
        }
    }
    Ok(out)
}

fn t(p: &[u64]) -> (u64, u64, u64) {
    (
        p.first().copied().unwrap_or(0),
        p.get(1).copied().unwrap_or(0),
        p.get(2).copied().unwrap_or(0),
    )
}

/// First version above everything the partial version `p` covers.
fn next_after(p: &[u64]) -> Option<(u64, u64, u64)> {
    match p.len() {
        0 => None,
        1 => Some((p[0] + 1, 0, 0)),
        2 => Some((p[0], p[1] + 1, 0)),
        _ => Some((p[0], p[1], p[2] + 1)),
    }
}

impl VersionReq {
    /// Parse a requirement such as `1`, `^1.2`, `>=1.0, <2`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut ranges = Vec::new();
        for raw in text.split(',') {
            let raw = raw.trim();
            if raw.is_empty() {
                return Err("empty comparator".into());
            }
            let (op, rest) = ["<=", ">=", "<", ">", "=", "~", "^"]
                .iter()
                .find_map(|op| raw.strip_prefix(op).map(|r| (*op, r.trim())))
                .unwrap_or(("^", raw));
            // A bare wildcard (`1.2.*`) pins the numbers before it, like `=`.
            let op = if op == "^"
                && raw == rest
                && rest.split('.').any(|c| matches!(c, "*" | "x" | "X"))
            {
                "="
            } else {
                op
            };
            let rest = rest.split(['-', '+']).next().unwrap_or("");
            let p = partial(rest)?;
            let range = match op {
                "=" => Range {
                    lower: (!p.is_empty()).then(|| t(&p)),
                    upper: next_after(&p),
                },
                ">=" => Range {
                    lower: Some(t(&p)),
                    upper: None,
                },
                ">" => Range {
                    lower: next_after(&p).or(Some((u64::MAX, 0, 0))),
                    upper: None,
                },
                "<" => Range {
                    lower: None,
                    upper: Some(t(&p)),
                },
                "<=" => Range {
                    lower: None,
                    upper: next_after(&p),
                },
                "~" => Range {
                    lower: Some(t(&p)),
                    upper: match p.len() {
                        0 => None,
                        1 => Some((p[0] + 1, 0, 0)),
                        _ => Some((p[0], p[1] + 1, 0)),
                    },
                },
                _ => Range {
                    lower: Some(t(&p)),
                    upper: match p.as_slice() {
                        [] => None,
                        [0] => Some((1, 0, 0)),
                        [0, 0] => Some((0, 1, 0)),
                        [0, 0, z] => Some((0, 0, z + 1)),
                        [0, y, ..] => Some((0, y + 1, 0)),
                        [x, ..] => Some((x + 1, 0, 0)),
                    },
                },
            };
            ranges.push(range);
        }
        Ok(Self { ranges })
    }

    /// Whether `v` meets every comparator; pre-releases never match.
    pub fn matches(&self, v: &Version) -> bool {
        v.pre.is_none()
            && self.ranges.iter().all(|r| {
                r.lower.is_none_or(|l| v.triple() >= l) && r.upper.is_none_or(|u| v.triple() < u)
            })
    }
}

/// Collapse `.` and `..` lexically, since no file system is consulted.
fn normalise(path: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir if matches!(out.last(), Some(Component::Normal(_))) => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out.iter().collect()
}

fn describe(source: &Source) -> String {
    match source {
        Source::Path(p) => format!("path {}", p.display()),
        Source::Git { url, reference } => format!("git {url} {reference:?}"),
    }
}

/// Load the package behind `dep`, declared by a package living in `base`.
fn load_candidate(
    dep: &Dep,
    base: &Path,
    io: &Io<'_>,
) -> Result<(PathBuf, Source, PackageManifest), ResolveError> {
    let missing = |reason: String| ResolveError::MissingPackage {
        name: dep.name.clone(),
        wanted: describe(&dep.source),
        reason,
    };
    let (dir, source) = match &dep.source {
        Source::Path(p) => {
            let dir = normalise(&base.join(p));
            (dir.clone(), Source::Path(dir))
        }
        Source::Git { url, reference } => {
            let dir = (io.fetch)(url, reference).map_err(missing)?;
            (dir, dep.source.clone())
        }
    };
    match manifest::load_with(&dir, io.read) {
        Ok(Manifest::Package(pkg)) => Ok((dir, source, pkg)),
        Ok(Manifest::Workspace(_)) => Err(missing("it is a workspace, not a package".into())),
        Err(ManifestError::Config(ConfigError::Read { path, .. })) => {
            tracing::error!(name = %dep.name, path = %path.display(), "E005 dependency has no cadetta.toml");
            Err(missing(format!("{} not found", path.display())))
        }
        Err(source) => Err(ResolveError::Manifest {
            name: dep.name.clone(),
            source,
        }),
    }
}

fn parse_version(name: &str, v: Option<&str>) -> Result<Option<Version>, ResolveError> {
    v.map(|text| {
        Version::parse(text).map_err(|reason| ResolveError::BadSemver {
            name: name.to_owned(),
            text: text.to_owned(),
            reason,
        })
    })
    .transpose()
}

fn parse_req(name: &str, text: &str) -> Result<VersionReq, ResolveError> {
    VersionReq::parse(text).map_err(|reason| ResolveError::BadSemver {
        name: name.to_owned(),
        text: text.to_owned(),
        reason,
    })
}

/// Resolve the dependencies of `roots` (package directory and manifest,
/// e.g. one package or every workspace member) to one version each.
pub fn resolve(
    roots: &[(PathBuf, PackageManifest)],
    io: &Io<'_>,
) -> Result<Resolution, ResolveError> {
    // name -> (selected version, requirement count)
    let mut chosen: BTreeMap<String, (Option<Version>, usize, ResolvedPackage)> = BTreeMap::new();
    let mut root_names: BTreeMap<String, Source> = BTreeMap::new();
    let mut queue: VecDeque<(String, PathBuf, Dep)> = VecDeque::new();
    for (dir, pkg) in roots {
        root_names.insert(pkg.name.clone(), Source::Path(normalise(dir)));
    }
    for (dir, pkg) in roots {
        for dep in &pkg.dependencies {
            queue.push_back((pkg.name.clone(), dir.clone(), dep.clone()));
        }
    }
    while let Some((by, base, dep)) = queue.pop_front() {
        let req = dep
            .version
            .as_deref()
            .map(|r| parse_req(&dep.name, r))
            .transpose()?;
        // A dependency on a root (workspace sibling) is already loaded.
        let (dir, source, pkg) = if let Some(src) = root_names.get(&dep.name) {
            let (dir, pkg) = roots
                .iter()
                .find(|(_, p)| p.name == dep.name)
                .expect("root name came from roots");
            tracing::debug!(name = %dep.name, "dependency satisfied by a root package");
            (dir.clone(), src.clone(), pkg.clone())
        } else {
            load_candidate(&dep, &base, io)?
        };
        let version = parse_version(&dep.name, pkg.version.as_deref())?;
        let shown = pkg.version.clone().unwrap_or_else(|| "none".into());
        let seen = chosen.get_mut(&dep.name);
        match seen {
            None => {
                if let Some(req) = &req {
                    let ok = version.as_ref().is_some_and(|v| req.matches(v));
                    if !ok {
                        tracing::error!(name = %dep.name, found = %shown, "E012 version unsatisfied");
                        return Err(ResolveError::VersionUnsatisfied {
                            name: dep.name,
                            found: shown,
                            req: dep.version.unwrap_or_default(),
                        });
                    }
                }
                tracing::info!(name = %dep.name, version = %shown, by = %by, "selected dependency");
                for sub in &pkg.dependencies {
                    queue.push_back((pkg.name.clone(), dir.clone(), sub.clone()));
                }
                let resolved = ResolvedPackage {
                    name: dep.name.clone(),
                    version: version
                        .as_ref()
                        .map_or_else(|| "0.0.0".into(), |_| shown.clone()),
                    source,
                    dir,
                    manifest: pkg,
                };
                chosen.insert(dep.name, (version, 1, resolved));
            }
            Some((selected, count, resolved)) => {
                *count += 1;
                if let Some(req) = &req {
                    let ok = selected.as_ref().is_some_and(|v| req.matches(v));
                    if !ok {
                        tracing::error!(name = %dep.name, found = %resolved.version, %by, "E013 version conflict");
                        return Err(ResolveError::VersionConflict {
                            name: dep.name,
                            found: resolved.version.clone(),
                            req: dep.version.unwrap_or_default(),
                            by,
                        });
                    }
                }
                if resolved.source != source {
                    tracing::warn!(name = %dep.name, kept = %describe(&resolved.source), ignored = %describe(&source), "second source for a package ignored; one version per package");
                }
            }
        }
    }
    let packages = chosen.into_values().map(|(_, _, p)| p).collect();
    Ok(Resolution { packages })
}

#[cfg(test)]
mod tests {
    use super::*;

    type Files = BTreeMap<PathBuf, String>;

    fn files(list: &[(&str, &str)]) -> Files {
        list.iter()
            .map(|(p, t)| (PathBuf::from(p), (*t).to_owned()))
            .collect()
    }

    fn pkg_toml(name: &str, version: &str, deps: &str) -> String {
        format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n[dependencies]\n{deps}")
    }

    /// Fake git: checkouts live at `/git/<last url segment>/<ref>`.
    fn fake_fetch(url: &str, r: &GitRef) -> Result<PathBuf, String> {
        let name = url
            .rsplit('/')
            .next()
            .unwrap_or(url)
            .trim_end_matches(".git");
        let tag = match r {
            GitRef::Tag(s) | GitRef::Rev(s) | GitRef::Branch(s) => s.as_str(),
            GitRef::Default => "default",
        };
        if name == "gone" {
            return Err("repository not found".into());
        }
        Ok(PathBuf::from(format!("/git/{name}/{tag}")))
    }

    fn run(files: &Files, root_deps: &str) -> Result<Resolution, ResolveError> {
        let read = |p: &Path| files.get(p).cloned();
        let io = Io {
            read: &read,
            fetch: &fake_fetch,
        };
        let text = pkg_toml("app", "1.0.0", root_deps);
        let Manifest::Package(root) = manifest::parse(&text).expect("root") else {
            panic!("package")
        };
        resolve(&[(PathBuf::from("/w/app"), root)], &io)
    }

    // frob:tests crates/cadetta-cli/src/pkg/resolve.rs::resolve
    #[test]
    fn path_and_git_sources_resolve_and_missing_is_e005() {
        let f = files(&[
            ("/w/shop/cadetta.toml", &pkg_toml("shop", "1.2.0", "")),
            (
                "/git/parts/v2/cadetta.toml",
                &pkg_toml("parts", "2.0.0", ""),
            ),
            ("/git/rv/abc123/cadetta.toml", &pkg_toml("rv", "0.1.0", "")),
            ("/git/br/main/cadetta.toml", &pkg_toml("br", "0.1.0", "")),
        ]);
        let r = run(
            &f,
            "shop = { path = \"../shop\", version = \"1\" }\nparts = { git = \"https://x/parts.git\", tag = \"v2\" }\nrv = { git = \"https://x/rv.git\", rev = \"abc123\" }\nbr = { git = \"https://x/br.git\", branch = \"main\" }\n",
        )
        .expect("resolves");
        let names: Vec<_> = r.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["br", "parts", "rv", "shop"]);
        assert_eq!(r.get("shop").unwrap().dir, PathBuf::from("/w/shop"));
        assert_eq!(r.get("parts").unwrap().dir, PathBuf::from("/git/parts/v2"));

        for bad in [
            "nope = { path = \"../nope\" }\n",
            "gone = { git = \"https://x/gone.git\", tag = \"v1\" }\n",
            "parts = { git = \"https://x/parts.git\", tag = \"v9\" }\n",
        ] {
            let err = run(&f, bad).unwrap_err();
            assert!(matches!(err, ResolveError::MissingPackage { .. }), "{bad}");
            assert!(err.to_string().contains("E005"));
        }
    }

    // frob:tests crates/cadetta-cli/src/pkg/resolve.rs::resolve
    #[test]
    fn unmet_version_requirement_is_e012() {
        let f = files(&[("/w/shop/cadetta.toml", &pkg_toml("shop", "1.2.0", ""))]);
        let err = run(&f, "shop = { path = \"../shop\", version = \"2\" }\n").unwrap_err();
        assert!(matches!(err, ResolveError::VersionUnsatisfied { .. }));
        assert!(err.to_string().contains("E012"));
        assert!(
            run(
                &f,
                "shop = { path = \"../shop\", version = \">=1.2, <1.3\" }\n"
            )
            .is_ok()
        );
        let err = run(&f, "shop = { path = \"../shop\", version = \"x.y\" }\n").unwrap_err();
        assert!(matches!(err, ResolveError::BadSemver { .. }));
    }

    // frob:tests crates/cadetta-cli/src/pkg/resolve.rs::resolve
    #[test]
    fn irreconcilable_requirements_are_e013_with_no_second_copy() {
        let f = files(&[
            (
                "/w/mid/cadetta.toml",
                &pkg_toml(
                    "mid",
                    "1.0.0",
                    "shop = { path = \"../shop2\", version = \"2\" }\n",
                ),
            ),
            ("/w/shop/cadetta.toml", &pkg_toml("shop", "1.2.0", "")),
            ("/w/shop2/cadetta.toml", &pkg_toml("shop", "2.0.0", "")),
        ]);
        let err = run(
            &f,
            "shop = { path = \"../shop\", version = \"1\" }\nmid = { path = \"../mid\" }\n",
        )
        .unwrap_err();
        assert!(matches!(err, ResolveError::VersionConflict { .. }));
        assert!(err.to_string().contains("E013"));

        // Compatible requirements on one package resolve to one copy.
        let f = files(&[
            (
                "/w/mid/cadetta.toml",
                &pkg_toml(
                    "mid",
                    "1.0.0",
                    "shop = { path = \"../shop\", version = \"^1.1\" }\n",
                ),
            ),
            ("/w/shop/cadetta.toml", &pkg_toml("shop", "1.2.0", "")),
        ]);
        let r = run(
            &f,
            "shop = { path = \"../shop\", version = \"1\" }\nmid = { path = \"../mid\" }\n",
        )
        .expect("compatible");
        assert_eq!(r.packages.len(), 2);
    }

    // frob:tests crates/cadetta-cli/src/pkg/resolve.rs::resolve
    #[test]
    fn transitive_dependencies_resolve_once_in_name_order() {
        let f = files(&[
            (
                "/w/zeta/cadetta.toml",
                &pkg_toml(
                    "zeta",
                    "1.0.0",
                    "base = { path = \"../base\", version = \"^1.1\" }\n",
                ),
            ),
            (
                "/w/alpha/cadetta.toml",
                &pkg_toml(
                    "alpha",
                    "0.4.0",
                    "base = { path = \"../base/\", version = \"1\" }\n",
                ),
            ),
            ("/w/base/cadetta.toml", &pkg_toml("base", "1.3.0", "")),
        ]);
        let r = run(
            &f,
            "zeta = { path = \"../zeta\" }\nalpha = { path = \"../alpha\" }\n",
        )
        .expect("resolves");
        let names: Vec<_> = r.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["alpha", "base", "zeta"]);
        assert_eq!(r.get("base").unwrap().version, "1.3.0");
        assert_eq!(
            r.get("base").unwrap().source,
            Source::Path("/w/base".into())
        );
    }

    // frob:tests crates/cadetta-cli/src/pkg/resolve.rs::VersionReq
    #[test]
    fn version_requirements_follow_semver() {
        let v = |s: &str| Version::parse(s).unwrap();
        let ok = |req: &str, ver: &str| VersionReq::parse(req).unwrap().matches(&v(ver));
        assert!(ok("1", "1.9.0") && !ok("1", "2.0.0") && !ok("1", "0.9.0"));
        assert!(ok("^0.2.3", "0.2.9") && !ok("^0.2.3", "0.3.0"));
        assert!(ok("~1.2", "1.2.7") && !ok("~1.2", "1.3.0"));
        assert!(ok("=1.2.3", "1.2.3") && !ok("=1.2.3", "1.2.4"));
        assert!(ok(">1", "2.0.0") && !ok(">1", "1.5.0"));
        assert!(ok("<=1.2", "1.2.9") && !ok("<=1.2", "1.3.0"));
        assert!(ok("*", "9.9.9") && ok("1.*", "1.4.0"));
        assert!(ok("1.2.*", "1.2.5") && !ok("1.2.*", "1.3.0"));
        assert!(!ok("1", "1.2.0-rc1"));
        assert!(VersionReq::parse("").is_err() && Version::parse("1.0").is_err());
    }
}
