//! frob:ticket 01M4XE4KFVQ24QF7QMXPBQGCFY
//! `cadetta lock verify`: check `cadetta.lock` is well formed, canonical, unique and
//! policy-conformant, and list attestations a merge invalidated (obligations.md 3,
//! config-cli.md 3).

use std::collections::{BTreeMap, HashMap};
use std::io::Write;

use cadetta_obligations::lock_verify::{RolePolicy, VerifyContext, VerifyReport, verify_text};

use super::obligations::Candidate;
use crate::config::Config;
use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::Project;
use crate::universe::{candidates, universes};
use crate::{EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta lock verify [--format json]";

/// The verify context from the live obligation candidates and the configured roles.
///
/// Authors are not tracked yet, so `distinct_from_author` is never violated here.
pub fn context_of(candidates: &[Candidate], config: &Config) -> VerifyContext {
    let digests: HashMap<_, _> = candidates
        .iter()
        .map(|c| (c.id.clone(), c.current.0.clone()))
        .collect();
    let roles: BTreeMap<_, _> = config
        .obligations
        .roles
        .iter()
        .map(|(name, r)| {
            (
                name.clone(),
                RolePolicy {
                    members: r.members.clone(),
                    distinct_from_author: r.distinct_from_author,
                },
            )
        })
        .collect();
    tracing::debug!(
        obligations = digests.len(),
        roles = roles.len(),
        "lock verify context built"
    );
    VerifyContext {
        digests,
        roles,
        authors: HashMap::new(),
    }
}

/// Render the report as text lines or the `cadetta.lock/1` JSON document.
pub fn render(report: &VerifyReport, format: Format, out: &mut dyn Write) {
    let result = if format == Format::Json {
        writeln!(out, "{}", report.to_json())
    } else if report.is_clean() {
        writeln!(out, "cadetta.lock: clean")
    } else {
        report
            .findings
            .iter()
            .try_for_each(|f| writeln!(out, "{f}"))
    };
    if let Err(error) = result {
        tracing::error!(%error, "could not write lock verify report");
    }
}

/// Run `cadetta lock ...`; only `verify` exists so far. Returns the exit code.
pub fn run_lock(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "lock failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    match args.split_first() {
        Some((verb, [])) if verb == "verify" => {}
        Some((verb, rest)) if verb == "verify" => {
            return fail(err, &format!("unexpected argument `{}`; {USAGE}", rest[0]));
        }
        Some((verb, _)) => return fail(err, &format!("unknown lock verb `{verb}`; {USAGE}")),
        None => return fail(err, USAGE),
    }
    let (project, members) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref())
    {
        Ok(o) => o,
        Err(e) => return fail(err, &e.to_string()),
    };
    let text = match std::fs::read_to_string(project.root.join("cadetta.lock")) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return fail(err, &e.to_string()),
    };
    let analysis = analyze(&project, &members);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    let ctx = context_of(&candidates(&universes(&analysis)), &project.config);
    let report = match verify_text(&text, &ctx) {
        Ok(r) => r,
        Err(e) => return fail(err, &e.to_string()),
    };
    render(&report, globals.format, out);
    tracing::info!(findings = report.findings.len(), "lock verify done");
    report.exit_code() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_obligations::lock_verify::Finding;

    // frob:tests crates/cadetta-cli/src/cmd/lock.rs::render
    #[test]
    fn json_report_carries_schema_and_exit_code() {
        let report = VerifyReport {
            findings: vec![Finding::NotCanonical],
        };
        let mut out = Vec::new();
        render(&report, Format::Json, &mut out);
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["schema"], "cadetta.lock/1");
        assert_eq!(v["clean"], false);
        assert_eq!(v["exit_code"], 1);
        assert_eq!(v["findings"][0]["kind"], "not_canonical");
    }

    // frob:tests crates/cadetta-cli/src/cmd/lock.rs::render
    #[test]
    fn text_report_is_clean_or_one_line_per_finding() {
        let mut out = Vec::new();
        render(&VerifyReport::default(), Format::Text, &mut out);
        assert_eq!(String::from_utf8(out).unwrap(), "cadetta.lock: clean\n");
        let mut out = Vec::new();
        let report = VerifyReport {
            findings: vec![Finding::NotCanonical],
        };
        render(&report, Format::Text, &mut out);
        assert_eq!(String::from_utf8(out).unwrap().lines().count(), 1);
    }

    // frob:tests crates/cadetta-cli/src/cmd/lock.rs::context_of
    #[test]
    fn context_maps_roles_and_digests() {
        let config: Config = toml::from_str(
            "[obligations.roles.review]\ncovers = [\"check:**\"]\nmembers = [\"a@x\"]\ndistinct_from_author = true\n",
        )
        .unwrap();
        let ctx = context_of(&[], &config);
        let role = &ctx.roles["review"];
        assert_eq!(role.members.as_deref(), Some(&["a@x".to_owned()][..]));
        assert!(role.distinct_from_author);
        assert!(ctx.digests.is_empty());
    }

    /// A self-removing scratch package.
    struct Dir(std::path::PathBuf);
    impl Dir {
        fn new() -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("cadetta-lock-{}-{n}", std::process::id()));
            std::fs::create_dir_all(p.join("src")).unwrap();
            std::fs::write(
                p.join("cadetta.toml"),
                "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n",
            )
            .unwrap();
            std::fs::write(
                p.join("src/lib.cdt"),
                "model Plate {\n  t: Length = 6mm\n}\n",
            )
            .unwrap();
            Self(p)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn run(dir: &Dir, format: Format, args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let globals = Globals {
            format,
            cwd: Some(dir.0.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_lock(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    // frob:tests crates/cadetta-cli/src/cmd/lock.rs::run_lock
    #[test]
    fn usage_errors_exit_2() {
        let dir = Dir::new();
        for args in [vec![], vec!["tidy"], vec!["verify", "x"]] {
            let (code, _, err) = run(&dir, Format::Text, &args);
            assert_eq!(code, EXIT_USAGE);
            assert!(err.contains("usage: cadetta lock verify"), "{err}");
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/lock.rs::run_lock
    #[test]
    fn absent_lock_is_clean_json() {
        let dir = Dir::new();
        let (code, out, _) = run(&dir, Format::Json, &["verify"]);
        assert_eq!(code, 0);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["schema"], "cadetta.lock/1");
        assert_eq!(v["clean"], true);
    }

    // frob:tests crates/cadetta-cli/src/cmd/lock.rs::run_lock
    #[test]
    fn malformed_lock_exits_2() {
        let dir = Dir::new();
        std::fs::write(dir.0.join("cadetta.lock"), "not a record\n").unwrap();
        let (code, _, err) = run(&dir, Format::Text, &["verify"]);
        assert_eq!(code, EXIT_USAGE, "{err}");
    }
}
