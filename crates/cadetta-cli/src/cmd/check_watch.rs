//! frob:ticket 01M4SWKA7HT50RJJ7ZZWP6WMKH
//! `cadetta check --format json --watch`: re-run `check` whenever a source file changes and
//! emit one `cadetta.diag/1` record per line (newline-delimited), sorted by (file, line,
//! code) within a run (config-cli.md 5, lints.md 1.2). Records already emitted by the previous
//! run are not emitted again, so an unchanged run prints nothing.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use super::check::{DriftHook, run_check_with};
use crate::output::Format;
use crate::project::Project;
use crate::{EXIT_USAGE, Globals};

/// How often the real watcher polls the source tree.
const POLL: Duration = Duration::from_millis(500);

/// Run the watch loop until the process is interrupted; polls every [`POLL`].
pub fn run_watch(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
    drift: &DriftHook<'_>,
) -> u8 {
    run_watch_with(args, globals, out, err, drift, &mut || {
        std::thread::sleep(POLL);
        true
    })
}

/// Run the watch loop with an injected clock: `tick` blocks until the next poll and returns
/// `false` to stop (tests use it to avoid wall-clock sleeps). Returns the last run's exit code.
pub fn run_watch_with(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
    drift: &DriftHook<'_>,
    tick: &mut dyn FnMut() -> bool,
) -> u8 {
    if globals.format != Format::Json {
        tracing::error!("check --watch requires --format json");
        let _ = writeln!(err, "error: `--watch` requires `--format json`");
        return EXIT_USAGE;
    }
    let inner: Vec<String> = args.iter().filter(|a| *a != "--watch").cloned().collect();
    let root = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok((p, _)) => p.root,
        Err(e) => {
            let _ = writeln!(err, "error: {e}");
            return EXIT_USAGE;
        }
    };
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut code;
    loop {
        let before = fingerprint(&root);
        let (c, lines) = one_run(&inner, globals, drift, err);
        code = c;
        let fresh: Vec<&String> = lines.iter().filter(|l| !seen.contains(*l)).collect();
        tracing::info!(
            total = lines.len(),
            fresh = fresh.len(),
            "check --watch: run done"
        );
        for line in &fresh {
            if writeln!(out, "{line}").is_err() {
                return code;
            }
        }
        let _ = out.flush();
        seen = lines.into_iter().collect();
        // Wait for the tree to differ from what this run read.
        loop {
            if !tick() {
                return code;
            }
            if fingerprint(&root) != before {
                tracing::debug!("check --watch: source change detected");
                break;
            }
        }
    }
}

/// One pass: run `check` in JSON and split its report into per-record lines (already sorted).
fn one_run(
    args: &[String],
    globals: &Globals,
    drift: &DriftHook<'_>,
    err: &mut dyn Write,
) -> (u8, Vec<String>) {
    let g = Globals {
        format: Format::Json,
        cwd: globals.cwd.clone(),
        config: globals.config.clone(),
    };
    let mut buf = Vec::new();
    let code = run_check_with(args, &g, &mut buf, err, drift);
    let report: serde_json::Value = serde_json::from_slice(&buf).unwrap_or_default();
    let lines = report["diagnostics"]
        .as_array()
        .map(|a| a.iter().map(ToString::to_string).collect())
        .unwrap_or_default();
    (code, lines)
}

/// A content hash of every watched file (`*.cdt`, `*.toml`, `cadetta.lock`) under `root`.
fn fingerprint(root: &Path) -> String {
    let mut files = Vec::new();
    collect(root, &mut files);
    files.sort();
    let mut h = blake3::Hasher::new();
    for f in files {
        h.update(f.to_string_lossy().as_bytes());
        h.update(&std::fs::read(&f).unwrap_or_default());
    }
    h.finalize().to_hex().to_string()
}

fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !name.starts_with('.') && name != "target" {
                collect(&path, out);
            }
        } else if name.ends_with(".cdt") || name.ends_with(".toml") || name == "cadetta.lock" {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::check::no_drift;

    const BAD: &str = "model Axle {\n  od: Length = 20mm\n  let bore: Length = 28mm\n  check bore_ok: bore < od \"bore fits\"\n}\n";
    const OK: &str = "model Axle {\n  od: Length = 20mm\n  let bore: Length = 8mm\n  check bore_ok: bore < od \"bore fits\"\n}\n";

    fn setup(name: &str, source: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cadetta-watch-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("cadetta.toml"),
            "[package]\nname = \"acme.axle\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("src/lib.cdt"), source).unwrap();
        dir
    }

    /// Drive the loop; `steps[i]` is applied at tick i (None = no change), then stop.
    fn drive(dir: &std::path::Path, steps: &[Option<&str>]) -> Vec<String> {
        let globals = Globals {
            format: Format::Json,
            cwd: Some(dir.to_path_buf()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let mut i = 0;
        // Ticks that change nothing are consumed until the next scripted change.
        let mut tick = || {
            if i >= steps.len() {
                return false;
            }
            if let Some(src) = steps[i] {
                std::fs::write(dir.join("src/lib.cdt"), src).unwrap();
            }
            i += 1;
            true
        };
        run_watch_with(
            &["--watch".to_owned()],
            &globals,
            &mut out,
            &mut err,
            &no_drift,
            &mut tick,
        );
        let _ = std::fs::remove_dir_all(dir);
        String::from_utf8(out)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    // frob:tests crates/cadetta-cli/src/cmd/check_watch.rs::run_watch_with
    #[test]
    fn change_emits_sorted_ndjson_records() {
        let dir = setup("change", OK);
        let lines = drive(&dir, &[Some(BAD), None]);
        assert!(!lines.is_empty());
        let recs: Vec<serde_json::Value> = lines
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert!(recs.iter().all(|r| r["schema"] == "cadetta.diag/1"));
        assert!(recs.iter().any(|r| r["code"] == "C001"));
        let keys: Vec<(String, u64, String)> = recs
            .iter()
            .map(|r| {
                (
                    r["location"]["file"].as_str().unwrap_or("").to_owned(),
                    r["location"]["start"]["line"].as_u64().unwrap_or(0),
                    r["code"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    // frob:tests crates/cadetta-cli/src/cmd/check_watch.rs::run_watch_with
    #[test]
    fn unchanged_run_emits_nothing_twice() {
        let dir = setup("same", BAD);
        let once = drive(&dir, &[None]);
        let dir = setup("same2", BAD);
        // Rewrite identical content plus a no-op edit: the same records must not repeat.
        let twice = drive(&dir, &[Some(BAD), Some(&format!("{BAD}\n")), None]);
        assert!(!once.is_empty());
        assert_eq!(once.len(), twice.len(), "{twice:?}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/check_watch.rs::run_watch_with
    #[test]
    fn requires_json_format() {
        let globals = Globals {
            format: Format::Text,
            cwd: None,
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_watch_with(&[], &globals, &mut out, &mut err, &no_drift, &mut || false);
        assert_eq!(code, EXIT_USAGE);
    }
}
