//! frob:ticket 01M4ST8F3SDF95MVP07VXXNX0X
//! `cadetta obligations [--stale] [--role R] [--release] [--as-of DATE]`: the design's
//! to-do list of open and stale obligations (obligations.md 5, config-cli.md 3).

use std::io::Write;
use std::path::Path;

use cadetta_ir::model::Digest;
use cadetta_obligations::as_of::{AsOf, Date as Today};
use cadetta_obligations::graph::{ObligationId, ObligationState};
use cadetta_obligations::lock::{Attest, Lock};
use cadetta_obligations::roles::Policy;
use cadetta_obligations::staleness::{Judgement, classify_attested};
use cadetta_obligations::trust::{Trust, Upstream};
use cadetta_obligations::why_stale::WhyStale;

use super::ack::policy_of;
use super::update::{checkout_dir, git_checkout, roots_of};
use crate::output::Format;
use crate::pipeline::{Analysis, analyze};
use crate::pkg::manifest::GitRef;
use crate::pkg::resolve;
use crate::project::{Dependency, PackageDir, Project};
use crate::universe::{candidates, sync_states, universes, waiver_states};
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

/// Append rows for obligations judged by state alone (`waiver`, `sync`) and re-gate the report.
// frob:ticket 01M4Y9Z3KDPFRNGQR1KTNGHA7Q
fn add_state_rows(
    report: &mut Report,
    states: Vec<(ObligationId, ObligationState)>,
    policy: &Policy,
    args: &ObligationsArgs,
) {
    for (id, state) in states {
        if args.stale && state != ObligationState::Stale {
            continue;
        }
        tracing::debug!(%id, ?state, "obligation listed");
        report.rows.push(Row {
            id,
            state,
            role: None,
            attester: None,
            age_days: None,
            nearest_changed: None,
            note: None,
            upstream: Vec::new(),
        });
    }
    let pending: Vec<_> = report
        .rows
        .iter()
        .map(|r| (r.id.clone(), r.state))
        .collect();
    report.gated = !policy.gate(&pending, args.release).passed();
}

/// Read the lock of every dependency and the roles each is trusted for (packages.md 6).
///
/// A path dependency is read in place; a git dependency is read from the checkout that
/// [`attach_git_dependencies`] materialised into `project.dependencies`.
fn upstream_of(project: &Project) -> Result<(Upstream, Trust), String> {
    let mut upstream = Upstream::new();
    let mut trust = Trust::new();
    for (name, dep) in &project.config.dependencies {
        trust.set(name, &dep.trust);
        let dir = match &dep.path {
            Some(path) => project.root.join(path),
            None => match project
                .dependencies
                .iter()
                .find(|d| &d.resolved.name == name)
            {
                Some(d) => d.resolved.dir.clone(),
                None => {
                    tracing::debug!(dependency = %name, "git dependency not materialised: upstream lock not read");
                    continue;
                }
            },
        };
        let lock = Lock::load(&dir.join("cadetta.lock"))
            .map_err(|e| format!("dependency `{name}`: {e}"))?;
        tracing::debug!(dependency = %name, dir = %dir.display(), "upstream lock read");
        upstream.add_package(name, &lock);
    }
    Ok((upstream, trust))
}

/// Materialise git dependencies (and their closure) into `project.dependencies`.
///
/// `check` never touches the network, so git packages are absent from the project; here an
/// existing checkout under `build/deps` is reused as-is (refresh it with `cadetta update`)
/// and a missing one is cloned. The new packages join the analysis universe so upstream
/// matching by obligation id sees their nodes.
// frob:ticket 01M4Y53MB9VXXCQB6FHQ06ZV6Q
fn attach_git_dependencies(project: &mut Project) -> Result<(), String> {
    if !project
        .config
        .dependencies
        .values()
        .any(|d| d.git.is_some())
    {
        return Ok(());
    }
    let roots = roots_of(project)?;
    let cache = project.root.join("build").join("deps");
    let read = |p: &Path| std::fs::read_to_string(p).ok();
    let fetch = |url: &str, r: &GitRef| {
        let dir = checkout_dir(&cache, url, r);
        if dir.join(".git").is_dir() {
            tracing::info!(%url, dir = %dir.display(), "reusing cached git dependency");
            Ok(dir)
        } else {
            git_checkout(&cache, url, r)
        }
    };
    let io = resolve::Io {
        read: &read,
        fetch: &fetch,
    };
    let resolution = resolve::resolve(&roots, &io).map_err(|e| e.to_string())?;
    for resolved in resolution.packages {
        if project
            .dependencies
            .iter()
            .any(|d| d.resolved.name == resolved.name)
        {
            continue;
        }
        let package = PackageDir::load(&resolved.dir).map_err(|e| e.to_string())?;
        tracing::info!(name = %resolved.name, "git dependency joins the analysis");
        project.dependencies.push(Dependency { resolved, package });
    }
    Ok(())
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
    let (mut project, members) =
        match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
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
    if let Err(message) = attach_git_dependencies(&mut project) {
        return fail(err, &message);
    }
    let analysis = analyze(&project, &members);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    let (upstream, trust) = match upstream_of(&project) {
        Ok(o) => o,
        Err(message) => return fail(err, &message),
    };
    let mut report = list_with_upstream(
        &project_candidates(&analysis, &lock, &policy, &parsed),
        &lock,
        &policy,
        &parsed,
        &as_of,
        &upstream,
        &trust,
    );
    let mut states = waiver_states(&analysis, &universes(&analysis), &lock);
    states.extend(sync_states(&project, &analysis, &lock));
    add_state_rows(&mut report, states, &policy, &parsed);
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

    const AXLE: &str = "model Axle {\n  od: Length = 20mm\n  let bore: Length = 8mm\n  check bore_ok: bore < od \"bore fits\"\n}\n";
    const CHECK: &str = "check:acme.axle::Axle.bore_ok";
    const ROLES: &str = "[obligations.roles.review]\ncovers = [\"check:acme.axle::**\"]\n";

    /// A scratch tree holding a git dependency repo `dep` (attested) and a dependent `app`.
    struct GitWorld(std::path::PathBuf);

    impl GitWorld {
        fn new(trust: &str) -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let root =
                std::env::temp_dir().join(format!("cadetta-gitdep-{}-{n}", std::process::id()));
            let (dep, app) = (root.join("dep"), root.join("app"));
            for d in [&dep, &app] {
                std::fs::create_dir_all(d.join("src")).unwrap();
            }
            std::fs::write(
                dep.join("cadetta.toml"),
                format!("[package]\nname = \"acme.axle\"\nversion = \"0.1.0\"\n{ROLES}"),
            )
            .unwrap();
            std::fs::write(dep.join("src/lib.cdt"), AXLE).unwrap();
            let globals = |cwd: &Path| Globals {
                format: Format::Json,
                cwd: Some(cwd.to_owned()),
                config: None,
            };
            let ack = ["--role", "review", "--reason", "ok", "--as", "ana@x"];
            let mut args = vec![CHECK.to_owned()];
            args.extend(ack.iter().map(|a| (*a).to_owned()));
            let code =
                crate::cmd::ack::run_ack(&args, &globals(&dep), &mut Vec::new(), &mut Vec::new());
            assert_eq!(code, EXIT_OK);
            let world = Self(root.clone());
            world.git(&["init", "--quiet"]);
            world.git(&["add", "."]);
            world.git(&["commit", "--quiet", "-m", "dep"]);
            std::fs::write(
                app.join("cadetta.toml"),
                format!(
                    "[package]\nname = \"acme.app\"\n{ROLES}[dependencies.\"acme.axle\"]\ngit = \"{}\"\n{trust}",
                    dep.display()
                ),
            )
            .unwrap();
            std::fs::write(app.join("src/lib.cdt"), "").unwrap();
            world
        }

        /// Run `git` in the dependency repo, failing the test on a non-zero exit.
        fn git(&self, args: &[&str]) {
            let st = std::process::Command::new("git")
                .arg("-C")
                .arg(self.0.join("dep"))
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@x",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .output()
                .unwrap();
            assert!(
                st.status.success(),
                "{}",
                String::from_utf8_lossy(&st.stderr)
            );
        }

        /// The closure digest of the dependency's check as the dependent's analysis sees it.
        fn dependent_digest(&self) -> String {
            let (mut project, members) = Project::open(Some(&self.0.join("app")), None).unwrap();
            attach_git_dependencies(&mut project).unwrap();
            let analysis = analyze(&project, &members);
            let policy = policy_of(&project.config).unwrap();
            project_candidates(
                &analysis,
                &Lock::new(),
                &policy,
                &ObligationsArgs::default(),
            )
            .into_iter()
            .find(|c| c.id.to_string() == CHECK)
            .unwrap()
            .current
            .0
        }

        fn run(&self) -> serde_json::Value {
            let globals = Globals {
                format: Format::Json,
                cwd: Some(self.0.join("app")),
                config: None,
            };
            let (mut out, mut err) = (Vec::new(), Vec::new());
            run_obligations(&[], &globals, &mut out, &mut err);
            serde_json::from_slice(&out)
                .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&err)))
        }
    }

    impl Drop for GitWorld {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::run_obligations
    #[test]
    fn git_dependency_lock_shows_attested_upstream() {
        let w = GitWorld::new("");
        let v = w.run();
        let row = v["obligations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == CHECK)
            .unwrap_or_else(|| panic!("{v}"));
        let up = row["upstream"][0].as_str().unwrap_or_else(|| panic!("{v}"));
        assert!(up.starts_with("acme.axle review by ana@x"), "{up}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::run_obligations
    #[test]
    fn git_dependency_trusted_role_discharges_dependent_obligation() {
        let w = GitWorld::new("trust = [\"review\"]\n");
        // The dependency attests the digest its check has in the dependent's universe.
        let digest = w.dependent_digest();
        let mut lock = Lock::new();
        attested(&mut lock, "acme.axle::Axle.bore_ok", &digest);
        lock.save(&w.0.join("dep/cadetta.lock")).unwrap();
        w.git(&["commit", "--quiet", "-am", "reattest"]);
        let _ = std::fs::remove_dir_all(w.0.join("app/build"));
        let v = w.run();
        let rows = v["obligations"].as_array().unwrap();
        assert!(rows.iter().all(|r| r["id"] != CHECK), "{v}");
    }

    // frob:tests crates/cadetta-cli/src/universe.rs::Universe.of_with_deps
    #[test]
    fn dependency_own_ack_digest_matches_the_dependent_closure_digest() {
        let w = GitWorld::new("");
        let lock = Lock::load(&w.0.join("dep/cadetta.lock")).unwrap();
        let own = lock
            .attestation(&id("acme.axle::Axle.bore_ok"), "review")
            .expect("dependency ack recorded")
            .digest
            .as_str()
            .to_owned();
        assert_eq!(own, w.dependent_digest());
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::attach_git_dependencies
    #[test]
    fn git_dependency_nodes_join_the_candidate_universe() {
        let w = GitWorld::new("");
        let before = w.0.join("app/build/deps");
        assert!(!before.exists());
        let v = w.run();
        assert!(before.is_dir(), "checkout is cached for reuse");
        assert!(
            v["obligations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == CHECK),
            "{v}"
        );
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
