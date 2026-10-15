//! frob:ticket 01M4ST8F3SDF95MVP07VXXNX0X
//! `cadetta obligations [--stale] [--role R] [--release] [--as-of DATE]`: the design's
//! to-do list of open and stale obligations (obligations.md 5, config-cli.md 3).

use std::io::Write;

use cadetta_ir::model::Digest;
use cadetta_obligations::as_of::{AsOf, Date as Today};
use cadetta_obligations::graph::{ObligationId, ObligationState};
use cadetta_obligations::lock::{Attest, Lock};
use cadetta_obligations::roles::Policy;
use cadetta_obligations::staleness::{Judgement, classify_attested};
use cadetta_obligations::trust::{Trust, Upstream};
use cadetta_obligations::why_stale::WhyStale;

use super::ack::policy_of;
use crate::output::Format;
use crate::pipeline::{Analysis, analyze};
use crate::project::Project;
use crate::universe::{candidates, universes};
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Exit code when a gating obligation is open or stale (config-cli.md 3).
pub const EXIT_PENDING: u8 = 1;

/// Schema tag of the JSON report.
pub const SCHEMA: &str = "cadetta.obligations/1";

const USAGE: &str = "usage: cadetta obligations [--stale] [--role R] [--release] [--as-of DATE]";

/// The parsed command line of `cadetta obligations`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ObligationsArgs {
    /// List stale obligations only.
    pub stale: bool,
    /// Restrict to obligations the named role covers.
    pub role: Option<String>,
    /// Gate as `check --release` does.
    pub release: bool,
    /// Date overriding `[package] as_of` and today.
    pub as_of: Option<String>,
}

/// Parse the words after `obligations`; the error is a usage message.
pub fn parse_args(args: &[String]) -> Result<ObligationsArgs, String> {
    let mut parsed = ObligationsArgs::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("`{flag}` needs a value"))
        };
        match arg.as_str() {
            "--stale" => parsed.stale = true,
            "--release" => parsed.release = true,
            "--role" => parsed.role = Some(value("--role")?),
            "--as-of" => parsed.as_of = Some(value("--as-of")?),
            flag if flag.starts_with("--") => return Err(format!("unknown flag `{flag}`")),
            extra => return Err(format!("unexpected argument `{extra}`")),
        }
    }
    Ok(parsed)
}

/// One obligation the current design offers, with the digests it is judged against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Obligation id.
    pub id: ObligationId,
    /// Current closure digest of the covered set.
    pub current: Digest,
    /// Current own digests of the transitive deps.
    pub deps: Vec<Digest>,
    /// Fresh verdict and margin, when the kind has one.
    pub judgement: Option<Judgement>,
    /// Whether a `waive` or `allow` suppresses the diagnostic.
    pub suppressed: bool,
    /// Nearest changed upstream node (the `why --stale` answer), when known.
    pub nearest_changed: Option<String>,
}

/// One printed row of the to-do list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Obligation id.
    pub id: ObligationId,
    /// Open or stale.
    pub state: ObligationState,
    /// The role responsible: the one asked for, else the first covering role.
    pub role: Option<String>,
    /// Last attester, if the obligation was ever attested.
    pub attester: Option<String>,
    /// Days since the last attestation, as of the report date.
    pub age_days: Option<i64>,
    /// Nearest changed upstream node.
    pub nearest_changed: Option<String>,
    /// "verdict unchanged, margin A -> B" when a stale verdict still holds.
    pub note: Option<String>,
    /// Attestations of this node carried by a dependency's lock ("attested upstream").
    pub upstream: Vec<String>,
}

/// The listing plus the gate outcome that decides the exit code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Open and stale rows in candidate order.
    pub rows: Vec<Row>,
    /// Whether any listed obligation is required for the chosen gate.
    pub gated: bool,
}

/// Days since 1970-01-01 of a `YYYY-MM-DD` string (Hinnant's days-from-civil); `None` if malformed.
fn day_number(date: &str) -> Option<i64> {
    let mut parts = date.split('-').map(str::parse::<i64>);
    let (y, m, d) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Build the open and stale list from the candidates, the lock and the role policy.
///
/// The row's state is judged against the attestation of the requested role (`--role`) or of
/// the first role covering the obligation; an obligation no role covers is open.
pub fn list(
    candidates: &[Candidate],
    lock: &Lock,
    policy: &Policy,
    args: &ObligationsArgs,
    as_of: &AsOf,
) -> Report {
    list_with_upstream(
        candidates,
        lock,
        policy,
        args,
        as_of,
        &Upstream::new(),
        &Trust::new(),
    )
}

/// [`list`] that also weighs dependency attestations (packages.md 6).
///
/// An obligation a trusted upstream role currently attests is discharged and not listed;
/// any other upstream attestation of it is shown on its row as information only.
pub fn list_with_upstream(
    candidates: &[Candidate],
    lock: &Lock,
    policy: &Policy,
    args: &ObligationsArgs,
    as_of: &AsOf,
    upstream: &Upstream,
    trust: &Trust,
) -> Report {
    let today = day_number(&as_of.date.to_string());
    let mut rows = Vec::new();
    for c in candidates {
        let covering: Vec<String> = policy
            .roles_covering(&c.id)
            .iter()
            .map(|r| r.name.clone())
            .collect();
        if let Some(wanted) = &args.role
            && !covering.contains(wanted)
        {
            tracing::trace!(id = %c.id, role = %wanted, "skipped: role does not cover");
            continue;
        }
        let role = args.role.clone().or_else(|| covering.first().cloned());
        let att: Option<&Attest> = role.as_deref().and_then(|r| lock.attestation(&c.id, r));
        let state = classify_attested(att, &c.current.0, &c.deps, c.suppressed);
        if state != ObligationState::Discharged
            && trust
                .discharging(upstream, &c.id, &covering, &c.current.0)
                .is_some()
        {
            tracing::debug!(id = %c.id, "not listed: discharged by trusted upstream attestation");
            continue;
        }
        let wanted_state = if args.stale {
            state == ObligationState::Stale
        } else {
            matches!(state, ObligationState::Stale | ObligationState::Open)
        };
        if !wanted_state {
            tracing::trace!(id = %c.id, ?state, "not listed");
            continue;
        }
        let note = match (state, att, &c.judgement) {
            (ObligationState::Stale, Some(a), Some(j)) if a.verdict == j.verdict => Some(format!(
                "verdict unchanged, margin {} -> {}",
                a.margin, j.margin
            )),
            _ => None,
        };
        let age_days = att
            .and_then(|a| day_number(a.at.as_str()))
            .zip(today)
            .map(|(then, now)| now - then);
        tracing::debug!(id = %c.id, ?state, ?role, "obligation listed");
        rows.push(Row {
            id: c.id.clone(),
            state,
            role,
            attester: att.map(|a| a.by.clone()),
            age_days,
            nearest_changed: c.nearest_changed.clone(),
            note,
            upstream: upstream.for_id(&c.id).map(|u| u.describe()).collect(),
        });
    }
    let pending: Vec<_> = rows.iter().map(|r| (r.id.clone(), r.state)).collect();
    let gated = !policy.gate(&pending, args.release).passed();
    tracing::info!(
        rows = rows.len(),
        gated,
        release = args.release,
        "obligations listed"
    );
    Report { rows, gated }
}

fn state_word(state: ObligationState) -> &'static str {
    match state {
        ObligationState::Stale => "stale",
        ObligationState::Open => "open",
        ObligationState::Discharged => "discharged",
        ObligationState::Violated => "violated",
        ObligationState::Undecided => "undecided",
        ObligationState::Waived => "waived",
    }
}

/// Render the report as text or the `cadetta.obligations/1` JSON document.
pub fn render(report: &Report, as_of: &AsOf, format: Format, out: &mut dyn Write) {
    if format == Format::Json {
        let rows: Vec<_> = report
            .rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id.to_string(),
                    "state": state_word(r.state),
                    "role": r.role,
                    "attester": r.attester,
                    "age_days": r.age_days,
                    "nearest_changed": r.nearest_changed,
                    "note": r.note,
                    "upstream": r.upstream,
                })
            })
            .collect();
        let doc = serde_json::json!({
            "schema": SCHEMA,
            "as_of": as_of.date.to_string(),
            "as_of_source": as_of.source.as_str(),
            "gated": report.gated,
            "obligations": rows,
        });
        let _ = writeln!(out, "{doc}");
        return;
    }
    for r in &report.rows {
        let mut line = format!(
            "{} {} role={}",
            state_word(r.state),
            r.id,
            r.role.as_deref().unwrap_or("-")
        );
        if let Some(by) = &r.attester {
            line.push_str(&format!(" last={by}"));
        }
        if let Some(age) = r.age_days {
            line.push_str(&format!(" age={age}d"));
        }
        if let Some(dep) = &r.nearest_changed {
            line.push_str(&format!(" changed={dep}"));
        }
        if let Some(note) = &r.note {
            line.push_str(&format!(" ({note})"));
        }
        for up in &r.upstream {
            line.push_str(&format!(" [attested upstream: {up}]"));
        }
        let _ = writeln!(out, "{line}");
    }
}

/// The obligations the current design offers, from the lowered packages (ir.md 4).
///
/// Each IR check is one candidate judged against its closure digest; a candidate whose
/// attestation (of the requested or first covering role) is stale also carries the nearest
/// changed upstream node (`why --stale`).
fn project_candidates(
    analysis: &Analysis,
    lock: &Lock,
    policy: &Policy,
    args: &ObligationsArgs,
) -> Vec<Candidate> {
    let universes = universes(analysis);
    let mut out = candidates(&universes);
    for cand in &mut out {
        let role = args.role.clone().or_else(|| {
            policy
                .roles_covering(&cand.id)
                .first()
                .map(|r| r.name.clone())
        });
        let att = role.as_deref().and_then(|r| lock.attestation(&cand.id, r));
        let Some((u, entry)) = universes
            .iter()
            .find_map(|u| u.entry(&cand.id).map(|e| (u, e)))
        else {
            continue;
        };
        if let Ok(WhyStale::Stale { nearest, .. }) = u.explain(entry, att) {
            cand.nearest_changed = Some(nearest.node);
        }
    }
    tracing::debug!(count = out.len(), "obligations: candidates from the IR");
    out
}

/// Read the lock of every path dependency and the roles each is trusted for (packages.md 6).
///
/// Git dependencies are not materialised here, so they contribute nothing.
fn upstream_of(project: &Project) -> Result<(Upstream, Trust), String> {
    let mut upstream = Upstream::new();
    let mut trust = Trust::new();
    for (name, dep) in &project.config.dependencies {
        trust.set(name, &dep.trust);
        let Some(path) = &dep.path else {
            tracing::debug!(dependency = %name, "no path: upstream lock not read");
            continue;
        };
        let lock = Lock::load(&project.root.join(path).join("cadetta.lock"))
            .map_err(|e| format!("dependency `{name}`: {e}"))?;
        upstream.add_package(name, &lock);
    }
    Ok((upstream, trust))
}

/// Run `cadetta obligations`; `args` are the words after the verb. Returns the exit code.
pub fn run_obligations(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "obligations failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(message) => return fail(err, &format!("{message}; {USAGE}")),
    };
    let (project, members) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref())
    {
        Ok(o) => o,
        Err(e) => return fail(err, &e.to_string()),
    };
    let policy = match policy_of(&project.config) {
        Ok(p) => p,
        Err(message) => return fail(err, &message),
    };
    if let Some(role) = &parsed.role
        && policy.role(role).is_none()
    {
        return fail(err, &format!("unknown role `{role}`"));
    }
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
    let analysis = analyze(&project, &members);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    let (upstream, trust) = match upstream_of(&project) {
        Ok(o) => o,
        Err(message) => return fail(err, &message),
    };
    let report = list_with_upstream(
        &project_candidates(&analysis, &lock, &policy, &parsed),
        &lock,
        &policy,
        &parsed,
        &as_of,
        &upstream,
        &trust,
    );
    render(&report, &as_of, globals.format, out);
    if report.gated { EXIT_PENDING } else { EXIT_OK }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_obligations::graph::{ObligationKind, Verdict};
    use cadetta_obligations::lock::{Date, Hex, Record};

    fn id(path: &str) -> ObligationId {
        ObligationId::new(ObligationKind::Check, path)
    }

    fn policy(required: Option<&str>) -> Policy {
        let mut p = Policy::new();
        p.add_role("review", &["check:a::**"], required, None, false)
            .unwrap();
        p
    }

    fn cand(path: &str, digest: &str) -> Candidate {
        Candidate {
            id: id(path),
            current: Digest(digest.into()),
            deps: vec![],
            judgement: Some(Judgement {
                verdict: Verdict::Holds,
                margin: "0.010mm".into(),
            }),
            suppressed: false,
            nearest_changed: Some("a::X.d".into()),
        }
    }

    fn attested(lock: &mut Lock, path: &str, digest: &str) {
        lock.insert(Record::Attest(Attest {
            id: id(path),
            role: "review".into(),
            by: "ana@x".into(),
            at: Date::new("2026-10-01").unwrap(),
            digest: Hex::new(digest).unwrap(),
            verdict: Verdict::Holds,
            margin: "0.012mm".into(),
            deps: vec![],
            reason: "ok".into(),
        }));
    }

    fn as_of() -> AsOf {
        AsOf::resolve(Some("2026-10-13"), None, Today::today()).unwrap()
    }

    fn lock() -> Lock {
        let mut l = Lock::new();
        attested(&mut l, "a::X.cur", "aa");
        attested(&mut l, "a::X.old", "bb");
        l
    }

    fn cands() -> Vec<Candidate> {
        vec![
            cand("a::X.cur", "aa"),
            cand("a::X.old", "cc"),
            cand("a::X.new", "dd"),
        ]
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::list
    #[test]
    fn lists_open_and_stale_with_role_and_note() {
        let r = list(
            &cands(),
            &lock(),
            &policy(None),
            &Default::default(),
            &as_of(),
        );
        let got: Vec<_> = r.rows.iter().map(|r| (r.id.path(), r.state)).collect();
        assert_eq!(
            got,
            [
                ("a::X.old", ObligationState::Stale),
                ("a::X.new", ObligationState::Open)
            ]
        );
        let stale = &r.rows[0];
        assert_eq!(stale.role.as_deref(), Some("review"));
        assert_eq!(stale.attester.as_deref(), Some("ana@x"));
        assert_eq!(stale.age_days, Some(12));
        assert_eq!(
            stale.note.as_deref(),
            Some("verdict unchanged, margin 0.012mm -> 0.010mm")
        );
        assert!(!r.gated);
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::render
    #[test]
    fn json_is_machine_readable() {
        let r = list(
            &cands(),
            &lock(),
            &policy(None),
            &Default::default(),
            &as_of(),
        );
        let mut out = Vec::new();
        render(&r, &as_of(), Format::Json, &mut out);
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["schema"], SCHEMA);
        assert_eq!(v["as_of"], "2026-10-13");
        assert_eq!(v["obligations"][0]["state"], "stale");
        assert_eq!(v["obligations"][1]["state"], "open");
        assert_eq!(v["obligations"][0]["role"], "review");
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::list
    #[test]
    fn stale_filter_and_exit_follow_required_for() {
        let args = ObligationsArgs {
            stale: true,
            ..Default::default()
        };
        let r = list(&cands(), &lock(), &policy(Some("check")), &args, &as_of());
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0].state, ObligationState::Stale);
        assert!(r.gated);
        // release-only role does not gate a plain run
        let r = list(&cands(), &lock(), &policy(Some("release")), &args, &as_of());
        assert!(!r.gated);
        let rel = ObligationsArgs {
            stale: true,
            release: true,
            ..Default::default()
        };
        assert!(list(&cands(), &lock(), &policy(Some("release")), &rel, &as_of()).gated);
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::list
    #[test]
    fn role_filter_drops_uncovered() {
        let args = ObligationsArgs {
            role: Some("nobody".into()),
            ..Default::default()
        };
        assert!(
            list(&cands(), &lock(), &policy(None), &args, &as_of())
                .rows
                .is_empty()
        );
    }

    fn shop_upstream() -> Upstream {
        let mut shop = Lock::new();
        attested(&mut shop, "a::X.cur", "aa");
        attested(&mut shop, "a::X.old", "bb");
        let mut u = Upstream::new();
        u.add_package("acme.shop", &shop);
        u
    }

    fn undiscounted() -> Vec<Candidate> {
        vec![cand("a::X.cur", "aa"), cand("a::X.old", "bb")]
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::list_with_upstream
    #[test]
    fn upstream_attestations_are_shown_as_attested_upstream() {
        let r = list_with_upstream(
            &undiscounted(),
            &Lock::new(),
            &policy(None),
            &Default::default(),
            &as_of(),
            &shop_upstream(),
            &Trust::new(),
        );
        assert_eq!(r.rows.len(), 2);
        assert_eq!(
            r.rows[0].upstream,
            ["acme.shop review by ana@x on 2026-10-01"]
        );
        let mut out = Vec::new();
        render(&r, &as_of(), Format::Text, &mut out);
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains("[attested upstream: acme.shop review by ana@x on 2026-10-01]"),
            "{text}"
        );
        let mut json = Vec::new();
        render(&r, &as_of(), Format::Json, &mut json);
        let v: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(
            v["obligations"][0]["upstream"][0],
            "acme.shop review by ana@x on 2026-10-01"
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::list_with_upstream
    #[test]
    fn trusted_roles_discharge_dependent_obligations() {
        let mut trust = Trust::new();
        trust.set("acme.shop", &["review".to_owned()]);
        let r = list_with_upstream(
            &undiscounted(),
            &Lock::new(),
            &policy(Some("check")),
            &Default::default(),
            &as_of(),
            &shop_upstream(),
            &trust,
        );
        assert!(r.rows.is_empty(), "{:?}", r.rows);
        assert!(!r.gated);
        // an upstream sign-off of an older state discharges nothing
        let stale = vec![cand("a::X.cur", "zz")];
        let r = list_with_upstream(
            &stale,
            &Lock::new(),
            &policy(Some("check")),
            &Default::default(),
            &as_of(),
            &shop_upstream(),
            &trust,
        );
        assert_eq!(r.rows.len(), 1);
        assert!(r.gated);
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::list_with_upstream
    #[test]
    fn without_trust_upstream_discharges_nothing() {
        let r = list_with_upstream(
            &undiscounted(),
            &Lock::new(),
            &policy(Some("check")),
            &Default::default(),
            &as_of(),
            &shop_upstream(),
            &Trust::new(),
        );
        assert_eq!(r.rows.len(), 2);
        assert!(r.rows.iter().all(|row| row.state == ObligationState::Open));
        assert!(r.gated);
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::upstream_of
    #[test]
    fn path_dependency_locks_and_trust_are_read() {
        let root = std::env::temp_dir().join(format!("cadetta-upstream-{}", std::process::id()));
        let shop = root.join("shop");
        std::fs::create_dir_all(shop.join("src")).unwrap();
        std::fs::write(shop.join("src/lib.cdt"), "").unwrap();
        std::fs::write(
            shop.join("cadetta.toml"),
            "[package]\nname = \"acme.shop\"\n",
        )
        .unwrap();
        std::fs::write(root.join("src.keep"), "").unwrap();
        let mut shop_lock = Lock::new();
        attested(&mut shop_lock, "a::X.cur", "aa");
        shop_lock.save(&shop.join("cadetta.lock")).unwrap();
        let config = crate::config::Config::parse(
            "[package]\nname = \"acme.app\"\n[dependencies.\"acme.shop\"]\npath = \"shop\"\ntrust = [\"review\"]\n",
        )
        .unwrap();
        let project = Project {
            root: root.clone(),
            config,
            packages: vec![],
            dependencies: vec![],
        };
        let (upstream, trust) = upstream_of(&project).unwrap();
        assert_eq!(upstream.for_id(&id("a::X.cur")).count(), 1);
        assert!(trust.trusts("acme.shop", "review"));
        let _ = std::fs::remove_dir_all(&root);
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::parse_args
    #[test]
    fn parses_flags() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let a = parse_args(&s(&[
            "--stale",
            "--role",
            "r",
            "--release",
            "--as-of",
            "2026-01-02",
        ]))
        .unwrap();
        assert!(a.stale && a.release);
        assert_eq!(a.role.as_deref(), Some("r"));
        assert_eq!(a.as_of.as_deref(), Some("2026-01-02"));
        assert!(parse_args(&s(&["--bogus"])).is_err());
        assert!(parse_args(&s(&["--role"])).is_err());
    }

    // frob:ticket 01M4SWN8G3QTQC5YK07VN1YQYC
    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::render
    #[test]
    fn json_validates_against_obligations_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/obligations.json"))
                .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let r = list(
            &cands(),
            &lock(),
            &policy(None),
            &Default::default(),
            &as_of(),
        );
        let mut out = Vec::new();
        render(&r, &as_of(), Format::Json, &mut out);
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert!(validator.is_valid(&v), "{v}");
    }
}
