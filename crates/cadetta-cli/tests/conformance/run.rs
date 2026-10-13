//! frob:ticket 01M4V0NSCWCW8W0P7DG628M8AP
//! Run one source fixture through `cadetta check` (the library entry point).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::compare::Produced;

/// Package name given to single-file fixtures; paths in `expect.json` use it.
const SINGLE_FILE_PACKAGE: &str = "acme.demo";

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// What a run produced.
pub struct RunResult {
    /// Diagnostics parsed from the JSON report.
    pub produced: Vec<Produced>,
    /// Process exit status.
    pub exit: i32,
}

/// A scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Self, String> {
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

/// Run `cadetta check --format json` on a `.cdt` file or a package directory.
pub fn run_check(fixture: &Path) -> Result<RunResult, String> {
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
    let args: Vec<String> = [
        "--cwd",
        root.to_str().ok_or("non-utf8 scratch path")?,
        "--format",
        "json",
        "check",
    ]
    .map(str::to_owned)
    .to_vec();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let exit = i32::from(cadetta::run_with_env(&args, None, &mut out, &mut err));
    tracing::debug!(fixture = %fixture.display(), exit, "fixture ran");
    let text = String::from_utf8_lossy(&out);
    let report: serde_json::Value = serde_json::from_str(text.trim()).map_err(|_| {
        format!(
            "no JSON report (exit {exit}): {}",
            String::from_utf8_lossy(&err).trim()
        )
    })?;
    let list = report["diagnostics"]
        .as_array()
        .ok_or("report lacks `diagnostics`")?;
    let field = |d: &serde_json::Value, k: &str| d[k].as_str().map(str::to_owned);
    let produced = list
        .iter()
        .map(|d| Produced {
            code: field(d, "code").unwrap_or_default(),
            severity: field(d, "severity").unwrap_or_default(),
            path: field(d, "path"),
        })
        .collect();
    Ok(RunResult { produced, exit })
}
