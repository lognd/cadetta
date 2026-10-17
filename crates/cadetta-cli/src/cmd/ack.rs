//! frob:ticket 01M4ST8F1W8T0BKARAMG0D49J2
//! `cadetta ack SELECTOR --role R --reason "..."`: attest obligations at their current
//! closure digests, or record reviewed waivers (obligations.md 3-5, config-cli.md 3).

use std::io::Write;
use std::path::Path;

use cadetta_ir::model::{Citation, Digest};
use cadetta_obligations::as_of::Date as Today;
use cadetta_obligations::citation::{SOURCE_ROLE, ack_citation, citation_id};
use cadetta_obligations::graph::{ObligationId, ObligationKind};
use cadetta_obligations::kinds::RequiredFor;
use cadetta_obligations::lock::{AltBinding, Date, Hex, Lock, Record, Waiver};
use cadetta_obligations::roles::Policy;
use cadetta_obligations::selector::Selector;
use cadetta_obligations::staleness::{Judgement, Signer, record_attest};

use crate::config::{Config, Gate};
use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::Project;
use crate::universe::{ack_candidates, sync_candidates, universes, waiver_candidates};
use crate::{EXIT_OK, EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta ack SELECTOR --role R --reason \"...\" \
                     [--baseline] [--release] [--dry-run] [--as EMAIL]";

/// The reason recorded by `--baseline` (obligations.md 3, 4).
pub const BASELINE_REASON: &str = "baseline";

/// The parsed command line of `cadetta ack`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct AckArgs {
    /// Obligation selector `KIND:GLOB[,KIND:GLOB...]`.
    pub selector: String,
    /// Role attesting.
    pub role: String,
    /// Free-text reason; `baseline` with `--baseline`; may be empty for waivers.
    pub reason: Option<String>,
    /// Adopt matching open obligations at current digests without review.
    pub baseline: bool,
    /// Limit to obligations that gate `check --release` (decided, obligations.md 4).
    pub release: bool,
    /// Report what would be recorded and write nothing.
    pub dry_run: bool,
    /// Attester identity overriding `git config user.email`.
    pub as_email: Option<String>,
}

/// Parse the words after `ack`; the error is a usage message.
pub fn parse_args(args: &[String]) -> Result<AckArgs, String> {
    let mut parsed = AckArgs::default();
    let mut selector = None;
    let mut role = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("`{flag}` needs a value"))
        };
        match arg.as_str() {
            "--role" => role = Some(value("--role")?),
            "--reason" => parsed.reason = Some(value("--reason")?),
            "--as" => parsed.as_email = Some(value("--as")?),
            "--baseline" => parsed.baseline = true,
            "--release" => parsed.release = true,
            "--dry-run" => parsed.dry_run = true,
            flag if flag.starts_with("--") => return Err(format!("unknown flag `{flag}`")),
            sel if selector.is_none() => selector = Some(sel.to_owned()),
            extra => return Err(format!("unexpected argument `{extra}`")),
        }
    }
    parsed.selector = selector.ok_or("missing SELECTOR")?;
    parsed.role = role.ok_or("missing `--role`")?;
    Ok(parsed)
}

/// What a candidate obligation records when acknowledged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// A sign-off: the covered closure, the deps' own digests and the verdict signed.
    Attest {
        /// Closure digest of the covered set (`covered_digest`).
        covered: Digest,
        /// Own digests of every transitive dep.
        deps: Vec<Digest>,
        /// Verdict and margin at ack time.
        judgement: Judgement,
        /// File citations of an `ots` also acknowledged under role `source`.
        citations: Vec<Citation>,
    },
    /// A reviewed waiver: its code, optional alternate and its targets' closure digest.
    Waiver {
        /// Waived diagnostic code.
        code: String,
        /// Alternate restriction.
        alt: Option<AltBinding>,
        /// Closure digest of the targets.
        digest: Digest,
    },
}

/// One obligation the current design offers for acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Obligation id (a waiver's path is its target glob).
    pub id: ObligationId,
    /// What acknowledging it records.
    pub subject: Subject,
}

/// Everything `ack` needs from its surroundings, injected so tests need no project.
pub struct AckEnv<'a> {
    /// Roles and kind policy from `[obligations]`.
    pub policy: &'a Policy,
    /// Obligations offered by the current design.
    pub candidates: &'a [Candidate],
    /// Resolved attester identity.
    pub by: &'a str,
    /// Today's date.
    pub today: Date,
    /// Author of the newest commit that changed the covered nodes (`None` = unknown).
    pub author: &'a dyn Fn(&ObligationId) -> Option<String>,
}

/// What `ack` did to one obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// A record was written or replaced.
    Recorded,
    /// The lock already held a record at the current digest; nothing changed.
    Unchanged,
    /// Skipped by `--baseline` (already attested) or `--release` (does not gate release).
    Skipped,
}

impl Action {
    fn word(self) -> &'static str {
        match self {
            Action::Recorded => "recorded",
            Action::Unchanged => "unchanged",
            Action::Skipped => "skipped",
        }
    }
}

/// A refusal; always exit code 2.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AckError {
    /// Bad selector, no match, undeclared role or other usage problem.
    #[error("{0}")]
    Usage(String),
}

fn usage(message: impl Into<String>) -> AckError {
    let message = message.into();
    tracing::warn!(%message, "ack refused");
    AckError::Usage(message)
}

fn hex(d: &Digest) -> Result<Hex, AckError> {
    Hex::new(&d.0).map_err(AckError::Usage)
}

/// Whether the role named `role` gates `check --release` (required for check or release).
fn gates_release(policy: &Policy, role: &str) -> bool {
    policy.role(role).is_some_and(|r| {
        matches!(
            r.required_for,
            Some(RequiredFor::Check | RequiredFor::Release)
        )
    })
}

/// Acknowledge the obligations matching `args` into `lock`; returns one line per match.
///
/// Atomic: any refusal (selector, role, coverage, membership) leaves `lock` untouched. A
/// repeat at unchanged digests changes nothing, not even the date (obligations.md 3).
pub fn ack(
    args: &AckArgs,
    env: &AckEnv<'_>,
    lock: &mut Lock,
) -> Result<Vec<(ObligationId, Action)>, AckError> {
    let selector = Selector::parse(&args.selector).map_err(|e| usage(e.to_string()))?;
    if env.policy.role(&args.role).is_none() {
        return Err(usage(format!("role `{}` is not declared", args.role)));
    }
    let ids: Vec<&ObligationId> = env.candidates.iter().map(|c| &c.id).collect();
    let matched = selector.resolve(ids).map_err(|e| usage(e.to_string()))?;
    let reason = match (&args.reason, args.baseline) {
        (_, true) => BASELINE_REASON.to_owned(),
        (Some(r), false) => r.clone(),
        (None, false) if matched.iter().all(|i| i.kind() == ObligationKind::Waiver) => {
            String::new()
        }
        (None, false) => return Err(usage("missing `--reason` (or use `--baseline`)")),
    };
    let signer = |role: &str| Signer {
        role: role.to_owned(),
        by: env.by.to_owned(),
        at: env.today.clone(),
        reason: reason.clone(),
    };
    // Plan on a copy so a refusal midway leaves the caller's lock unchanged.
    let mut next = lock.clone();
    let mut done = Vec::new();
    for cand in env.candidates.iter().filter(|c| matched.contains(&c.id)) {
        env.policy
            .authorize(
                &args.role,
                &cand.id,
                env.by,
                (env.author)(&cand.id).as_deref(),
            )
            .map_err(|e| usage(e.to_string()))?;
        let existing = next.attestation(&cand.id, &args.role).cloned();
        let action = match &cand.subject {
            Subject::Attest {
                covered,
                deps,
                judgement,
                citations,
            } => {
                let current = existing
                    .as_ref()
                    .is_some_and(|a| a.digest.as_str() == covered.0);
                if (args.baseline && existing.is_some())
                    || (args.release && !gates_release(env.policy, &args.role))
                {
                    Action::Skipped
                } else if current {
                    Action::Unchanged
                } else {
                    let rec = record_attest(
                        cand.id.clone(),
                        signer(&args.role),
                        judgement.clone(),
                        covered,
                        deps,
                    )
                    .map_err(AckError::Usage)?;
                    next.remove(&Record::Attest(rec.clone()).key());
                    next.insert(Record::Attest(rec));
                    ack_citations(citations, &signer(SOURCE_ROLE), &mut next)?;
                    Action::Recorded
                }
            }
            Subject::Waiver { code, alt, digest } => {
                let rec = Record::Waiver(Waiver {
                    code: code.clone(),
                    glob: cand.id.path().to_owned(),
                    alt: alt.clone(),
                    digest: hex(digest)?,
                });
                if next.get(&rec.key()) == Some(&rec) {
                    Action::Unchanged
                } else {
                    next.remove(&rec.key());
                    next.insert(rec);
                    Action::Recorded
                }
            }
        };
        tracing::debug!(id = %cand.id, role = %args.role, action = action.word(), "ack decided");
        done.push((cand.id.clone(), action));
    }
    *lock = next;
    Ok(done)
}

/// Record `attest citation:PATH` (role `source`) for each cite item not already current.
// frob:ticket 01M4VB63M63PQVRKN0HDXQP567
fn ack_citations(citations: &[Citation], signer: &Signer, lock: &mut Lock) -> Result<(), AckError> {
    for c in citations {
        let Ok(rec) = ack_citation(c, signer.clone()) else {
            tracing::debug!(path = %c.path, "citation could not be recorded; skipped");
            continue;
        };
        let id = citation_id(&c.path);
        if lock
            .attestation(&id, SOURCE_ROLE)
            .is_some_and(|a| a.digest == rec.digest)
        {
            continue;
        }
        lock.remove(&Record::Attest(rec.clone()).key());
        lock.insert(Record::Attest(rec));
    }
    Ok(())
}

/// Build the role policy from `[obligations]`; the error is a config message.
pub(crate) fn policy_of(config: &Config) -> Result<Policy, String> {
    let mut policy = Policy::new();
    for (name, role) in &config.obligations.roles {
        let covers: Vec<&str> = role.covers.iter().map(String::as_str).collect();
        let gate = role.required_for.map(|g| match g {
            Gate::Check => "check",
            Gate::Release => "release",
        });
        policy
            .add_role(
                name,
                &covers,
                gate,
                role.members.clone(),
                role.distinct_from_author,
            )
            .map_err(|e| e.to_string())?;
    }
    for (kind, k) in &config.obligations.kinds {
        if let Some(g) = k.required_for {
            let g = if g == Gate::Check { "check" } else { "release" };
            policy.kinds.set(kind, g).map_err(|e| e.to_string())?;
        }
    }
    Ok(policy)
}

/// The `email` under `[user]` in git-config text, or `None`.
fn email_in(text: &str) -> Option<String> {
    let mut in_user = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_user = line.trim_end_matches(']').trim_start_matches('[').trim() == "user";
        } else if in_user && let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"');
            if key.trim().eq_ignore_ascii_case("email") && !value.is_empty() {
                return Some(value.to_owned());
            }
        }
    }
    None
}

/// The git `user.email`: the repository's `.git/config`, else `~/.gitconfig`.
///
/// Read directly so `ack` never spawns a process; `None` when neither sets it.
pub(crate) fn git_email(dir: &Path) -> Option<String> {
    let home = crate::env::home_dir();
    let candidates = dir
        .ancestors()
        .map(|d| d.join(".git").join("config"))
        .chain(home.map(|h| h.join(".gitconfig")));
    candidates
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .find_map(|t| email_in(&t))
}

fn render(
    results: &[(ObligationId, Action)],
    args: &AckArgs,
    wrote: bool,
    format: Format,
    out: &mut dyn Write,
) {
    if format == Format::Json {
        let records: Vec<_> = results
            .iter()
            .map(|(id, a)| serde_json::json!({"id": id.to_string(), "action": a.word()}))
            .collect();
        let doc = serde_json::json!({
            "schema": "cadetta.ack/1",
            "role": args.role,
            "dry_run": args.dry_run,
            "written": wrote,
            "obligations": records,
        });
        let _ = writeln!(out, "{doc}");
        return;
    }
    for (id, action) in results {
        let _ = writeln!(out, "{} {id} role={}", action.word(), args.role);
    }
}

/// Run `cadetta ack`; `args` are the words after `ack`. Returns the exit code.
pub fn run_ack(args: &[String], globals: &Globals, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "ack failed");
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
    let Some(by) = parsed.as_email.clone().or_else(|| git_email(&project.root)) else {
        return fail(
            err,
            "cannot determine the attester: set git user.email or pass `--as`",
        );
    };
    let lock_path = project.root.join("cadetta.lock");
    let mut lock = match Lock::load(&lock_path) {
        Ok(l) => l,
        Err(e) => return fail(err, &e.to_string()),
    };
    let today = match Date::new(&Today::today().to_string()) {
        Ok(d) => d,
        Err(message) => return fail(err, &message),
    };
    let analysis = analyze(&project, &members);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    let universes = universes(&analysis);
    let mut candidates = ack_candidates(&universes);
    candidates.extend(waiver_candidates(&analysis, &universes));
    candidates.extend(sync_candidates(&lock));
    let author = |_: &ObligationId| None;
    let env = AckEnv {
        policy: &policy,
        candidates: &candidates,
        by: &by,
        today,
        author: &author,
    };
    let results = match ack(&parsed, &env, &mut lock) {
        Ok(r) => r,
        Err(AckError::Usage(message)) => return fail(err, &message),
    };
    let wrote = if parsed.dry_run {
        false
    } else {
        match lock.save(&lock_path) {
            Ok(w) => w,
            Err(e) => return fail(err, &e.to_string()),
        }
    };
    tracing::info!(
        count = results.len(),
        wrote,
        dry_run = parsed.dry_run,
        "ack done"
    );
    render(&results, &parsed, wrote, globals.format, out);
    EXIT_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_obligations::graph::Verdict;
    use cadetta_obligations::lock::Attest;

    fn policy() -> Policy {
        let mut p = Policy::new();
        p.add_role(
            "review",
            &["check:a::**", "waiver:a::**"],
            Some("release"),
            None,
            false,
        )
        .unwrap();
        p.add_role(
            "lead",
            &["check:a::**"],
            None,
            Some(vec!["boss@x".into()]),
            false,
        )
        .unwrap();
        p
    }

    fn check(path: &str, digest: &str) -> Candidate {
        Candidate {
            id: ObligationId::new(ObligationKind::Check, path),
            subject: Subject::Attest {
                covered: Digest(digest.into()),
                deps: vec![Digest("ab".into())],
                judgement: Judgement {
                    verdict: Verdict::Holds,
                    margin: "1mm".into(),
                },
                citations: Vec::new(),
            },
        }
    }

    fn args(sel: &str, role: &str) -> AckArgs {
        AckArgs {
            selector: sel.into(),
            role: role.into(),
            reason: Some("reviewed".into()),
            ..AckArgs::default()
        }
    }

    fn run(
        a: &AckArgs,
        cands: &[Candidate],
        lock: &mut Lock,
        today: &str,
    ) -> Result<Vec<(ObligationId, Action)>, AckError> {
        let p = policy();
        let author = |_: &ObligationId| None;
        let env = AckEnv {
            policy: &p,
            candidates: cands,
            by: "ana@x",
            today: Date::new(today).unwrap(),
            author: &author,
        };
        ack(a, &env, lock)
    }

    fn attest_of(lock: &Lock, path: &str, role: &str) -> Option<Attest> {
        lock.attestation(&ObligationId::new(ObligationKind::Check, path), role)
            .cloned()
    }

    // frob:ticket 01M52TH35NQBTKBHMBH6WF08GT
    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::ack
    #[test]
    fn sync_file_candidate_is_attested_under_its_role() {
        let mut p = policy();
        p.add_role("cad", &["sync:**"], None, None, false).unwrap();
        let cand = Candidate {
            id: ObligationId::new(ObligationKind::Sync, "part.FCStd"),
            ..check("x", "0a")
        };
        let author = |_: &ObligationId| None;
        let env = AckEnv {
            policy: &p,
            candidates: std::slice::from_ref(&cand),
            by: "ana@x",
            today: Date::new("2026-10-16").unwrap(),
            author: &author,
        };
        let mut lock = Lock::new();
        let a = args("sync:part.FCStd", "cad");
        let first = ack(&a, &env, &mut lock).unwrap();
        assert_eq!(first[0].1, Action::Recorded);
        let id = ObligationId::new(ObligationKind::Sync, "part.FCStd");
        assert!(lock.attestation(&id, "cad").is_some());
        let again = ack(&a, &env, &mut lock).unwrap();
        assert_eq!(again[0].1, Action::Unchanged);
    }

    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::ack
    #[test]
    fn ack_attests_at_current_closure_digest() {
        let mut lock = Lock::new();
        let r = run(
            &args("check:a::X.*", "review"),
            &[check("a::X.len", "aa")],
            &mut lock,
            "2026-10-13",
        )
        .unwrap();
        assert_eq!(r[0].1, Action::Recorded);
        let a = attest_of(&lock, "a::X.len", "review").unwrap();
        assert_eq!(a.digest.as_str(), "aa");
        assert_eq!(a.by, "ana@x");
        assert_eq!(a.reason, "reviewed");
        assert_eq!(a.deps.len(), 1);
        // A changed digest re-attests even on the same day.
        run(
            &args("check:a::X.*", "review"),
            &[check("a::X.len", "bb")],
            &mut lock,
            "2026-10-13",
        )
        .unwrap();
        assert_eq!(
            attest_of(&lock, "a::X.len", "review")
                .unwrap()
                .digest
                .as_str(),
            "bb"
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::ack
    #[test]
    fn no_match_or_undeclared_role_is_a_usage_error_and_writes_nothing() {
        let mut lock = Lock::new();
        let c = [check("a::X.len", "aa")];
        let e = run(&args("check:b::*", "review"), &c, &mut lock, "2026-10-13").unwrap_err();
        assert!(e.to_string().contains("matches no obligation"), "{e}");
        let e = run(&args("check:a::**", "ghost"), &c, &mut lock, "2026-10-13").unwrap_err();
        assert!(e.to_string().contains("not declared"), "{e}");
        let e = run(&args("check:a::**", "lead"), &c, &mut lock, "2026-10-13").unwrap_err();
        assert!(e.to_string().contains("not a member"), "{e}");
        assert!(lock.is_empty());
    }

    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::ack
    #[test]
    fn repeat_at_same_digest_changes_nothing() {
        let mut lock = Lock::new();
        let c = [check("a::X.len", "aa")];
        run(&args("check:a::**", "review"), &c, &mut lock, "2026-10-13").unwrap();
        let before = lock.render();
        let r = run(&args("check:a::**", "review"), &c, &mut lock, "2026-11-01").unwrap();
        assert_eq!(r[0].1, Action::Unchanged);
        assert_eq!(lock.render(), before);
    }

    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::ack
    #[test]
    fn baseline_uses_reason_baseline_and_skips_attested() {
        let mut lock = Lock::new();
        let mut a = args("check:a::**", "review");
        a.reason = None;
        a.baseline = true;
        let both = [check("a::X.len", "aa"), check("a::X.dia", "cc")];
        run(
            &args("check:a::X.len", "review"),
            &both[..1],
            &mut lock,
            "2026-10-12",
        )
        .unwrap();
        let r = run(&a, &both, &mut lock, "2026-10-13").unwrap();
        assert_eq!(r[0].1, Action::Skipped);
        assert_eq!(r[1].1, Action::Recorded);
        assert_eq!(
            attest_of(&lock, "a::X.dia", "review").unwrap().reason,
            "baseline"
        );
        assert_eq!(
            attest_of(&lock, "a::X.len", "review").unwrap().reason,
            "reviewed"
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::ack
    #[test]
    fn waiver_records_target_digest_without_a_reason() {
        let mut lock = Lock::new();
        let w = Candidate {
            id: ObligationId::new(ObligationKind::Waiver, "a::X.*"),
            subject: Subject::Waiver {
                code: "C001".into(),
                alt: None,
                digest: Digest("dd".into()),
            },
        };
        let mut a = args("waiver:a::X.*", "review");
        a.reason = None;
        let r = run(&a, std::slice::from_ref(&w), &mut lock, "2026-10-13").unwrap();
        assert_eq!(r[0].1, Action::Recorded);
        assert!(
            lock.render().contains("waiver C001 a::X.* digest=dd"),
            "{}",
            lock.render()
        );
        let r = run(&a, &[w], &mut lock, "2026-10-14").unwrap();
        assert_eq!(r[0].1, Action::Unchanged);
    }

    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::parse_args
    #[test]
    fn parses_flags_and_requires_selector_and_role() {
        let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        let a = parse_args(&s(&[
            "ots:p",
            "--role",
            "source",
            "--as",
            "a@b",
            "--dry-run",
            "--release",
        ]))
        .unwrap();
        assert_eq!((a.selector.as_str(), a.role.as_str()), ("ots:p", "source"));
        assert!(a.dry_run && a.release && a.as_email.as_deref() == Some("a@b"));
        assert!(parse_args(&s(&["ots:p"])).is_err());
        assert!(parse_args(&s(&["--role", "r"])).is_err());
        assert!(parse_args(&s(&["x", "--role", "r", "--bogus"])).is_err());
    }

    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::email_in
    #[test]
    fn email_is_read_from_the_user_section() {
        let text = "[core]\n\temail = no@x\n[user]\n\tname = A\n\temail = ana@x\n";
        assert_eq!(email_in(text).as_deref(), Some("ana@x"));
        assert_eq!(email_in("[user]\n\tname = A\n"), None);
    }

    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::run_ack
    #[test]
    fn selector_matching_nothing_exits_two() {
        let dir = std::env::temp_dir().join(format!("cadetta-ack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("cadetta.toml"),
            "[package]\nname = \"acme.t\"\nversion = \"0.1.0\"\n\n\
             [obligations.roles.source]\ncovers = [\"ots:*\"]\n",
        )
        .unwrap();
        std::fs::write(dir.join("src/lib.cdt"), "").unwrap();
        let globals = Globals {
            format: Format::Text,
            cwd: Some(dir.clone()),
            config: None,
        };
        let a: Vec<String> = ["ots:*", "--role", "source", "--reason", "r", "--as", "a@b"]
            .map(str::to_owned)
            .to_vec();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run_ack(&a, &globals, &mut out, &mut err), EXIT_USAGE);
        assert!(
            String::from_utf8(err)
                .unwrap()
                .contains("matches no obligation")
        );
        assert!(!dir.join("cadetta.lock").exists());
    }

    // frob:ticket 01M4SWN8G3QTQC5YK07VN1YQYC
    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::render
    #[test]
    fn json_validates_against_ack_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/ack.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let results = vec![
            (
                ObligationId::new(ObligationKind::Check, "a::X.cur"),
                Action::Recorded,
            ),
            (
                ObligationId::new(ObligationKind::Check, "a::X.old"),
                Action::Skipped,
            ),
        ];
        let mut out = Vec::new();
        render(
            &results,
            &args("check:a::**", "review"),
            true,
            Format::Json,
            &mut out,
        );
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert!(validator.is_valid(&v), "{v}");
    }
}
