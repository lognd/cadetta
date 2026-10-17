//! frob:ticket 01M4V0NSCWCW8W0P7DG628M8AP
//! frob:ticket 01M4V1X55R20C1PT8B56BCVN48
//! Stage a fixture as a package in a scratch directory, run the `cadetta`
//! library on it and capture everything an expectation can observe.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use serde_json::Value;

use crate::compare::Produced;

/// Package name given to single-file fixtures; paths in `expect.json` use it.
const SINGLE_FILE_PACKAGE: &str = "acme.demo";

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Orders file writes against process spawns across the test threads of one binary.
///
/// A child forked while another thread still holds a script open for writing inherits that
/// handle, and `exec` of the script then fails with ETXTBSY (flaky under load). Writers hold
/// the write side while they create files; every run that may spawn an adapter holds the read side.
static WRITE_VS_SPAWN: RwLock<()> = RwLock::new(());

/// Hold this while creating or copying fixture files, so no spawn overlaps an open write handle.
pub fn writing() -> RwLockWriteGuard<'static, ()> {
    tracing::trace!("fixture write section");
    WRITE_VS_SPAWN.write().unwrap_or_else(|e| e.into_inner())
}

/// Hold this while running the CLI, which may spawn adapter processes.
pub fn spawning() -> RwLockReadGuard<'static, ()> {
    WRITE_VS_SPAWN.read().unwrap_or_else(|e| e.into_inner())
}

/// Everything one run of an executor observed.
#[derive(Default)]
pub struct Facts {
    /// Diagnostics parsed from the JSON report (empty when stdout is not a report).
    pub produced: Vec<Produced>,
    /// Process exit status (or the executor's synthetic status).
    pub exit: i32,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Diagnostic and error messages the run produced.
    pub messages: Vec<String>,
    /// Package files before the run (relative path to text).
    pub before: BTreeMap<String, String>,
    /// Package files after the run (relative path to text).
    pub after: BTreeMap<String, String>,
    /// Evaluated member values: canonical path to `n/d` or `n/d*pi^k`.
    pub members: BTreeMap<String, String>,
    /// Executor-specific observations compared by key with the expectation.
    pub observed: BTreeMap<String, Value>,
}

/// A scratch directory removed on drop.
pub struct Scratch(pub PathBuf);

impl Scratch {
    /// Create an empty scratch directory.
    pub fn new() -> Result<Self, String> {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cadetta-conf-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).map_err(|e| format!("scratch dir: {e}"))?;
        Ok(Self(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    for entry in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| format!("{}: {e}", target.display()))?;
        }
    }
    Ok(())
}

/// Stage a `.cdt` file as the single-file package, or copy a package directory.
pub fn stage(fixture: &Path) -> Result<Scratch, String> {
    let _writing = writing();
    let scratch = Scratch::new()?;
    let root = &scratch.0;
    if fixture.is_dir() {
        copy_dir(fixture, root)?;
    } else {
        let toml = format!(
            "[package]\nname = \"{SINGLE_FILE_PACKAGE}\"\nversion = \"0.1.0\"\nas_of = \"2026-10-13\"\n\n[lint]\nfail_on = \"error\"\n"
        );
        fs::write(root.join("cadetta.toml"), toml).map_err(|e| e.to_string())?;
        fs::create_dir_all(root.join("src")).map_err(|e| e.to_string())?;
        fs::copy(fixture, root.join("src/lib.cdt")).map_err(|e| e.to_string())?;
    }
    Ok(scratch)
}

/// Every file under `root` as relative path to lossy text.
pub fn snapshot(root: &Path) -> BTreeMap<String, String> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if let (Ok(rel), Ok(bytes)) = (path.strip_prefix(root), fs::read(&path)) {
                out.insert(
                    rel.to_string_lossy().replace('\\', "/"),
                    String::from_utf8_lossy(&bytes).into_owned(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// Parse the JSON document in `stdout`, tolerating log noise before it.
pub fn parse_json(stdout: &str) -> Option<Value> {
    let text = stdout.trim();
    serde_json::from_str(text).ok().or_else(|| {
        let start = text.find(['{', '['])?;
        serde_json::from_str(&text[start..]).ok()
    })
}

/// Diagnostics (`code`, `severity`, `path`) and messages from a `diagnostics` list; waived ones are suppressed.
// frob:ticket 01M51T6EHNEGAJ9KYQ42VJQFPG
fn diagnostics_of(report: &Value) -> (Vec<Produced>, Vec<String>) {
    let field = |d: &Value, k: &str| d[k].as_str().map(str::to_owned);
    let list: Vec<Value> = report["diagnostics"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|d| d["waived"].is_null())
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let produced = list
        .iter()
        .map(|d| Produced {
            code: field(d, "code").unwrap_or_default(),
            severity: field(d, "severity").unwrap_or_default(),
            path: field(d, "path"),
        })
        .collect();
    let messages = list.iter().filter_map(|d| field(d, "message")).collect();
    (produced, messages)
}

/// Run `cadetta --cwd <root> --format json <args...>` and capture the facts.
pub fn run_cli(root: &Path, args: &[String]) -> Result<Facts, String> {
    let mut argv: Vec<String> = vec![
        "--cwd".into(),
        root.to_str().ok_or("non-utf8 scratch path")?.into(),
        "--format".into(),
        "json".into(),
    ];
    argv.extend(args.iter().cloned());
    let before = snapshot(root);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let exit = {
        let _spawning = spawning();
        i32::from(cadetta::run_with_env(&argv, None, &mut out, &mut err))
    };
    tracing::debug!(?args, exit, "fixture ran");
    let mut facts = Facts {
        exit,
        stdout: String::from_utf8_lossy(&out).into_owned(),
        stderr: String::from_utf8_lossy(&err).into_owned(),
        before,
        after: snapshot(root),
        ..Facts::default()
    };
    if let Some(report) = parse_json(&facts.stdout) {
        (facts.produced, facts.messages) = diagnostics_of(&report);
    }
    if !facts.stderr.trim().is_empty() {
        facts.messages.push(facts.stderr.trim().to_owned());
    }
    Ok(facts)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A waived diagnostic is not counted as produced; an unwaived one is.
    // frob:ticket 01M51T6EHNEGAJ9KYQ42VJQFPG
    // frob:tests crates/cadetta-cli/tests/conformance/run.rs::diagnostics_of kind=unit
    #[test]
    fn waived_diagnostics_are_not_produced() {
        let report = serde_json::json!({"diagnostics": [
            {"code": "C001", "severity": "warning", "message": "m", "waived": {"by": "w"}},
            {"code": "C002", "severity": "warning", "message": "n", "waived": null},
            {"code": "C003", "severity": "warning", "message": "o"}
        ]});
        let (produced, messages) = diagnostics_of(&report);
        let codes: Vec<&str> = produced.iter().map(|p| p.code.as_str()).collect();
        assert_eq!(codes, ["C002", "C003"]);
        assert_eq!(messages.len(), 2);
    }
}
