//! frob:ticket 01M4TA3C87N60Y711163RK3GPH
//! The real adapter drift hook of `cadetta check`: spawn each external adapter, `read` its
//! files, compare with the design IR and run the R lints (lints.md 3 R, adapters.md 2, 3, 7).
//! Nothing is written: the lock is only read for `synced` bases.

use cadetta_adapter::rpc::client::ProcessClient;
use cadetta_adapter::rpc::{CapabilityName, Client};
use cadetta_ir::digest::digest_project;
use cadetta_ir::model::Project as IrProject;
use cadetta_lint::diag::{Diagnostic, Severity};
use cadetta_lint::rules::r::{
    BindingFact, DriftFact, NativeRef, check_r001, check_r002, check_r003, check_r005,
};
use cadetta_lint::rules::r_adapter::{AdapterReport, pass_through};
use std::io::{BufRead, Write};

use super::check::DriftSite;
use super::import::program;
use super::sync::{Probe, probe_adapter};

/// Codes `build_entries` reports that are R lints in their own right (R003 is structured).
const FORWARDED_ERRORS: [&str; 3] = ["R001", "R005", "R013"];

/// The default drift hook: spawn the adapter named by `command` and lint what it reads.
///
/// A spawn or version failure is R008, which `check` downgrades to info (adapters.md 7).
// frob:ticket 01M4TA3C87N60Y711163RK3GPH
pub(crate) fn process_drift(site: &DriftSite<'_>, ir: &IrProject) -> Vec<Diagnostic> {
    let adapter = site.adapter;
    let Some(command) = &adapter.command else {
        return Vec::new();
    };
    let Some((path, argv)) = program(site.pkg, command) else {
        tracing::warn!(adapter = %adapter.id(), "check: adapter has an empty command");
        return r008(adapter.id(), "the adapter command is empty".to_owned());
    };
    tracing::info!(adapter = %adapter.id(), "check: starting adapter for drift lints");
    let mut client = match ProcessClient::spawn(&path, &argv, adapter.version.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(adapter = %adapter.id(), error = %e, "check: adapter did not start (R008)");
            return r008(adapter.id(), format!("could not start adapter: {e}"));
        }
    };
    let out = drift_with_client(&mut client, site, ir);
    if let Err(e) = client.shutdown() {
        tracing::warn!(adapter = %adapter.id(), error = %e, "check: adapter shutdown failed");
    }
    out
}

/// A one-record R008 pass-through for an adapter that could not be used.
fn r008(adapter: &str, message: String) -> Vec<Diagnostic> {
    pass_through(&[AdapterReport {
        code: "R008".to_owned(),
        message: format!("adapter `{adapter}`: {message}"),
        native: None,
    }])
}

/// Lint one adapter's read over an already initialized `client`.
///
/// Adapters without the `read` capability are skipped; a failed read is R008.
// frob:ticket 01M4TA3C87N60Y711163RK3GPH
pub(crate) fn drift_with_client<R: BufRead, W: Write>(
    client: &mut Client<R, W>,
    site: &DriftSite<'_>,
    ir: &IrProject,
) -> Vec<Diagnostic> {
    let adapter = site.adapter;
    if !client.info().capabilities.contains(&CapabilityName::Read) {
        tracing::debug!(adapter = %adapter.id(), "check: adapter cannot read; no drift lints");
        return Vec::new();
    }
    let mut ir = ir.clone();
    if let Err(e) = digest_project(&mut ir) {
        tracing::warn!(error = %e, "check: could not digest the IR; linting undigested");
    }
    match probe_adapter(client, site.pkg, adapter, &ir, site.lock) {
        Ok(probe) => lint_probe(adapter.id(), &bindings_file(site), &ir, &probe),
        Err(message) => {
            tracing::warn!(adapter = %adapter.id(), %message, "check: read failed (R008)");
            r008(adapter.id(), message)
        }
    }
}

/// The adapter's bindings file as the package sees it, for R005 locations.
fn bindings_file(site: &DriftSite<'_>) -> String {
    site.adapter.bindings_path()
}

/// The R lints over one probe: R001, R002, R003, R005 and the adapter's own diagnostics.
fn lint_probe(
    adapter: &str,
    bindings_file: &str,
    ir: &IrProject,
    probe: &Probe,
) -> Vec<Diagnostic> {
    let native = |file: &str, locator: &str| NativeRef {
        adapter: adapter.to_owned(),
        file: file.to_owned(),
        locator: locator.to_owned(),
    };
    let facts: Vec<DriftFact> = probe
        .entries
        .iter()
        .map(|e| DriftFact {
            path: e.path.clone(),
            native: native(&e.file, &e.locator),
            base: e.base.clone(),
            ours: e.ours.clone(),
            theirs: e.theirs.clone(),
        })
        .collect();
    let mut out = check_r001(&facts);
    let missing: Vec<(String, NativeRef)> = probe
        .missing
        .iter()
        .map(|(path, file, locator)| (path.clone(), native(file, locator)))
        .collect();
    out.extend(check_r003(&missing));
    let unbound: Vec<NativeRef> = probe.unbound.iter().map(|(f, l)| native(f, l)).collect();
    out.extend(check_r002(&unbound));

    let members: Vec<_> = ir.components.iter().flat_map(|c| &c.members).collect();
    let ir_paths: Vec<String> = members.iter().map(|m| m.path.to_string()).collect();
    let was_paths: Vec<String> = members
        .iter()
        .flat_map(|m| {
            let path = m.path.to_string();
            let stem = path.rsplit_once('.').map_or("", |(s, _)| s).to_owned();
            m.was
                .iter()
                .map(move |w| format!("{stem}.{w}"))
                .collect::<Vec<_>>()
        })
        .collect();
    let bindings: Vec<BindingFact> = probe
        .bindings
        .iter()
        .map(|(path, locator)| BindingFact {
            path: path.clone(),
            native: native(bindings_file, locator),
        })
        .collect();
    out.extend(check_r005(&bindings, &ir_paths, &was_paths));

    // Diagnostics build_entries raised itself; R003 came structured above.
    out.extend(
        probe
            .diagnostics
            .iter()
            .filter(|(code, _)| FORWARDED_ERRORS.contains(&code.as_str()))
            .map(|(code, message)| Diagnostic::new(code.clone(), Severity::Error, message.clone())),
    );
    let reports: Vec<AdapterReport> = probe
        .reports
        .iter()
        .map(|(code, message)| AdapterReport {
            code: code.clone(),
            message: message.clone(),
            native: None,
        })
        .collect();
    out.extend(pass_through(&reports));
    tracing::debug!(%adapter, diagnostics = out.len(), "check: drift lints done");
    out
}

#[cfg(test)]
mod tests {
    use crate::cmd::Globals;
    use crate::cmd::check::run_check;
    use crate::output::Format;
    use cadetta_obligations::lock::{Lock as FileLock, Record, SyncedValue};
    use serde_json::{Value, json};
    use std::path::PathBuf;

    const PATH: &str = "acme.plate::CoverPlate.thickness";
    const BINDINGS: &str = "[\"acme.plate::CoverPlate.thickness\"]\nlocators = [ { locator = \"Plate_Thk\", kind = \"global\", native_unit = \"mm\" } ]\n";

    /// A fake adapter: answers `read` with the project's `scenario.json`.
    const ADAPTER: &str = r#"
import json, os, sys
here = os.path.dirname(os.path.abspath(__file__))
sc = json.load(open(os.path.join(here, "scenario.json")))
info = {"name": "fake", "version": "0.1.0", "capabilities": ["read"],
        "native_units": "mm", "precision": 6, "platforms": ["linux"]}
for line in sys.stdin:
    req = json.loads(line)
    m = req["method"]
    res = info if m == "initialize" else (sc if m == "read" else None)
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": res}) + "\n")
    sys.stdout.flush()
    if m == "shutdown":
        break
"#;

    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("cadetta-drift-{}-{n}", std::process::id()));
            std::fs::create_dir_all(p.join("src")).unwrap();
            std::fs::create_dir_all(p.join("bindings")).unwrap();
            Self(p)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn observed(members: &[(&str, &str, &str)]) -> Value {
        let ms: Vec<Value> = members
            .iter()
            .map(|(path, native, text)| {
                json!({"path": path,
                       "provenance": {"imported": {"file": "part.FCStd", "native_name": native}},
                       "value": {"authored": text}})
            })
            .collect();
        json!({"observed": {"components": [{"members": ms}]}, "diagnostics": []})
    }

    /// A project whose adapter is the python fake; `command` overrides the adapter command.
    fn project(scenario: &Value, base: Option<&str>, bindings: &str, command: Option<&str>) -> Dir {
        let dir = Dir::new();
        std::fs::write(dir.0.join("adapter.py"), ADAPTER).unwrap();
        std::fs::write(dir.0.join("scenario.json"), scenario.to_string()).unwrap();
        let cmd = command.map_or_else(
            || format!("[\"python3\", \"{}\"]", dir.0.join("adapter.py").display()),
            str::to_owned,
        );
        std::fs::write(
            dir.0.join("cadetta.toml"),
            format!("[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"sw-equations\"\nid = \"equations\"\ncommand = {cmd}\nfiles = [\"part.FCStd\"]\n"),
        )
        .unwrap();
        std::fs::write(
            dir.0.join("src/lib.cdt"),
            "model CoverPlate {\n  thickness: Length = 6mm\n}\n",
        )
        .unwrap();
        std::fs::write(dir.0.join("bindings/equations.toml"), bindings).unwrap();
        if let Some(b) = base {
            let mut lock = FileLock::load(&dir.0.join("cadetta.lock")).unwrap();
            lock.insert(Record::SyncedValue(SyncedValue {
                adapter: "equations".into(),
                path: PATH.into(),
                locator: "Plate_Thk".into(),
                value: b.into(),
            }));
            lock.save(&dir.0.join("cadetta.lock")).unwrap();
        }
        dir
    }

    /// Run `cadetta check` with the real default hook; returns exit code and diagnostics.
    fn check(dir: &Dir) -> (u8, Vec<(String, String)>) {
        let globals = Globals {
            format: Format::Json,
            cwd: Some(dir.0.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_check(&[], &globals, &mut out, &mut err);
        let v: Value = serde_json::from_slice(&out).unwrap();
        let diags = v["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| {
                (
                    d["code"].as_str().unwrap().to_owned(),
                    d["severity"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        (code, diags)
    }

    fn has(diags: &[(String, String)], code: &str) -> bool {
        diags.iter().any(|(c, _)| c == code)
    }

    // frob:ticket 01M4TA3C87N60Y711163RK3GPH
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::process_drift
    #[test]
    fn check_reports_r001_when_cad_and_design_both_moved() {
        let obs = observed(&[(PATH, "Plate_Thk", "7mm")]);
        let dir = project(&obs, Some("5mm"), BINDINGS, None);
        let (code, diags) = check(&dir);
        assert!(has(&diags, "R001"), "{diags:?}");
        assert_eq!(code, 1);
    }

    // frob:ticket 01M4TA3C87N60Y711163RK3GPH
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::process_drift
    #[test]
    fn check_is_clean_when_cad_matches_design() {
        let obs = observed(&[(PATH, "Plate_Thk", "6mm")]);
        let dir = project(&obs, Some("6mm"), BINDINGS, None);
        let (code, diags) = check(&dir);
        assert!(!diags.iter().any(|(c, _)| c.starts_with('R')), "{diags:?}");
        assert_eq!(code, 0);
    }

    // frob:ticket 01M4TA3C87N60Y711163RK3GPH
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::lint_probe
    #[test]
    fn check_reports_r003_when_the_locator_is_missing_from_the_file() {
        let obs = observed(&[]);
        let dir = project(&obs, Some("6mm"), BINDINGS, None);
        let (code, diags) = check(&dir);
        assert!(has(&diags, "R003"), "{diags:?}");
        assert_eq!(code, 1);
    }

    // frob:ticket 01M4TA3C87N60Y711163RK3GPH
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::lint_probe
    #[test]
    fn check_reports_r002_for_an_unbound_native_value() {
        let obs = observed(&[
            (PATH, "Plate_Thk", "6mm"),
            ("acme.plate::CoverPlate.extra", "Extra_Dim", "3mm"),
        ]);
        let dir = project(&obs, Some("6mm"), BINDINGS, None);
        let (_, diags) = check(&dir);
        assert!(has(&diags, "R002"), "{diags:?}");
    }

    // frob:ticket 01M4TA3C87N60Y711163RK3GPH
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::lint_probe
    #[test]
    fn check_reports_r005_for_a_binding_to_a_missing_path() {
        let obs = observed(&[(PATH, "Plate_Thk", "6mm")]);
        let bad = format!(
            "{BINDINGS}[\"acme.plate::CoverPlate.nope\"]\nlocators = [ {{ locator = \"Gone\", kind = \"global\", native_unit = \"mm\" }} ]\n"
        );
        let dir = project(&obs, Some("6mm"), &bad, None);
        let (code, diags) = check(&dir);
        let r005 = diags.iter().find(|(c, _)| c == "R005").expect("R005");
        assert_eq!(r005.1, "warning", "{diags:?}");
        assert_eq!(code, 0);
    }

    // frob:ticket 01M4TA3C87N60Y711163RK3GPH
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::process_drift
    #[test]
    fn check_downgrades_an_unstartable_adapter_to_r008_info() {
        let obs = observed(&[(PATH, "Plate_Thk", "6mm")]);
        let dir = project(
            &obs,
            Some("6mm"),
            BINDINGS,
            Some("[\"no-such-adapter-binary\"]"),
        );
        let (code, diags) = check(&dir);
        let r008 = diags.iter().find(|(c, _)| c == "R008").expect("R008");
        assert_eq!(r008.1, "info", "{diags:?}");
        assert_eq!(code, 0);
    }

    // frob:ticket 01M4TA3C87N60Y711163RK3GPH
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::lint_probe
    #[test]
    fn check_passes_adapter_reported_diagnostics_through() {
        let mut obs = observed(&[(PATH, "Plate_Thk", "6mm")]);
        obs["diagnostics"] = json!([{"schema": "cadetta.diag/1", "code": "R009", "severity": "warning", "message": "file has unsaved changes"}]);
        let dir = project(&obs, Some("6mm"), BINDINGS, None);
        let (_, diags) = check(&dir);
        assert!(has(&diags, "R009"), "{diags:?}");
        assert!(!has(&diags, "R008"), "{diags:?}");
    }
}
