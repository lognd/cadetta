//! frob:ticket 01M4SXRFQ0RENW7YM154W0EWP9
//! `cadetta update [NAME]`: re-resolve dependencies and rewrite their `package` records in
//! `cadetta.lock` (packages.md 1, 6; config-cli.md 3).

use std::io::Write;
use std::path::{Path, PathBuf};

use cadetta_obligations::lock::{Lock, Record};
use cadetta_obligations::lock_package::{ResolvedPackage as Pin, content_digest, pin_package};
use serde_json::{Value, json};

use crate::output::{EXIT_DIAGNOSTICS, Format};
use crate::pkg::manifest::{self, GitRef, Manifest, PackageManifest, Source};
use crate::pkg::resolve::{self, ResolveError};
use crate::project::Project;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Schema tag of the JSON report; the schema is `docs/schemas/update.json`.
pub const SCHEMA: &str = "cadetta.update/1";

const USAGE: &str = "usage: cadetta update [NAME]";

/// What an update did to one package record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The package had no record before.
    Added,
    /// The record differs from the one it replaced.
    Changed,
    /// The record is byte-for-byte what it was.
    Unchanged,
}

impl Change {
    /// The word used in text and JSON output.
    pub fn word(self) -> &'static str {
        match self {
            Change::Added => "added",
            Change::Changed => "changed",
            Change::Unchanged => "unchanged",
        }
    }
}

/// One package record written by an update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The pinned record.
    pub pin: Pin,
    /// What changed against the old record.
    pub change: Change,
}

/// The outcome of an update.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// Records written, ordered by package name.
    pub entries: Vec<Entry>,
    /// Packages whose record was dropped because they left the dependency closure.
    pub removed: Vec<String>,
}

/// Why an update failed; the lock is untouched in every case.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// Resolution failed (E005, E012, E013, a bad manifest).
    #[error(transparent)]
    Resolve(#[from] ResolveError),
    /// `NAME` is not in the dependency closure.
    #[error("`{0}` is not a dependency of this project")]
    UnknownName(String),
    /// A dependency's files could not be read to digest them.
    #[error("cannot digest dependency `{name}`: {reason}")]
    Files {
        /// Dependency name.
        name: String,
        /// What went wrong.
        reason: String,
    },
}

impl UpdateError {
    /// E012 and E013 are diagnostics (exit 1); everything else is a usage or I/O error (exit 2).
    pub fn exit_code(&self) -> u8 {
        match self {
            UpdateError::Resolve(
                ResolveError::VersionUnsatisfied { .. } | ResolveError::VersionConflict { .. },
            ) => EXIT_DIAGNOSTICS,
            _ => EXIT_USAGE,
        }
    }
}

/// Injected file access: a package directory's files as (relative path, bytes) pairs.
pub type ReadFiles<'a> = &'a dyn Fn(&Path) -> Result<Vec<(String, Vec<u8>)>, String>;

/// `path` relative to `root` with `/` separators and `..` steps; `.` when they are equal.
fn relative_to(root: &Path, path: &Path) -> String {
    let (a, b): (Vec<_>, Vec<_>) = (root.components().collect(), path.components().collect());
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let parts: Vec<String> = std::iter::repeat_n("..".to_owned(), a.len() - common)
        .chain(
            b[common..]
                .iter()
                .map(|c| c.as_os_str().to_string_lossy().into_owned()),
        )
        .collect();
    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}

/// Render a resolved source as the lock's `source` text; paths are relative to `root`.
pub fn source_text(root: &Path, source: &Source) -> String {
    match source {
        Source::Path(p) => format!("path:{}", relative_to(root, p)),
        Source::Git { url, reference } => {
            let (kind, name) = match reference {
                GitRef::Tag(s) => ("tag", s.as_str()),
                GitRef::Rev(s) => ("rev", s.as_str()),
                GitRef::Branch(s) => ("branch", s.as_str()),
                GitRef::Default => ("default", ""),
            };
            format!("git:{url}#{kind}={name}")
        }
    }
}

/// Re-resolve the dependencies of `roots` and pin them in `lock`; all of them, or only `only`.
///
/// A full update also drops records of packages that left the closure. The lock is modified
/// only after every package resolved and digested, so a failure leaves it unchanged.
pub fn update(
    root: &Path,
    roots: &[(PathBuf, PackageManifest)],
    only: Option<&str>,
    lock: &mut Lock,
    io: &resolve::Io<'_>,
    files: ReadFiles<'_>,
) -> Result<Report, UpdateError> {
    let resolution = resolve::resolve(roots, io)?;
    if let Some(name) = only
        && resolution.get(name).is_none()
    {
        tracing::error!(%name, "update names a package outside the dependency closure");
        return Err(UpdateError::UnknownName(name.to_owned()));
    }
    let mut pins = Vec::new();
    for pkg in resolution
        .packages
        .iter()
        .filter(|p| only.is_none_or(|n| n == p.name))
    {
        let contents = files(&pkg.dir).map_err(|reason| UpdateError::Files {
            name: pkg.name.clone(),
            reason,
        })?;
        pins.push(Pin {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            source: source_text(root, &pkg.source),
            digest: content_digest(&contents),
        });
    }
    let mut report = Report::default();
    for pin in pins {
        let key = ("package", vec![pin.name.clone()]);
        let change = match lock.get(&key) {
            None => Change::Added,
            Some(Record::Package(old))
                if old.version == pin.version
                    && old.source == pin.source
                    && old.digest == pin.digest =>
            {
                Change::Unchanged
            }
            Some(_) => Change::Changed,
        };
        tracing::info!(name = %pin.name, change = change.word(), "package record rewritten");
        pin_package(lock, &pin);
        report.entries.push(Entry { pin, change });
    }
    if only.is_none() {
        let stale: Vec<_> = lock
            .records()
            .filter_map(|r| match r {
                Record::Package(p) if resolution.get(&p.name).is_none() => Some(p.name.clone()),
                _ => None,
            })
            .collect();
        for name in stale {
            tracing::info!(%name, "package record dropped; no longer a dependency");
            lock.remove(&("package", vec![name.clone()]));
            report.removed.push(name);
        }
    }
    Ok(report)
}

/// The `cadetta.update/1` record.
pub fn to_json(report: &Report, wrote: bool) -> Value {
    let packages: Vec<Value> = report
        .entries
        .iter()
        .map(|e| {
            json!({
                "name": e.pin.name,
                "version": e.pin.version,
                "source": e.pin.source,
                "digest": e.pin.digest.as_str(),
                "change": e.change.word(),
            })
        })
        .collect();
    json!({
        "schema": SCHEMA,
        "packages": packages,
        "removed": report.removed,
        "wrote": wrote,
    })
}

/// The human text: one line per record, then dropped records.
pub fn to_text(report: &Report) -> String {
    let mut s = String::new();
    for e in &report.entries {
        s.push_str(&format!(
            "{} {} {} ({}) blake3:{}\n",
            e.pin.name,
            e.pin.version,
            e.pin.source,
            e.change.word(),
            e.pin.digest.as_str()
        ));
    }
    for name in &report.removed {
        s.push_str(&format!("{name} dropped (no longer a dependency)\n"));
    }
    if s.is_empty() {
        s.push_str("no dependencies\n");
    }
    s
}

/// Every file under `dir` as (relative path, bytes), skipping `.git` and the root `build/`.
pub fn read_package_files(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    fn walk(
        base: &Path,
        dir: &Path,
        out: &mut Vec<(String, Vec<u8>)>,
    ) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            if path.is_dir() {
                if name.as_deref() == Some(".git")
                    || (dir == base && name.as_deref() == Some("build"))
                {
                    continue;
                }
                walk(base, &path, out)?;
            } else {
                let rel = path.strip_prefix(base).unwrap_or(&path);
                out.push((
                    rel.to_string_lossy().replace('\\', "/"),
                    std::fs::read(&path)?,
                ));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(out)
}

/// The directory under `cache` where `url` at `reference` is (or will be) checked out.
// frob:ticket 01M4Y53MB9VXXCQB6FHQ06ZV6Q
pub fn checkout_dir(cache: &Path, url: &str, reference: &GitRef) -> PathBuf {
    let key = blake3::hash(format!("{url}\n{reference:?}").as_bytes()).to_hex();
    cache.join(&key.as_str()[..16])
}

/// Check out `url` at `reference` under `cache` with the `git` binary; returns the checkout.
pub fn git_checkout(cache: &Path, url: &str, reference: &GitRef) -> Result<PathBuf, String> {
    let git = |dir: Option<&Path>, args: &[&str]| -> Result<(), String> {
        let mut cmd = std::process::Command::new("git");
        if let Some(dir) = dir {
            cmd.arg("-C").arg(dir);
        }
        let output = cmd.args(args).output().map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
        }
    };
    let dir = checkout_dir(cache, url, reference);
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    if dir.join(".git").is_dir() {
        tracing::info!(%url, dir = %dir.display(), "fetching dependency");
        git(Some(&dir), &["fetch", "--tags", "--force", "origin"])?;
    } else {
        tracing::info!(%url, dir = %dir.display(), "cloning dependency");
        git(None, &["clone", "--quiet", url, &dir.to_string_lossy()])?;
    }
    let target = match reference {
        GitRef::Tag(s) | GitRef::Rev(s) => s.clone(),
        GitRef::Branch(s) => format!("origin/{s}"),
        GitRef::Default => "origin/HEAD".to_owned(),
    };
    git(Some(&dir), &["checkout", "--quiet", "--detach", &target])?;
    Ok(dir)
}

/// Load the package manifests of the project's packages as resolution roots.
pub(crate) fn roots_of(project: &Project) -> Result<Vec<(PathBuf, PackageManifest)>, String> {
    project
        .packages
        .iter()
        .map(|p| match manifest::load(&p.dir) {
            Ok(Manifest::Package(m)) => Ok((p.dir.clone(), m)),
            Ok(Manifest::Workspace(_)) => Err(format!("{}: not a package", p.dir.display())),
            Err(e) => Err(format!("{}: {e}", p.dir.display())),
        })
        .collect()
}

/// Run `cadetta update`; `args` are the words after the verb. Returns the exit code.
pub fn run_update(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, code: u8, message: &str| {
        tracing::error!(%message, code, "update failed");
        let _ = writeln!(err, "error: {message}");
        code
    };
    let only = match args {
        [] => None,
        [name] if !name.starts_with("--") => Some(name.as_str()),
        _ => return fail(err, EXIT_USAGE, USAGE),
    };
    let project = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok((project, _scope)) => project,
        Err(e) => return fail(err, EXIT_USAGE, &e.to_string()),
    };
    let roots = match roots_of(&project) {
        Ok(r) => r,
        Err(message) => return fail(err, EXIT_USAGE, &message),
    };
    let lock_path = project.root.join("cadetta.lock");
    let mut lock = match Lock::load(&lock_path) {
        Ok(l) => l,
        Err(e) => return fail(err, EXIT_USAGE, &e.to_string()),
    };
    let cache = project.root.join("build").join("deps");
    let read = |p: &Path| std::fs::read_to_string(p).ok();
    let fetch = |url: &str, r: &GitRef| git_checkout(&cache, url, r);
    let io = resolve::Io {
        read: &read,
        fetch: &fetch,
    };
    let report = match update(
        &project.root,
        &roots,
        only,
        &mut lock,
        &io,
        &read_package_files,
    ) {
        Ok(r) => r,
        Err(e) => return fail(err, e.exit_code(), &e.to_string()),
    };
    let wrote = match lock.save(&lock_path) {
        Ok(w) => w,
        Err(e) => return fail(err, EXIT_USAGE, &e.to_string()),
    };
    tracing::info!(
        records = report.entries.len(),
        removed = report.removed.len(),
        wrote,
        "update done"
    );
    match globals.format {
        Format::Json => {
            let _ = writeln!(out, "{}", to_json(&report, wrote));
        }
        _ => {
            let _ = write!(out, "{}", to_text(&report));
        }
    }
    EXIT_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_obligations::lock::Hex;
    use std::collections::BTreeMap;

    type Files = BTreeMap<PathBuf, String>;

    fn pkg_toml(name: &str, version: &str, deps: &str) -> String {
        format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n[dependencies]\n{deps}")
    }

    fn files(list: &[(&str, String)]) -> Files {
        list.iter()
            .map(|(p, t)| (PathBuf::from(p), t.clone()))
            .collect()
    }

    fn no_git(_: &str, _: &GitRef) -> Result<PathBuf, String> {
        Err("no network".into())
    }

    fn contents(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
        Ok(vec![(
            "cadetta.toml".into(),
            dir.to_string_lossy().as_bytes().to_vec(),
        )])
    }

    fn run(
        f: &Files,
        deps: &str,
        only: Option<&str>,
        lock: &mut Lock,
    ) -> Result<Report, UpdateError> {
        let read = |p: &Path| f.get(p).cloned();
        let io = resolve::Io {
            read: &read,
            fetch: &no_git,
        };
        let Manifest::Package(root) = manifest::parse(&pkg_toml("app", "1.0.0", deps)).unwrap()
        else {
            panic!("package")
        };
        update(
            Path::new("/w/app"),
            &[(PathBuf::from("/w/app"), root)],
            only,
            lock,
            &io,
            &contents,
        )
    }

    fn two() -> Files {
        files(&[
            ("/w/a/cadetta.toml", pkg_toml("a", "1.0.0", "")),
            ("/w/b/cadetta.toml", pkg_toml("b", "2.1.0", "")),
        ])
    }

    const DEPS: &str = "a = { path = \"../a\" }\nb = { path = \"../b\", version = \"2\" }\n";

    fn stale(name: &str) -> Pin {
        Pin {
            name: name.into(),
            version: "0.0.1".into(),
            source: "path:old".into(),
            digest: Hex::new(&"0".repeat(64)).unwrap(),
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/update.rs::update
    #[test]
    fn update_rewrites_every_record_and_drops_stale_ones() {
        let mut lock = Lock::new();
        pin_package(&mut lock, &stale("a"));
        pin_package(&mut lock, &stale("gone"));
        let report = run(&two(), DEPS, None, &mut lock).unwrap();
        let names: Vec<_> = report.entries.iter().map(|e| e.pin.name.as_str()).collect();
        assert_eq!(names, ["a", "b"]);
        assert_eq!(report.entries[0].change, Change::Changed);
        assert_eq!(report.entries[1].change, Change::Added);
        assert_eq!(report.removed, ["gone"]);
        let Some(Record::Package(b)) = lock.get(&("package", vec!["b".into()])) else {
            panic!("b pinned")
        };
        assert_eq!(
            (b.version.as_str(), b.source.as_str()),
            ("2.1.0", "path:../b")
        );
        assert!(lock.get(&("package", vec!["gone".into()])).is_none());
        // A second run changes nothing.
        let again = run(&two(), DEPS, None, &mut lock).unwrap();
        assert!(again.entries.iter().all(|e| e.change == Change::Unchanged));
    }

    // frob:tests crates/cadetta-cli/src/cmd/update.rs::update
    #[test]
    fn update_name_touches_only_that_record() {
        let mut lock = Lock::new();
        pin_package(&mut lock, &stale("a"));
        pin_package(&mut lock, &stale("b"));
        let report = run(&two(), DEPS, Some("b"), &mut lock).unwrap();
        assert_eq!(report.entries.len(), 1);
        assert!(report.removed.is_empty());
        let Some(Record::Package(a)) = lock.get(&("package", vec!["a".into()])) else {
            panic!("a pinned")
        };
        assert_eq!(a.version, "0.0.1");
        let err = run(&two(), DEPS, Some("zzz"), &mut lock).unwrap_err();
        assert!(matches!(err, UpdateError::UnknownName(_)));
        assert_eq!(err.exit_code(), EXIT_USAGE);
    }

    // frob:tests crates/cadetta-cli/src/cmd/update.rs::update
    #[test]
    fn e012_exits_one_and_leaves_the_lock_unchanged() {
        let mut lock = Lock::new();
        pin_package(&mut lock, &stale("a"));
        let before = lock.render();
        let err = run(
            &two(),
            "a = { path = \"../a\" }\nb = { path = \"../b\", version = \"3\" }\n",
            None,
            &mut lock,
        )
        .unwrap_err();
        assert!(err.to_string().contains("E012"));
        assert_eq!(err.exit_code(), EXIT_DIAGNOSTICS);
        assert_eq!(lock.render(), before);
    }

    // frob:tests crates/cadetta-cli/src/cmd/update.rs::update
    #[test]
    fn e013_exits_one() {
        let mut f = two();
        f.insert(
            "/w/a/cadetta.toml".into(),
            pkg_toml("a", "1.0.0", "b = { path = \"../b\", version = \"1\" }\n"),
        );
        let mut lock = Lock::new();
        let err = run(&f, DEPS, None, &mut lock).unwrap_err();
        assert!(err.to_string().contains("E013"));
        assert_eq!(err.exit_code(), EXIT_DIAGNOSTICS);
        assert!(lock.is_empty());
    }

    // frob:tests crates/cadetta-cli/src/cmd/update.rs::source_text
    #[test]
    fn sources_render_stably() {
        let root = Path::new("/w/app");
        assert_eq!(source_text(root, &Source::Path("/w/a".into())), "path:../a");
        assert_eq!(
            source_text(root, &Source::Path("/w/app/vendor/a".into())),
            "path:vendor/a"
        );
        let git = Source::Git {
            url: "https://x/p.git".into(),
            reference: GitRef::Tag("v2".into()),
        };
        assert_eq!(source_text(root, &git), "git:https://x/p.git#tag=v2");
    }

    // frob:tests crates/cadetta-cli/src/cmd/update.rs::to_json
    #[test]
    fn json_validates_against_update_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/update.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let mut lock = Lock::new();
        pin_package(&mut lock, &stale("gone"));
        let report = run(&two(), DEPS, None, &mut lock).unwrap();
        let value = to_json(&report, true);
        assert_eq!(value["schema"], SCHEMA);
        assert!(validator.is_valid(&value), "{value}");
        assert!(validator.is_valid(&to_json(&Report::default(), false)));
    }

    // frob:tests crates/cadetta-cli/src/cmd/update.rs::run_update
    #[test]
    fn run_update_writes_the_lock_and_reports() {
        let dir = std::env::temp_dir().join(format!("cadetta-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("app/src")).unwrap();
        std::fs::create_dir_all(dir.join("lib/src")).unwrap();
        std::fs::write(dir.join("app/src/lib.cdt"), "").unwrap();
        std::fs::write(dir.join("lib/src/lib.cdt"), "").unwrap();
        std::fs::write(dir.join("lib/cadetta.toml"), pkg_toml("lib", "1.0.0", "")).unwrap();
        std::fs::write(
            dir.join("app/cadetta.toml"),
            pkg_toml(
                "app",
                "1.0.0",
                "lib = { path = \"../lib\", version = \"1\" }\n",
            ),
        )
        .unwrap();
        let globals = Globals {
            format: Format::Json,
            cwd: Some(dir.join("app")),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run_update(&[], &globals, &mut out, &mut err), EXIT_OK);
        let v: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["packages"][0]["name"], "lib");
        assert_eq!(v["wrote"], true);
        let lock = std::fs::read_to_string(dir.join("app/cadetta.lock")).unwrap();
        assert!(lock.contains("package"), "{lock}");
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(
            run_update(&["a".into(), "b".into()], &globals, &mut out, &mut err),
            EXIT_USAGE
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
