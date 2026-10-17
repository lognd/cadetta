//! frob:ticket 01M4TA3C87N60Y711163RK3GPH
//! The real adapter drift hook of `cadetta check`: spawn each external adapter, `read` its
//! files, compare with the design IR and run the R lints (lints.md 3 R, adapters.md 2, 3, 7).
//! Nothing is written: the lock is only read for `synced` bases.

use cadetta_adapter::read_facts::SKETCH_ORIGIN;
use cadetta_adapter::rpc::client::ProcessClient;
use cadetta_adapter::rpc::{CapabilityName, Client};
use cadetta_ir::digest::digest_project;
use cadetta_ir::model::Project as IrProject;
use cadetta_lint::diag::{Diagnostic, Severity};
use cadetta_lint::rules::r::{
    BindingFact, DriftFact, NativeRef, check_r001, check_r002, check_r003, check_r005,
};
use cadetta_lint::rules::r_adapter::{
    AdapterReport, CoordFact, SlotFact, StarBinding, check_r007, check_r011, check_r012,
    pass_through,
};
use std::io::{BufRead, Write};
use std::time::Duration;

use super::check::DriftSite;
use super::import::program;
use super::sync::{Probe, probe_adapter};

/// Codes `build_entries` reports that are R lints in their own right (R003 is structured).
const FORWARDED_ERRORS: [&str; 3] = ["R001", "R005", "R013"];

/// The default drift hook: spawn the adapter named by `command` and lint what it reads.
///
/// The adapter's `timeout` (default 60s) bounds each call. A spawn, version or timeout failure is R008, which `check` downgrades to info (adapters.md 7).
// frob:ticket 01M4TA3C87N60Y711163RK3GPH
pub(crate) fn process_drift(site: &DriftSite<'_>, ir: &IrProject) -> Vec<Diagnostic> {
    process_drift_within(site, ir, site.adapter.timeout())
}

/// [`process_drift`] with an explicit per-call `timeout`; a silent adapter is killed and is R008.
// frob:ticket 01M4YG7D22XEEK9M3X678B88BT
pub(crate) fn process_drift_within(
    site: &DriftSite<'_>,
    ir: &IrProject,
    timeout: Duration,
) -> Vec<Diagnostic> {
    let adapter = site.adapter;
    let Some(command) = &adapter.command else {
        return Vec::new();
    };
    let Some((path, argv)) = program(site.pkg, command) else {
        tracing::warn!(adapter = %adapter.id(), "check: adapter has an empty command");
        return r008(adapter.id(), "the adapter command is empty".to_owned());
    };
    tracing::info!(adapter = %adapter.id(), "check: starting adapter for drift lints");
    let mut client = match ProcessClient::spawn_with_timeout(
        &path,
        &argv,
        adapter.version.as_deref(),
        Some(timeout),
    ) {
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

/// The R lints over one probe: R001, R002, R003, R005, R007, R011, R012 and the adapter's own diagnostics.
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

    out.extend(observed_fact_lints(&native, probe));

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

/// R007, R011 and R012 from the sketch, component and per-instance facts a read carried.
///
/// A fact whose locator has no binding is skipped (R002 already names it). An adapter that
/// reports none of these fact lists yields no diagnostics (adapters.md 5).
// frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
fn observed_fact_lints(native: &dyn Fn(&str, &str) -> NativeRef, probe: &Probe) -> Vec<Diagnostic> {
    let path_of = |locator: &str| {
        probe
            .locator_paths
            .iter()
            .find(|(_, l)| l == locator)
            .map(|(p, _)| p.clone())
    };
    let coords: Vec<CoordFact> = probe
        .sketches
        .iter()
        .filter_map(|s| {
            Some(CoordFact {
                path: path_of(&s.locator)?,
                native: native(&s.file, &s.locator),
                on_datum_plane: s.on_datum_plane,
                from_origin: s.measured_from == SKETCH_ORIGIN,
            })
        })
        .collect();
    let slots: Vec<SlotFact> = probe
        .components
        .iter()
        .filter_map(|c| {
            let slot = path_of(&c.locator)?;
            let prefix = format!("{slot}.");
            let primary_file = probe
                .member_files
                .iter()
                .find(|(p, _)| *p == slot || p.starts_with(&prefix))
                .map(|(_, f)| f.clone())?;
            Some(SlotFact {
                slot,
                primary_file,
                held_file: c.component_file.clone(),
                native: native(&c.file, &c.locator),
            })
        })
        .collect();
    let stars: Vec<StarBinding> = probe
        .instance_values
        .iter()
        .filter_map(|i| {
            Some(StarBinding {
                path: path_of(&i.locator)?,
                native: native(&i.file, &i.locator),
                values: i.values.clone(),
            })
        })
        .collect();
    tracing::debug!(
        coords = coords.len(),
        slots = slots.len(),
        stars = stars.len(),
        "check: observed facts"
    );
    let mut out = check_r007(&coords);
    out.extend(check_r011(&slots));
    out.extend(check_r012(&stars));
    out
}

#[cfg(test)]
mod tests {
    use super::process_drift_within;
    use crate::cmd::Globals;
    use crate::cmd::check::{run_check, run_check_with};
    use crate::output::Format;
    use cadetta_obligations::lock::{Lock as FileLock, Record, SyncedValue};
    use serde_json::{Value, json};
    use std::path::PathBuf;
    use std::time::Duration;

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
            || {
                format!(
                    "[\"/usr/bin/env\", \"python3\", \"{}\"]",
                    dir.0.join("adapter.py").display()
                )
            },
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

    /// Write an executable shell adapter `name` into `dir` and return its absolute path.
    fn script(dir: &Dir, name: &str, body: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.0.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p.display().to_string()
    }

    /// `check` with a short adapter timeout, returning wall time as well.
    fn check_within(dir: &Dir, timeout: Duration) -> (u8, Vec<(String, String)>, Duration) {
        let globals = Globals {
            format: Format::Json,
            cwd: Some(dir.0.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let start = std::time::Instant::now();
        let code = run_check_with(&[], &globals, &mut out, &mut err, &|s, ir| {
            process_drift_within(s, ir, timeout)
        });
        let took = start.elapsed();
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
        (code, diags, took)
    }

    // frob:ticket 01M4YG7D22XEEK9M3X678B88BT
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::process_drift_within
    #[test]
    fn check_reports_r008_within_bounded_time_for_a_silent_adapter() {
        let obs = observed(&[(PATH, "Plate_Thk", "6mm")]);
        let dir = project(&obs, Some("6mm"), BINDINGS, None);
        let silent = script(&dir, "silent.sh", "sleep 300");
        let toml = std::fs::read_to_string(dir.0.join("cadetta.toml")).unwrap();
        let adapter = format!("[\"{silent}\"]");
        let cmd_line = toml.lines().find(|l| l.starts_with("command")).unwrap();
        std::fs::write(
            dir.0.join("cadetta.toml"),
            toml.replace(cmd_line, &format!("command = {adapter}")),
        )
        .unwrap();
        let (code, diags, took) = check_within(&dir, Duration::from_millis(500));
        let r008 = diags.iter().find(|(c, _)| c == "R008").expect("R008");
        assert_eq!(r008.1, "info", "{diags:?}");
        assert_eq!(code, 0);
        assert!(took < Duration::from_secs(30), "took {took:?}");
    }

    // frob:ticket 01M4YG7D22XEEK9M3X678B88BT
    // frob:tests crates/cadetta-cli/src/cmd/import.rs::program
    #[test]
    fn a_bare_system_command_is_not_found_on_path() {
        // `sh` exists on PATH but must not be run: with no PATH search the command is
        // `<pkg>/sh`, which is absent, so check reports R008 instead of piping JSON to sh.
        let obs = observed(&[(PATH, "Plate_Thk", "6mm")]);
        let dir = project(&obs, Some("6mm"), BINDINGS, Some("[\"sh\"]"));
        let (code, diags, took) = check_within(&dir, Duration::from_secs(20));
        let r008 = diags.iter().find(|(c, _)| c == "R008").expect("R008");
        assert_eq!(r008.1, "info", "{diags:?}");
        assert_eq!(code, 0);
        assert!(took < Duration::from_secs(15), "took {took:?}");
    }

    /// Add `extra` top-level keys to the scenario's read result.
    fn with_facts(mut obs: Value, extra: Value) -> Value {
        for (k, v) in extra.as_object().unwrap() {
            obs[k] = v.clone();
        }
        obs
    }

    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::observed_fact_lints
    #[test]
    fn check_reports_r007_for_a_sketch_off_its_datum_plane() {
        let obs = with_facts(
            observed(&[(PATH, "Plate_Thk", "6mm")]),
            json!({"sketches": [{"file": "part.FCStd", "locator": "Plate_Thk",
                                 "on_datum_plane": false, "measured_from": "sketch origin"}]}),
        );
        let dir = project(&obs, Some("6mm"), BINDINGS, None);
        let (_, diags) = check(&dir);
        assert!(has(&diags, "R007"), "{diags:?}");
    }

    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::observed_fact_lints
    #[test]
    fn check_is_quiet_for_a_sketch_on_its_plane_from_the_origin() {
        let obs = with_facts(
            observed(&[(PATH, "Plate_Thk", "6mm")]),
            json!({"sketches": [{"file": "part.FCStd", "locator": "Plate_Thk",
                                 "on_datum_plane": true, "measured_from": "sketch origin"}]}),
        );
        let dir = project(&obs, Some("6mm"), BINDINGS, None);
        let (_, diags) = check(&dir);
        assert!(!has(&diags, "R007"), "{diags:?}");
    }

    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::observed_fact_lints
    #[test]
    fn check_reports_r011_on_a_swapped_component_file() {
        let obs = with_facts(
            observed(&[(PATH, "Plate_Thk", "6mm")]),
            json!({"components": [{"file": "asm.FCStd", "locator": "Plate_Thk",
                                   "component_file": "other.FCStd"}]}),
        );
        let dir = project(&obs, Some("6mm"), BINDINGS, None);
        let (_, diags) = check(&dir);
        assert!(has(&diags, "R011"), "{diags:?}");
    }

    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    // frob:tests crates/cadetta-cli/src/cmd/check_drift.rs::observed_fact_lints
    #[test]
    fn check_reports_r012_for_differing_instance_values() {
        let obs = with_facts(
            observed(&[(PATH, "Plate_Thk", "6mm")]),
            json!({"instance_values": [{"file": "part.FCStd", "locator": "Plate_Thk",
                                        "values": ["6mm", "7mm"]}]}),
        );
        let dir = project(&obs, Some("6mm"), BINDINGS, None);
        let (code, diags) = check(&dir);
        assert!(has(&diags, "R012"), "{diags:?}");
        assert_eq!(code, 1);
    }
}
