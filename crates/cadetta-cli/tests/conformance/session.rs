//! frob:ticket 01M52SAFHK1694NYSMG6NNC66V
//! `binding sync` session fixtures (adapters.md 2.2): attach to a running instance and close
//! only what the adapter opened.
//!
//! Both are decisions the adapter makes, not the host, so the host executor cannot model them
//! without restating the answer. This executor runs the real FreeCAD reference adapter
//! (`adapters/freecad/tests/conformance/session_driver.py`, `uv run`) over a fake tool seeded
//! from the fixture and records what the adapter did to it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use super::Job;
use crate::compare::Produced;
use crate::run::Facts;

/// Fixture keys that select this executor instead of the host sync model.
pub const KEYS: &[&str] = &["running_instance", "opened_by_adapter", "preopened"];

/// The repository root above `fixture` (the directory holding `adapters/freecad`).
fn repo_root(fixture: &Path) -> Result<PathBuf, String> {
    fixture
        .ancestors()
        .find(|d| d.join("adapters/freecad/pyproject.toml").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("{}: no adapters/freecad above it", fixture.display()))
}

/// Run the adapter-side driver on the fixture and fold its JSON facts into [`Facts`].
pub fn run(job: &Job) -> Result<Facts, String> {
    let root = repo_root(job.fixture)?;
    let project = root.join("adapters/freecad");
    let driver = project.join("tests/conformance/session_driver.py");
    let fixture = fs::canonicalize(job.fixture).map_err(|e| e.to_string())?;
    tracing::debug!(fixture = %fixture.display(), "session: driving the FreeCAD adapter");
    let out = Command::new("uv")
        .args(["run", "--quiet", "--project"])
        .arg(&project)
        .arg("python")
        .arg(&driver)
        .arg(&fixture)
        .output()
        .map_err(|e| format!("session driver needs `uv` on PATH: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "session driver failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text
        .lines()
        .last()
        .ok_or("session driver printed nothing")?;
    let mut observed: Value = serde_json::from_str(line).map_err(|e| format!("{e}: {line}"))?;
    let mut facts = Facts::default();
    let diagnostics = observed["diagnostics"].take();
    for d in diagnostics.as_array().into_iter().flatten() {
        facts.produced.push(Produced {
            code: d["code"].as_str().unwrap_or_default().to_owned(),
            severity: d["severity"].as_str().unwrap_or_default().to_owned(),
            path: d["path"].as_str().map(str::to_owned),
        });
    }
    if let Some(map) = observed.as_object() {
        for (k, v) in map {
            facts.observed.insert(k.clone(), v.clone());
        }
    }
    tracing::info!(
        diagnostics = facts.produced.len(),
        "session: adapter driven"
    );
    Ok(facts)
}
