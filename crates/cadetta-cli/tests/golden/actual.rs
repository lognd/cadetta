//! frob:ticket 01M4SXSGQQHR6RMBNJQJ3392AZ
//! Run the `cadetta` library on a scratch copy of a case and capture what the compare reads.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::Value as Json;

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A scratch directory removed on drop.
pub struct Scratch(pub PathBuf);

impl Scratch {
    /// Create an empty scratch directory under the cargo test temp dir.
    pub fn new() -> Result<Self, String> {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let base =
            option_env!("CARGO_TARGET_TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let dir = base.join(format!("golden-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).map_err(|e| format!("scratch dir: {e}"))?;
        Ok(Self(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Copy a directory tree.
pub fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
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

/// Output of one `cadetta` invocation.
pub struct Run {
    /// Exit status.
    pub exit: i32,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

/// Run `cadetta --cwd <root> --format json <args...>` in process.
pub fn run_cli(root: &Path, args: &[String]) -> Result<Run, String> {
    let mut argv: Vec<String> = vec![
        "--cwd".into(),
        root.to_str().ok_or("non-utf8 path")?.into(),
        "--format".into(),
        "json".into(),
    ];
    argv.extend(args.iter().cloned());
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let exit = i32::from(cadetta::run_with_env(&argv, None, &mut out, &mut err));
    tracing::debug!(?args, exit, "golden: cadetta ran");
    Ok(Run {
        exit,
        stdout: String::from_utf8_lossy(&out).into_owned(),
        stderr: String::from_utf8_lossy(&err).into_owned(),
    })
}

/// One diagnostic from `cadetta check --format json`.
#[derive(Debug, Clone)]
pub struct Diag {
    /// Diagnostic code.
    pub code: String,
    /// `error`, `warning` or `info`.
    pub severity: String,
    /// Subject path.
    pub path: Option<String>,
    /// Variant key (the report's `alt`).
    pub alt: Option<String>,
    /// Whether a waiver covers it.
    pub waived: bool,
}

/// What a case run observed.
pub struct Actual {
    /// The `cadetta ir` document, or why it is missing.
    pub ir: Result<Json, String>,
    /// The `check` diagnostics, or why they are missing.
    pub diags: Result<Vec<Diag>, String>,
}

impl Actual {
    /// True when the IR built and `check` reports no live error.
    pub fn clean(&self) -> bool {
        self.ir.is_ok()
            && self
                .diags
                .as_ref()
                .is_ok_and(|d| d.iter().all(|x| x.waived || x.severity != "error"))
    }
}

fn first_line(s: &str) -> String {
    s.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .to_owned()
}

fn diags_of(stdout: &str) -> Result<Vec<Diag>, String> {
    let doc: Json = serde_json::from_str(stdout).map_err(|e| format!("check output: {e}"))?;
    let list = doc
        .get("diagnostics")
        .and_then(Json::as_array)
        .ok_or("check output has no diagnostics array")?;
    let text = |d: &Json, k: &str| d.get(k).and_then(Json::as_str).map(str::to_owned);
    Ok(list
        .iter()
        .map(|d| Diag {
            code: text(d, "code").unwrap_or_default(),
            severity: text(d, "severity").unwrap_or_default(),
            path: text(d, "path"),
            alt: text(d, "alt"),
            waived: d.get("waived").and_then(Json::as_bool).unwrap_or(false),
        })
        .collect())
}

/// Run `check` and `ir` at the root.
pub fn observe(root: &Path) -> Actual {
    let diags = run_cli(root, &["check".into()]).and_then(|r| diags_of(&r.stdout));
    let ir = run_cli(root, &["ir".into()]).and_then(|r| {
        if r.exit != 0 {
            Err(format!(
                "`cadetta ir` exit {}: {}",
                r.exit,
                first_line(&r.stderr)
            ))
        } else {
            serde_json::from_str(&r.stdout).map_err(|e| format!("ir output: {e}"))
        }
    });
    Actual { ir, diags }
}

/// Split an artifact command into arguments and an optional `> file` redirect.
pub fn split_command(command: &str) -> Result<(Vec<String>, Option<String>), String> {
    let mut words: Vec<String> = command.split_whitespace().map(str::to_owned).collect();
    if words.first().map(String::as_str) != Some("cadetta") {
        return Err(format!(
            "artifact command is not a cadetta command: `{command}`"
        ));
    }
    words.remove(0);
    let redirect = match words.iter().position(|w| w == ">") {
        Some(i) => {
            let target = words.get(i + 1).cloned().ok_or("redirect without a file")?;
            words.truncate(i);
            Some(target)
        }
        None => None,
    };
    Ok((words, redirect))
}

/// Run an artifact command at `root` and return the bytes of the file it produces.
pub fn produce(root: &Path, command: &str, produces: &str) -> Result<Vec<u8>, String> {
    let (args, redirect) = split_command(command)?;
    let run = run_cli(root, &args)?;
    if run.exit != 0 {
        return Err(format!(
            "`{command}` exit {}: {}",
            run.exit,
            first_line(&run.stderr)
        ));
    }
    if let Some(file) = redirect {
        let target = root.join(&file);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        fs::write(&target, run.stdout.as_bytes()).map_err(|e| format!("{file}: {e}"))?;
    }
    fs::read(root.join(produces)).map_err(|e| format!("{produces}: {e}"))
}
