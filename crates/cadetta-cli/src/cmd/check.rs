//! frob:ticket 01M4SPWT3FP4SN4RTCHE631Y52
//! `cadetta check [--release] [--as-of DATE] [--watch] [--module PATH]... [GLOB]...`: compile
//! the project, run every lint and check, apply the obligation gates and exit 1 when a
//! diagnostic reaches `[lint] fail_on` (config-cli.md 3, obligations.md 4).

use std::io::Write;

use cadetta_ir::model::Project as IrProject;
use cadetta_lint::diag::{self, Diagnostic, Severity};
use cadetta_lint::engine::{LintConfig, Suppressions, apply};
use cadetta_lint::rules::s::{CitationDrift, check_s004 as s004};
use cadetta_lint::rules::w;
use cadetta_obligations::as_of::{AsOf, Date as Today};
use cadetta_obligations::citation::{SOURCE_ROLE, check_citation, citation_id};
use cadetta_obligations::graph::{ObligationKind, ObligationState};
use cadetta_obligations::lock::Lock;
use cadetta_obligations::roles::Policy;

use super::ack::policy_of;
use super::obligations::{Candidate, ObligationsArgs, list};
use crate::config::{Adapter, Config};
use crate::pipeline::{Analysis, analyze};
use crate::project::Project;
use crate::scope::Scope;
use crate::universe::{self, universes};
use crate::{EXIT_USAGE, Globals, output};

const USAGE: &str =
    "usage: cadetta check [--release] [--as-of DATE] [--watch] [--module PATH]... [GLOB]...";

/// The parsed command line of `cadetta check`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CheckArgs {
    /// Also gate obligations whose role is `required_for = "release"`.
    pub release: bool,
    /// Date overriding `[package] as_of` and today.
    pub as_of: Option<String>,
    /// Re-run on change (separate verb machinery; see `run_check`).
    pub watch: bool,
    /// `--module` scopes.
    pub modules: Vec<String>,
    /// Path globs.
    pub globs: Vec<String>,
}

/// Parse the words after `check`; the error is a usage message.
pub fn parse_args(args: &[String]) -> Result<CheckArgs, String> {
    let mut parsed = CheckArgs::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--release" => parsed.release = true,
            "--watch" => parsed.watch = true,
            "--as-of" => {
                parsed.as_of = Some(it.next().cloned().ok_or("`--as-of` needs a value")?);
            }
            "--module" => {
                parsed
                    .modules
                    .push(it.next().cloned().ok_or("`--module` needs a value")?);
            }
            flag if flag.starts_with("--") => return Err(format!("unknown flag `{flag}`")),
            glob => parsed.globs.push(glob.to_owned()),
        }
    }
    Ok(parsed)
}

/// A drift hook: lints comparing an adapter's observed IR with the design IR (adapters.md).
pub type DriftHook<'a> = dyn Fn(&Adapter, &IrProject) -> Vec<Diagnostic> + 'a;

/// The default drift hook: the adapter host is not wired, so no drift is observed.
pub(crate) fn no_drift(adapter: &Adapter, _ir: &IrProject) -> Vec<Diagnostic> {
    tracing::info!(adapter = %adapter.name, "check: adapter read/drift lints are not wired; skipped");
    Vec::new()
}

/// The lint configuration from `[lint]`; the error is a config message.
fn lint_config(config: &Config) -> Result<LintConfig, String> {
    let mut cfg = LintConfig::new();
    for (code, sev) in &config.lint.severity {
        let word = match sev {
            crate::config::Severity::Info => "info",
            crate::config::Severity::Warning => "warning",
            crate::config::Severity::Error => "error",
        };
        cfg.set_severity(code, word).map_err(|e| e.to_string())?;
    }
    Ok(cfg)
}

/// One obligation per IR check and `ots` component that some role covers; cite items are left
/// to S004 alone, never O001/O002 (obligations.md 2.3).
// frob:ticket 01M4TA3C5QWZ2NH70V6QN4PR7X
fn candidates(analysis: &Analysis, policy: &Policy) -> Vec<Candidate> {
    let mut out = universe::candidates(&universes(analysis));
    out.retain(|c| {
        c.id.kind() != ObligationKind::Citation && !policy.roles_covering(&c.id).is_empty()
    });
    out
}

/// S004 diagnostics: acknowledged `cite` items whose digest (fields or cited file) has moved on.
// frob:ticket 01M4VB63M63PQVRKN0HDXQP567
fn citation_diagnostics(analysis: &Analysis, lock: &Lock) -> Vec<Diagnostic> {
    let mut drift = Vec::new();
    for package in &analysis.packages {
        let prefix = format!("{}::", package.name);
        for c in package
            .ir
            .citations
            .iter()
            .filter(|c| c.path.starts_with(&prefix))
        {
            let att = lock.attestation(&citation_id(&c.path), SOURCE_ROLE);
            if !check_citation(c, att).is_stale() {
                continue;
            }
            let cites = |r: &Option<cadetta_ir::model::CiteRef>| {
                r.as_ref().and_then(|r| r.cite.as_deref()) == Some(c.path.as_str())
            };
            let mut citing = Vec::new();
            for comp in &package.ir.components {
                if cites(&comp.citation) {
                    citing.push(comp.path.to_string());
                }
                citing.extend(
                    comp.members
                        .iter()
                        .filter(|m| cites(&m.citation))
                        .map(|m| m.path.to_string()),
                );
            }
            drift.push(CitationDrift {
                path: c.path.clone(),
                citing,
            });
        }
    }
    s004(&drift)
}

/// O001/O002 diagnostics for the open and stale obligations; gating rows are errors.
fn obligation_diagnostics(
    analysis: &Analysis,
    policy: &Policy,
    lock: &Lock,
    args: &CheckArgs,
    as_of: &AsOf,
) -> Vec<(Diagnostic, bool)> {
    let list_args = ObligationsArgs {
        release: args.release,
        ..ObligationsArgs::default()
    };
    let report = list(
        &candidates(analysis, policy),
        lock,
        policy,
        &list_args,
        as_of,
    );
    report
        .rows
        .iter()
        .map(|row| {
            let (code, word) = match row.state {
                ObligationState::Stale => ("O001", "stale"),
                _ => ("O002", "open"),
            };
            let gating = !policy
                .gate(&[(row.id.clone(), row.state)], args.release)
                .passed();
            let role = row.role.as_deref().unwrap_or("-");
            let mut d = Diagnostic::new(
                code,
                Severity::Warning,
                format!("obligation `{}` is {word} (role {role})", row.id),
            );
            d.path = Some(row.id.path().to_owned());
            (d, gating)
        })
        .collect()
}

/// Run `cadetta check` with the default (unwired) adapter drift hook.
pub fn run_check(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    run_check_with(args, globals, out, err, &no_drift)
}

/// Run `cadetta check` with an explicit drift hook, called once per adapter that can read.
///
/// `--watch` hands over to the ndjson watch loop in `check_watch`.
pub fn run_check_with(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
    drift: &DriftHook<'_>,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "check failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(m) => return fail(err, &format!("{m}; {USAGE}")),
    };
    let scope = match Scope::new(&parsed.modules, &parsed.globs) {
        Ok(s) => s,
        Err(e) => return fail(err, &e.to_string()),
    };
    let (project, members) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref())
    {
        Ok(o) => o,
        Err(e) => return fail(err, &e.to_string()),
    };
    if parsed.watch {
        return super::check_watch::run_watch(args, globals, out, err, drift);
    }
    let policy = match policy_of(&project.config) {
        Ok(p) => p,
        Err(m) => return fail(err, &m),
    };
    let lock = match Lock::load(&project.root.join("cadetta.lock")) {
        Ok(l) => l,
        Err(e) => return fail(err, &e.to_string()),
    };
    let config_as_of = project
        .config
        .package
        .as_ref()
        .and_then(|p| p.as_of.as_deref());
    let as_of = match AsOf::resolve(parsed.as_of.as_deref(), config_as_of, Today::today()) {
        Ok(a) => a,
        Err(e) => return fail(err, &e.to_string()),
    };
    let cfg = match lint_config(&project.config) {
        Ok(c) => c,
        Err(m) => return fail(err, &m),
    };

    let analysis = analyze(&project, &members);
    let mut raw = analysis.diagnostics.clone();
    if !analysis.halted {
        for package in &analysis.packages {
            let adapters = project
                .packages
                .iter()
                .find(|p| {
                    p.config.package.as_ref().and_then(|c| c.name.as_deref())
                        == Some(package.name.as_str())
                })
                .map(|p| p.config.adapters.as_slice())
                .unwrap_or(&[]);
            for adapter in adapters.iter().filter(|a| a.command.is_some()) {
                tracing::debug!(adapter = %adapter.name, "check: running drift lints");
                raw.extend(drift(adapter, &package.ir));
            }
        }
    }
    let gating: Vec<(Diagnostic, bool)> = if analysis.halted {
        Vec::new()
    } else {
        obligation_diagnostics(&analysis, &policy, &lock, &parsed, &as_of)
    };
    if !analysis.halted {
        raw.extend(citation_diagnostics(&analysis, &lock));
    }
    let gating_keys: Vec<(String, Option<String>)> = gating
        .iter()
        .filter(|(_, g)| *g)
        .map(|(d, _)| (d.code.clone(), d.path.clone()))
        .collect();
    raw.extend(gating.into_iter().map(|(d, _)| d));

    if !scope.is_all() {
        raw.retain(|d| d.path.as_deref().is_some_and(|p| scope.contains(p)));
    }
    let sup = Suppressions {
        as_of: Some(as_of.date.to_string()),
        ..analysis.suppressions.clone()
    };
    let mut outcome = apply(raw, &cfg, &sup);
    // W001-W004 describe the waivers themselves, so they follow the engine run.
    let waiver_diags = w::check(&outcome, &sup, &cfg);
    outcome.diagnostics.extend(waiver_diags);
    for d in &mut outcome.diagnostics {
        if (d.code == "O001" || d.code == "O002")
            && gating_keys.contains(&(d.code.clone(), d.path.clone()))
        {
            d.severity = Severity::Error;
        }
    }
    let mut diagnostics = outcome.diagnostics;
    diag::sort(&mut diagnostics);
    let text = output::render(globals.format, &diagnostics, &|file, line| {
        analysis.sources.line(file, line)
    });
    if let Err(e) = out.write_all(text.as_bytes()) {
        return fail(err, &format!("could not write output: {e}"));
    }
    let code = output::exit_code(&diagnostics, project.config.lint.fail_on);
    tracing::info!(diagnostics = diagnostics.len(), code, "check finished");
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Format;
    use cadetta_obligations::graph::{ObligationId, ObligationKind, Verdict};
    use cadetta_obligations::lock::{Attest, Date, Hex, Record};
    use serde_json::Value;
    use std::cell::Cell;

    struct Dir(std::path::PathBuf);

    impl Dir {
        fn new() -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("cadetta-check-{}-{n}", std::process::id()));
            std::fs::create_dir_all(p.join("src")).unwrap();
            Self(p)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const AXLE_BAD: &str = "model Axle {\n  seat: Length = 5kg\n  od: Length = 20mm\n  let bore: Length = 8mm\n  check bore_ok: bore < od \"bore fits\"\n}\n";
    const AXLE_BAD_CHECK: &str = "model Axle {\n  od: Length = 20mm\n  let bore: Length = 28mm\n  check bore_ok: bore < od \"bore fits\"\n}\n";
    const AXLE_OK: &str = "model Axle {\n  od: Length = 20mm\n  let bore: Length = 8mm\n  check bore_ok: bore < od \"bore fits\"\n}\n";
    const ROLE_COVERS: &str = "check:acme.axle::**";

    fn project(source: &str, extra_toml: &str) -> Dir {
        let dir = Dir::new();
        std::fs::write(
            dir.0.join("cadetta.toml"),
            format!("[package]\nname = \"acme.axle\"\nversion = \"0.1.0\"\n{extra_toml}"),
        )
        .unwrap();
        std::fs::write(dir.0.join("src/lib.cdt"), source).unwrap();
        dir
    }

    fn roles(required_for: &str) -> String {
        format!(
            "[obligations.roles.review]\ncovers = [\"{ROLE_COVERS}\"]\nrequired_for = \"{required_for}\"\n"
        )
    }

    fn run_with(dir: &Dir, args: &[&str], drift: &DriftHook<'_>) -> (u8, Value, String) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let globals = Globals {
            format: Format::Json,
            cwd: Some(dir.0.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_check_with(&args, &globals, &mut out, &mut err, drift);
        let text = String::from_utf8(out).unwrap();
        let json = serde_json::from_str(&text).unwrap_or(Value::Null);
        (code, json, String::from_utf8(err).unwrap())
    }

    fn run(dir: &Dir, args: &[&str]) -> (u8, Value, String) {
        run_with(dir, args, &no_drift)
    }

    fn codes(v: &Value) -> Vec<String> {
        v["diagnostics"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|d| d["code"].as_str().unwrap().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::run_check_with
    #[test]
    fn q001_style_error_exits_one_with_code_path_and_span() {
        let dir = project(AXLE_BAD, "");
        let (code, json, _) = run(&dir, &[]);
        assert_eq!(code, 1);
        let diags = json["diagnostics"].as_array().unwrap();
        let q = diags.iter().find(|d| d["code"] == "Q002").expect("Q002");
        assert_eq!(q["path"], "acme.axle::Axle.seat");
        assert_eq!(q["location"]["file"], "src/lib.cdt");
        assert_eq!(q["location"]["start"]["line"], 2);
        assert!(q["location"]["start"]["col"].as_u64().unwrap() > 0);
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::run_check_with
    #[test]
    fn cross_component_read_with_wrong_dimension_is_q002() {
        let src =
            "model Plate {\n  mass: Mass = 2kg\n}\nmodel Axle {\n  seat: Length = Plate.mass\n}\n";
        let dir = project(src, "");
        let (code, json, _) = run(&dir, &[]);
        assert_eq!(code, 1);
        let q = json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["code"] == "Q002")
            .expect("Q002 for Length = Plate.mass");
        assert_eq!(q["path"], "acme.axle::Axle.seat");
        assert_eq!(q["location"]["start"]["line"], 5);
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::run_check_with
    #[test]
    fn cross_component_read_with_matching_dimension_is_clean() {
        let src = "model Plate {\n  width: Length = 4mm\n}\nmodel Axle {\n  seat: Length = Plate.width\n}\n";
        let dir = project(src, "");
        let (code, json, _) = run(&dir, &[]);
        assert_eq!(code, 0, "{json}");
        let bad: Vec<_> = codes(&json)
            .into_iter()
            .filter(|c| c.starts_with('Q') || c.starts_with('E'))
            .collect();
        assert!(bad.is_empty(), "{bad:?}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::run_check_with
    #[test]
    fn violated_check_is_c001_with_path() {
        let dir = project(AXLE_BAD_CHECK, "");
        let (code, json, _) = run(&dir, &[]);
        assert_eq!(code, 1);
        let c = json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["code"] == "C001")
            .expect("C001");
        assert_eq!(c["path"], "acme.axle::Axle.bore_ok");
        assert_eq!(c["location"]["start"]["line"], 4);
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::run_check_with
    #[test]
    fn clean_project_exits_zero() {
        let dir = project(AXLE_OK, "");
        let (code, json, e) = run(&dir, &[]);
        assert_eq!(e, "");
        assert_eq!(code, 0, "{json}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::run_check_with
    #[test]
    fn drift_lints_run_for_adapters_that_can_read() {
        let dir = project(
            AXLE_OK,
            "[[adapter]]\nname = \"ext\"\ncommand = [\"ext-adapter\"]\n[[adapter]]\nname = \"sw-equations\"\n",
        );
        let calls = Cell::new(0);
        let hook = |a: &Adapter, _ir: &IrProject| {
            calls.set(calls.get() + 1);
            assert_eq!(a.name, "ext");
            vec![Diagnostic::new("R001", Severity::Error, "drifted")]
        };
        let (code, json, _) = run_with(&dir, &[], &hook);
        assert_eq!(calls.get(), 1);
        assert!(codes(&json).contains(&"R001".to_owned()), "{json}");
        assert_eq!(code, 1);
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::obligation_diagnostics
    #[test]
    fn release_role_gates_open_obligations_only_with_release() {
        let dir = project(AXLE_OK, &roles("release"));
        let (code, json, _) = run(&dir, &[]);
        assert_eq!(code, 0, "{json}");
        assert!(codes(&json).contains(&"O002".to_owned()));
        let (code, json, _) = run(&dir, &["--release"]);
        assert_eq!(code, 1, "{json}");
        let o = json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["code"] == "O002")
            .unwrap();
        assert_eq!(o["severity"], "error");
        assert_eq!(o["path"], "acme.axle::Axle.bore_ok");
    }

    const AXLE_M001: &str = "model Axle {\n  od: Length = 20.1234mm\n}\n";

    // frob:ticket 01M4TQJJK40BJQTA59SMC5WD33
    // frob:tests crates/cadetta-cli/src/cmd/check.rs::run_check_with
    #[test]
    fn ir_lint_families_run_in_check() {
        let dir = project(AXLE_M001, "");
        let (_, json, _) = run(&dir, &[]);
        let c = codes(&json);
        assert!(c.contains(&"M001".to_owned()), "{json}");
    }

    // frob:ticket 01M4TQJJK40BJQTA59SMC5WD33
    // frob:tests crates/cadetta-cli/src/cmd/check.rs::run_check_with
    #[test]
    fn waive_suppresses_through_the_engine_and_unused_waiver_is_w004() {
        let waived = format!(
            "{AXLE_M001}waive M001 on acme.axle::Axle.od {{ because \"measured to the micron\" }}\n"
        );
        let dir = project(&waived, "");
        let (_, json, _) = run(&dir, &[]);
        let m = json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["code"] == "M001")
            .expect("M001 listed in json as waived");
        assert!(m["waived"].is_object(), "{json}");
        assert!(!codes(&json).contains(&"W004".to_owned()), "{json}");

        let unused = format!(
            "{AXLE_OK}waive M001 on acme.axle::Axle.od {{ because \"nothing to waive\" }}\n"
        );
        let dir = project(&unused, "");
        let (_, json, _) = run(&dir, &[]);
        assert!(codes(&json).contains(&"W004".to_owned()), "{json}");
    }

    fn attest(dir: &Dir, digest: &str) {
        let mut lock = Lock::new();
        lock.insert(Record::Attest(Attest {
            id: ObligationId::new(ObligationKind::Check, "acme.axle::Axle.bore_ok"),
            role: "review".into(),
            by: "ana@x".into(),
            at: Date::new("2026-10-01").unwrap(),
            digest: Hex::new(digest).unwrap(),
            verdict: Verdict::Holds,
            margin: "12mm".into(),
            deps: vec![],
            reason: "ok".into(),
        }));
        lock.save(&dir.0.join("cadetta.lock")).unwrap();
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::obligation_diagnostics
    #[test]
    fn stale_check_obligation_exits_one() {
        let dir = project(AXLE_OK, &roles("check"));
        attest(&dir, "aa");
        let (code, json, _) = run(&dir, &[]);
        assert!(codes(&json).contains(&"O001".to_owned()), "{json}");
        assert_eq!(code, 1);
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::obligation_diagnostics
    #[test]
    fn obligations_no_role_covers_are_not_raised() {
        let dir = project(AXLE_OK, "");
        let (code, json, _) = run(&dir, &[]);
        assert_eq!(code, 0);
        assert!(!codes(&json).iter().any(|c| c.starts_with('O')));
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::run_check_with
    #[test]
    fn flags_scope_as_of_watch_and_fail_on() {
        let dir = project(AXLE_BAD_CHECK, "");
        let (code, _, _) = run(&dir, &["--as-of", "2026-10-13"]);
        assert_eq!(code, 1);
        let (code, json, _) = run(&dir, &["--module", "other"]);
        assert_eq!(code, 0, "{json}");
        let (code, _, _) = run(&dir, &["--module", "acme.axle", "acme.axle::Axle.bore_*"]);
        assert_eq!(code, 1);
        let (code, _, _) = run(&dir, &["acme.axle::Axle.nothing"]);
        assert_eq!(code, 0);
        let lenient = project(AXLE_BAD_CHECK, "[lint]\nfail_on = \"never\"\n");
        let (code, json, _) = run(&lenient, &[]);
        assert_eq!(code, 0);
        assert!(codes(&json).contains(&"C001".to_owned()));
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::parse_args
    #[test]
    fn bad_arguments_are_usage_errors() {
        let dir = project(AXLE_OK, "");
        assert_eq!(run(&dir, &["--bogus"]).0, 2);
        assert_eq!(run(&dir, &["--as-of"]).0, 2);
        assert_eq!(run(&dir, &["--as-of", "not-a-date"]).0, 2);
        assert_eq!(run(&dir, &["--module"]).0, 2);
        assert_eq!(run(&dir, &["a[b"]).0, 2);
    }

    // frob:tests crates/cadetta-cli/src/cmd/check.rs::parse_args
    #[test]
    fn parse_collects_every_flag() {
        let args: Vec<String> = [
            "--release",
            "--watch",
            "--as-of",
            "2026-01-01",
            "--module",
            "m",
            "g*",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        let p = parse_args(&args).unwrap();
        assert!(p.release && p.watch);
        assert_eq!(p.as_of.as_deref(), Some("2026-01-01"));
        assert_eq!(
            (p.modules, p.globs),
            (vec!["m".to_owned()], vec!["g*".to_owned()])
        );
    }

    // frob:ticket 01M4SWN8G3QTQC5YK07VN1YQYC
    // frob:tests crates/cadetta-cli/src/cmd/check.rs::run_check_with
    #[test]
    fn json_validates_against_check_and_diag_schemas() {
        let load = |text: &str| -> Value { serde_json::from_str(text).unwrap() };
        let check_schema = load(include_str!("../../../../docs/schemas/check.json"));
        let diag_schema = load(include_str!("../../../../docs/schemas/diag.json"));
        let check_validator = jsonschema::validator_for(&check_schema).unwrap();
        let diag_validator = jsonschema::validator_for(&diag_schema).unwrap();
        let dir = project(AXLE_BAD, "");
        let (_, json, _) = run(&dir, &[]);
        assert!(check_validator.is_valid(&json), "{json}");
        let diags = json["diagnostics"].as_array().unwrap();
        assert!(!diags.is_empty());
        for d in diags {
            assert!(diag_validator.is_valid(d), "{d}");
        }
    }
}
