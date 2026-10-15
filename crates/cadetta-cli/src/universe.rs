//! frob:ticket 01M4TA3CEMSKPPZTY1CM2AFDDQ
//! The obligation universe of a lowered package: the digest graph over its members, solvers,
//! parts and checks, and the check obligations with their closure digests (ir.md 4,
//! obligations.md 2 and 3).
//!
//! `check`, `obligations`, `ack` and `why --stale` all judge attestations against these digests,
//! so they are built in one place. A check reads the members whose path occurs in its expression
//! (the same rule `why` and `impact` use); its covered closure is the check's own digest over
//! the closures of those members, so editing the expression or anything upstream stales it.

use std::collections::BTreeSet;

use cadetta_ir::digest::{DigestError, DigestGraph, check_digest};
use cadetta_ir::model::{
    Check, Citation, ComponentClass, Digest, Project as IrProject, VerdictState,
};
use cadetta_obligations::citation::citation_id;
use cadetta_obligations::graph::{ObligationId, ObligationKind, ObligationState, Verdict};
use cadetta_obligations::lock::{Attest, Lock, Record};
use cadetta_obligations::staleness::Judgement;
use cadetta_obligations::why_stale::{WhyStale, why_stale};

use crate::cmd::ack::{Candidate as AckCandidate, Subject};
use crate::cmd::emit::{
    attr_point, config_point, kind_of, load_bindings, locators_for, native_unit, negate_text,
    written_for,
};
use crate::cmd::obligations::Candidate;
use crate::cmd::sync::normalize;
use crate::config::Adapter;
use crate::pipeline::Analysis;
use crate::project::{PackageDir, Project};

/// Whether a check at `check` with expression `expr` reads the member at `member`: its full
/// path occurs in the expression, or it lives in the check's scope and its name is an
/// identifier token (lowered expressions use bare names for same-scope reads).
pub fn check_reads(check: &str, expr: &str, member: &str) -> bool {
    if expr.contains(member) {
        return true;
    }
    let scope = |p: &str| p.rfind('.').map(|i| p[..=i].to_owned());
    let Some(prefix) = scope(check) else {
        return false;
    };
    let Some(name) = member.strip_prefix(prefix.as_str()) else {
        return false;
    };
    !name.contains(['.', ':'])
        && expr
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|tok| tok == name)
}

/// The margin text recorded for a check: the IR carries verdicts, not margins.
const NO_MARGIN: &str = "-";

/// One `check` obligation with the digests it is judged against.
#[derive(Debug, Clone)]
pub struct CheckEntry {
    /// The obligation id (`check:PATH`).
    pub id: ObligationId,
    /// The check's key in the digest graph (its canonical path).
    pub key: String,
    /// Closure digest of the covered set.
    pub covered: Digest,
    /// Own digests of every transitive dep.
    pub deps: Vec<Digest>,
    /// The fresh verdict.
    pub verdict: Verdict,
}

/// One `ots:PATH` source sign-off with the cite items it reaches.
#[derive(Debug, Clone)]
pub struct OtsEntry {
    /// The obligation id (`ots:PATH`).
    pub id: ObligationId,
    /// Closure digest of the component and its members.
    pub covered: Digest,
    /// Own digests of every transitive dep.
    pub deps: Vec<Digest>,
    /// The distinct `cite` items its component and members reference.
    pub citations: Vec<Citation>,
}

/// The digest graph and check obligations of one package.
#[derive(Debug)]
pub struct Universe {
    graph: DigestGraph,
    /// Direct dependencies of every graph key (for the `why --stale` walk).
    edges: std::collections::BTreeMap<String, Vec<String>>,
    /// One entry per IR check (`check` or `require`) in IR order, then one per fit.
    pub checks: Vec<CheckEntry>,
    /// One entry per `ots` component (ack only).
    pub ots: Vec<OtsEntry>,
}

fn verdict_of(c: &Check) -> Verdict {
    match c.verdicts.primary {
        Some(VerdictState::Pass) => Verdict::Holds,
        Some(VerdictState::Fail) => Verdict::Violated,
        Some(VerdictState::Unknown) | None => Verdict::Undecided,
    }
}

impl Universe {
    /// Build the graph and the check obligations of `ir` with no `require` known; errors on a
    /// dependency cycle.
    pub fn of(ir: &IrProject) -> Result<Self, DigestError> {
        Self::of_with(ir, &BTreeSet::new())
    }

    /// Build the graph and the check, fit and `require` obligations of `ir`; `requires` holds
    /// the canonical paths of the IR checks that are `require`s (the IR does not mark them).
    // frob:ticket 01M4V1HYN215BC5W2R6Y0NVZAF
    pub fn of_with(ir: &IrProject, requires: &BTreeSet<String>) -> Result<Self, DigestError> {
        let mut nodes: Vec<(String, Digest, Vec<String>)> = Vec::new();
        let members: Vec<_> = ir.components.iter().flat_map(|c| &c.members).collect();
        for m in &members {
            let deps = m.deps.iter().map(ToString::to_string).collect();
            nodes.push((
                m.path.to_string(),
                cadetta_ir::digest::member_digest(m),
                deps,
            ));
        }
        // An `ots` component is a node over its members, so a citation locator edit (a member or
        // component edit) changes its closure digest (obligations.md 2.3, O001).
        // frob:ticket 01M4VB63M63PQVRKN0HDXQP567
        for c in ir
            .components
            .iter()
            .filter(|c| c.class == ComponentClass::Ots)
        {
            let deps = c.members.iter().map(|m| m.path.to_string()).collect();
            nodes.push((
                c.path.to_string(),
                cadetta_ir::digest::component_digest(c),
                deps,
            ));
        }
        for s in &ir.solvers {
            let deps = s.deps.iter().map(ToString::to_string).collect();
            nodes.push((
                s.path.to_string(),
                cadetta_ir::digest::solver_digest(s),
                deps,
            ));
        }
        // A fit is a node over its hole and shaft members (obligations.md 1, `evaluates`).
        for f in ir.components.iter().flat_map(|c| &c.fits) {
            let deps = vec![f.hole.to_string(), f.shaft.to_string()];
            nodes.push((f.path.to_string(), cadetta_ir::digest::fit_digest(f), deps));
        }
        let mut g = DigestGraph::new();
        for c in &ir.components {
            for p in &c.parts {
                g.add_part(&p.path.to_string(), p);
            }
        }
        for c in &ir.checks {
            let reads = members
                .iter()
                .map(|m| m.path.to_string())
                .filter(|m| check_reads(&c.path.to_string(), &c.expr, m))
                .collect();
            nodes.push((c.path.to_string(), check_digest(c), reads));
        }
        // Keep only edges to known nodes: a read of a table or a dependency has no node.
        let mut known: BTreeSet<String> = nodes.iter().map(|(k, ..)| k.clone()).collect();
        known.extend(ir.components.iter().flat_map(|c| {
            c.parts.iter().flat_map(|p| {
                let slot = p.path.to_string();
                let args: Vec<String> = p.args.keys().map(|k| format!("{slot}.{k}")).collect();
                std::iter::once(slot).chain(args)
            })
        }));
        let mut edges = std::collections::BTreeMap::new();
        for (key, own, deps) in nodes {
            let deps: Vec<String> = deps.into_iter().filter(|d| known.contains(d)).collect();
            edges.insert(key.clone(), deps.clone());
            g.add_node(key, own, deps);
        }
        let mut checks = Vec::new();
        for c in &ir.checks {
            let key = c.path.to_string();
            let covered = g.closure(&key)?;
            let deps = g.dep_owns(&key)?;
            let kind = if requires.contains(&key) {
                ObligationKind::Require
            } else {
                ObligationKind::Check
            };
            tracing::debug!(%key, ?kind, deps = deps.len(), "universe: check obligation");
            checks.push(CheckEntry {
                id: ObligationId::new(kind, key.clone()),
                key,
                covered,
                deps,
                verdict: verdict_of(c),
            });
        }
        // A fit's result is signed off like a check; F004 and C002 report a bad clearance on
        // their own, so the sign-off judges the fit as evaluated (obligations.md 2.3).
        for f in ir.components.iter().flat_map(|c| &c.fits) {
            let key = f.path.to_string();
            tracing::debug!(%key, "universe: fit obligation");
            checks.push(CheckEntry {
                id: ObligationId::new(ObligationKind::Fit, key.clone()),
                covered: g.closure(&key)?,
                deps: g.dep_owns(&key)?,
                key,
                verdict: Verdict::Holds,
            });
        }
        let mut ots = Vec::new();
        for c in ir
            .components
            .iter()
            .filter(|c| c.class == ComponentClass::Ots)
        {
            let key = c.path.to_string();
            let mut cites: Vec<String> = c
                .citation
                .iter()
                .chain(c.members.iter().filter_map(|m| m.citation.as_ref()))
                .filter_map(|r| r.cite.clone())
                .collect();
            cites.sort();
            cites.dedup();
            let citations = cites
                .iter()
                .filter_map(|p| ir.citations.iter().find(|c| &c.path == p).cloned())
                .collect();
            tracing::debug!(%key, cites = cites.len(), "universe: ots obligation");
            ots.push(OtsEntry {
                id: ObligationId::new(ObligationKind::Ots, key.clone()),
                covered: g.closure(&key)?,
                deps: g.dep_owns(&key)?,
                citations,
            });
        }
        Ok(Self {
            graph: g,
            edges,
            checks,
            ots,
        })
    }

    /// The closure digest of every graph key (waiver target binding, W002).
    // frob:ticket 01M4TRKDEST57137A1Y203JFR7
    pub fn closures(&self) -> Result<std::collections::BTreeMap<String, Digest>, DigestError> {
        self.graph.closures()
    }

    /// The entry for the obligation `id`, when this package offers it.
    pub fn entry(&self, id: &ObligationId) -> Option<&CheckEntry> {
        self.checks.iter().find(|e| &e.id == id)
    }

    /// The graph keys `key` covers: itself and every transitive dependency (frob:ticket 01M4ST8FAX8HDAXH8NBC8VKPXW).
    pub fn covered_keys(&self, key: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut stack = vec![key.to_owned()];
        while let Some(k) = stack.pop() {
            if !seen.insert(k.clone()) {
                continue;
            }
            stack.extend(self.edges.get(&k).into_iter().flatten().cloned());
        }
        seen
    }

    /// Explain why `att` on `entry` is stale: the nearest changed upstream node.
    pub fn explain(
        &self,
        entry: &CheckEntry,
        att: Option<&Attest>,
    ) -> Result<WhyStale, DigestError> {
        let deps_of = |k: &str| self.edges.get(k).cloned().unwrap_or_default();
        why_stale(
            att,
            std::slice::from_ref(&entry.key),
            &entry.covered.0,
            &self.graph,
            &deps_of,
        )
    }
}

fn judgement(e: &CheckEntry) -> Judgement {
    Judgement {
        verdict: e.verdict,
        margin: NO_MARGIN.to_owned(),
    }
}

/// Every package's universe, logging and skipping a package whose graph is cyclic.
pub fn universes(analysis: &Analysis) -> Vec<Universe> {
    analysis
        .packages
        .iter()
        .filter_map(
            |p| match Universe::of_with(&p.ir, &analysis.suppressions.require_paths) {
                Ok(u) => Some(u),
                Err(error) => {
                    tracing::warn!(package = %p.name, %error, "universe: digest graph unavailable");
                    None
                }
            },
        )
        .collect()
}

/// Whether `e` is offered for sign-off: a `require` only when it holds (obligations.md 4, the
/// `require` rule: sign-off never changes a violated or undecided verdict).
fn signable(e: &&CheckEntry) -> bool {
    e.id.kind() != ObligationKind::Require || e.verdict == Verdict::Holds
}

/// The `obligations` candidates: every IR check, fit, passing require, every `ots` component and every cite item an
/// `ots` reaches, judged against closure digests (obligations.md 2.2).
// frob:ticket 01M4TA3C5QWZ2NH70V6QN4PR7X
pub fn candidates(universes: &[Universe]) -> Vec<Candidate> {
    let checks = universes
        .iter()
        .flat_map(|u| &u.checks)
        .filter(signable)
        .map(|e| Candidate {
            id: e.id.clone(),
            current: e.covered.clone(),
            deps: e.deps.clone(),
            judgement: Some(judgement(e)),
            suppressed: false,
            nearest_changed: None,
        });
    let ots = universes.iter().flat_map(|u| &u.ots).map(|e| Candidate {
        id: e.id.clone(),
        current: e.covered.clone(),
        deps: e.deps.clone(),
        judgement: Some(Judgement {
            verdict: Verdict::Holds,
            margin: NO_MARGIN.to_owned(),
        }),
        suppressed: false,
        nearest_changed: None,
    });
    let mut seen = BTreeSet::new();
    let citations = universes
        .iter()
        .flat_map(|u| &u.ots)
        .flat_map(|e| &e.citations)
        .filter(|c| seen.insert(c.path.clone()))
        .map(|c| Candidate {
            id: citation_id(&c.path),
            current: c.digest.clone(),
            deps: Vec::new(),
            judgement: Some(Judgement {
                verdict: Verdict::Holds,
                margin: NO_MARGIN.to_owned(),
            }),
            suppressed: false,
            nearest_changed: None,
        });
    let out: Vec<Candidate> = checks.chain(ots).chain(citations).collect();
    tracing::debug!(count = out.len(), "universe: obligation candidates");
    out
}

/// The `ack` candidates: one sign-off per IR check.
pub fn ack_candidates(universes: &[Universe]) -> Vec<AckCandidate> {
    universes
        .iter()
        .flat_map(|u| &u.checks)
        .filter(signable)
        .map(|e| AckCandidate {
            id: e.id.clone(),
            subject: Subject::Attest {
                covered: e.covered.clone(),
                deps: e.deps.clone(),
                judgement: judgement(e),
                citations: Vec::new(),
            },
        })
        .chain(universes.iter().flat_map(|u| &u.ots).map(|e| AckCandidate {
            id: e.id.clone(),
            subject: Subject::Attest {
                covered: e.covered.clone(),
                deps: e.deps.clone(),
                judgement: Judgement {
                    verdict: Verdict::Holds,
                    margin: NO_MARGIN.to_owned(),
                },
                citations: e.citations.clone(),
            },
        }))
        .collect()
}

/// The text `sync` would push for every bound locator of `adapter` now, keyed by
/// `(parameter path, locator)`, normalized the way the `synced` lock values are.
// frob:ticket 01M4V1HYS6PKK0F82XMJET4TC0
fn emitted_values(
    pkg: &PackageDir,
    adapter: &Adapter,
    ir: &IrProject,
) -> std::collections::BTreeMap<(String, String), String> {
    use cadetta_adapter::builtin::sw_equations::{DEFAULT_PRECISION, ValueKind};
    use cadetta_adapter::policy::resolve_point;
    use cadetta_adapter::scope::Scope as AdapterScope;
    use cadetta_ir::model::{LocatorMode, Sign};
    use cadetta_units::UnitTable;

    let mut out = std::collections::BTreeMap::new();
    let Ok((set, _)) = load_bindings(pkg, adapter, ir) else {
        tracing::debug!(adapter = %adapter.id(), "universe: bindings unavailable, no sync values");
        return out;
    };
    let scope = AdapterScope::new(adapter.modules.clone(), adapter.paths.clone());
    let table = UnitTable::builtin();
    let precision = adapter.precision.map_or(DEFAULT_PRECISION, u32::from);
    let cfg_point = config_point(pkg.config.emit.point);
    for member in ir.components.iter().flat_map(|c| &c.members) {
        let canonical = member.path.to_string();
        let Some(value) = &member.value else { continue };
        if member.analysis || !scope.accepts(&canonical) {
            continue;
        }
        let Some(locators) = locators_for(&set, &member.path) else {
            continue;
        };
        let kind = kind_of(value);
        let point = resolve_point(&[attr_point(member)], cfg_point);
        for loc in locators
            .iter()
            .filter(|l| l.for_alternate.is_none() && l.mode != LocatorMode::Ref)
        {
            let unit = native_unit(adapter, loc);
            let Ok(Some(written)) =
                written_for(&table, value, member, &unit, kind, point, precision)
            else {
                continue;
            };
            let written = if loc.sign == Some(Sign::Minus) {
                negate_text(&written)
            } else {
                written
            };
            let default_unit = if kind == ValueKind::Quantity {
                unit
            } else {
                String::new()
            };
            out.insert(
                (canonical.clone(), loc.locator.clone()),
                normalize(&written, &default_unit),
            );
        }
    }
    out
}

/// The stale `sync` obligations (obligations.md 2.3, O004): for every native file an adapter
/// last saved (`synced file=` record), the obligation `sync:FILE` is stale when a parameter the
/// adapter binds now emits a value that differs from its `synced` lock value. The lock keeps
/// values per adapter, so every saved file of a drifted adapter is reported.
// frob:ticket 01M4V1HYS6PKK0F82XMJET4TC0
pub fn sync_states(
    project: &Project,
    analysis: &Analysis,
    lock: &Lock,
) -> Vec<(ObligationId, ObligationState)> {
    let mut out = Vec::new();
    for package in &analysis.packages {
        let Some(pkg) = project.packages.iter().find(|p| {
            p.config.package.as_ref().and_then(|c| c.name.as_deref()) == Some(package.name.as_str())
        }) else {
            continue;
        };
        for adapter in &pkg.config.adapters {
            let synced: Vec<_> = lock
                .records()
                .filter_map(|r| match r {
                    Record::SyncedValue(v) if v.adapter == adapter.id() => Some(v),
                    _ => None,
                })
                .collect();
            if synced.is_empty() {
                continue;
            }
            let now = emitted_values(pkg, adapter, &package.ir);
            let drifted = synced.iter().any(|v| {
                now.get(&(v.path.clone(), v.locator.clone()))
                    .is_some_and(|text| *text != v.value)
            });
            tracing::debug!(adapter = %adapter.id(), drifted, "universe: sync drift judged");
            if !drifted {
                continue;
            }
            for r in lock.records() {
                if let Record::SyncedFile(f) = r
                    && f.adapter == adapter.id()
                {
                    out.push((
                        ObligationId::new(ObligationKind::Sync, f.file.clone()),
                        ObligationState::Stale,
                    ));
                }
            }
        }
    }
    out
}

// frob:ticket 01M4TA3CEMSKPPZTY1CM2AFDDQ
#[cfg(test)]
mod tests {
    use crate::cmd::{ack, impact, ir, obligations, why};
    use crate::output::Format;
    use crate::{EXIT_OK, Globals};
    use cadetta_ir::model::{Check, Project as IrProject, VerdictState};
    use serde_json::Value;
    use std::collections::BTreeSet;

    const AXLE: &str = "model Axle {\n  od: Length = 20mm\n  let bore: Length = 8mm\n  check bore_ok: bore < od \"bore fits\"\n}\n";
    const ROLES: &str = "[obligations.roles.review]\ncovers = [\"check:acme.axle::**\"]\n";
    const CHECK: &str = "check:acme.axle::Axle.bore_ok";

    struct Dir(std::path::PathBuf);

    impl Dir {
        fn new() -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("cadetta-uni-{}-{n}", std::process::id()));
            std::fs::create_dir_all(p.join("src")).unwrap();
            std::fs::write(
                p.join("cadetta.toml"),
                format!("[package]\nname = \"acme.axle\"\nversion = \"0.1.0\"\n{ROLES}"),
            )
            .unwrap();
            std::fs::write(p.join("src/lib.cdt"), AXLE).unwrap();
            Self(p)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    type Verb = fn(&[String], &Globals, &mut dyn std::io::Write, &mut dyn std::io::Write) -> u8;

    fn run(verb: Verb, dir: &Dir, args: &[&str]) -> Value {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let globals = Globals {
            format: Format::Json,
            cwd: Some(dir.0.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = verb(&args, &globals, &mut out, &mut err);
        let err = String::from_utf8(err).unwrap();
        assert_eq!(code, EXIT_OK, "{err}");
        serde_json::from_slice(&out).unwrap_or(Value::Null)
    }

    fn ack_it(dir: &Dir) {
        run(
            ack::run_ack,
            dir,
            &[CHECK, "--role", "review", "--reason", "ok", "--as", "ana@x"],
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/ir.rs::run_ir
    #[test]
    fn ir_prints_lowered_digested_members() {
        let v = run(ir::run_ir, &Dir::new(), &[]);
        let members = v["components"][0]["members"].as_array().unwrap();
        assert!(members.len() >= 2, "{v}");
        assert!(
            members
                .iter()
                .all(|m| m["digest"].is_object() || m["digest"].is_string())
        );
        assert_eq!(v["checks"].as_array().unwrap().len(), 1);
    }

    // frob:tests crates/cadetta-cli/src/cmd/why.rs::run_why
    #[test]
    fn why_traces_a_lowered_member() {
        let v = run(why::run_why, &Dir::new(), &["acme.axle::Axle.bore"]);
        assert_eq!(v["path"], "acme.axle::Axle.bore");
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact.rs::run_impact
    #[test]
    fn impact_lists_dependents_of_a_lowered_member() {
        let v = run(impact::run_impact, &Dir::new(), &["acme.axle::Axle.od"]);
        assert!(!v["dependents"].as_array().unwrap().is_empty(), "{v}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/obligations.rs::run_obligations
    #[test]
    fn obligations_lists_the_open_check() {
        let v = run(obligations::run_obligations, &Dir::new(), &[]);
        let rows = v["obligations"].as_array().unwrap();
        assert!(
            rows.iter()
                .any(|r| r["id"] == CHECK && r["state"] == "open"),
            "{v}"
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::run_ack
    #[test]
    fn ack_records_an_attestation_for_the_check() {
        let dir = Dir::new();
        ack_it(&dir);
        let lock = std::fs::read_to_string(dir.0.join("cadetta.lock")).unwrap();
        assert!(lock.contains(CHECK), "{lock}");
        let v = run(obligations::run_obligations, &dir, &[]);
        assert!(
            !v["obligations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == CHECK),
            "{v}"
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/why.rs::run_why
    #[test]
    fn why_stale_names_the_changed_node_after_an_edit() {
        let dir = Dir::new();
        ack_it(&dir);
        let v = run(why::run_why, &dir, &["--stale", CHECK]);
        assert_eq!(v["stale"]["state"], "current", "{v}");
        std::fs::write(dir.0.join("src/lib.cdt"), AXLE.replace("= 8mm", "= 9mm")).unwrap();
        let v = run(why::run_why, &dir, &["--stale", CHECK]);
        assert_eq!(v["stale"]["state"], "stale", "{v}");
        assert_eq!(v["stale"]["nearest"]["node"], "acme.axle::Axle.bore", "{v}");
    }

    const OTS_SRC: &str = "cite spec: document {\n  title = \"Spec\"\n  date = \"2026-01-01\"\n  file = \"docs/spec.txt\"\n}\nots Tube @spec {\n  od = 20mm @spec:\"p.1\"\n}\n";

    // frob:ticket 01M4TA3C5QWZ2NH70V6QN4PR7X
    // frob:tests crates/cadetta-cli/src/universe.rs::candidates
    #[test]
    fn obligations_enumerates_ots_and_citation_kinds() {
        let dir = Dir::new();
        std::fs::create_dir_all(dir.0.join("docs")).unwrap();
        std::fs::write(dir.0.join("docs/spec.txt"), "one").unwrap();
        std::fs::write(dir.0.join("src/lib.cdt"), OTS_SRC).unwrap();
        let ids = |v: &Value| -> Vec<String> {
            v["obligations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["id"].as_str().unwrap().to_owned())
                .collect()
        };
        let v = run(obligations::run_obligations, &dir, &[]);
        let open = ids(&v);
        assert!(open.contains(&"ots:acme.axle::Tube".to_owned()), "{v}");
        assert!(open.contains(&"citation:acme.axle::spec".to_owned()), "{v}");
        let toml = std::fs::read_to_string(dir.0.join("cadetta.toml")).unwrap();
        std::fs::write(
            dir.0.join("cadetta.toml"),
            format!("{toml}[obligations.roles.source]\ncovers = [\"ots:acme.axle::**\", \"citation:acme.axle::**\"]\n"),
        )
        .unwrap();
        run(
            ack::run_ack,
            &dir,
            &[
                "ots:acme.axle::Tube",
                "--role",
                "source",
                "--reason",
                "ok",
                "--as",
                "ana@x",
            ],
        );
        let v = run(obligations::run_obligations, &dir, &[]);
        assert!(ids(&v).is_empty(), "{v}");
        std::fs::write(dir.0.join("docs/spec.txt"), "two").unwrap();
        let v = run(obligations::run_obligations, &dir, &[]);
        let rows = v["obligations"].as_array().unwrap();
        let cit = rows
            .iter()
            .find(|r| r["id"] == "citation:acme.axle::spec")
            .unwrap_or_else(|| panic!("{v}"));
        assert_eq!(cit["state"], "stale", "{v}");
        assert!(!ids(&v).contains(&"ots:acme.axle::Tube".to_owned()), "{v}");
    }

    // frob:ticket 01M4VB63M63PQVRKN0HDXQP567
    // frob:tests crates/cadetta-cli/src/cmd/ack.rs::run_ack
    // frob:tests crates/cadetta-cli/src/cmd/check.rs::citation_diagnostics
    #[test]
    fn ack_ots_records_citation_and_a_file_change_is_s004_alone() {
        let dir = Dir::new();
        std::fs::create_dir_all(dir.0.join("docs")).unwrap();
        std::fs::write(dir.0.join("docs/spec.txt"), "one").unwrap();
        std::fs::write(dir.0.join("src/lib.cdt"), OTS_SRC).unwrap();
        let toml = std::fs::read_to_string(dir.0.join("cadetta.toml")).unwrap();
        std::fs::write(
            dir.0.join("cadetta.toml"),
            format!("{toml}[obligations.roles.source]\ncovers = [\"ots:acme.axle::**\"]\n"),
        )
        .unwrap();
        run(
            ack::run_ack,
            &dir,
            &[
                "ots:acme.axle::Tube",
                "--role",
                "source",
                "--reason",
                "ok",
                "--as",
                "ana@x",
            ],
        );
        let lock = std::fs::read_to_string(dir.0.join("cadetta.lock")).unwrap();
        assert!(
            lock.contains("attest citation:acme.axle::spec role=source"),
            "{lock}"
        );
        let v = run(crate::cmd::check::run_check, &dir, &[]);
        assert!(v.to_string().find("S004").is_none(), "{v}");
        std::fs::write(dir.0.join("docs/spec.txt"), "two").unwrap();
        let v = run(crate::cmd::check::run_check, &dir, &[]).to_string();
        assert!(v.contains("S004"), "{v}");
        assert!(!v.contains("O001"), "{v}");
    }

    const SYNC_SRC: &str = "model Plate {\n  thickness: Length = 6mm\n}\n";
    const SYNC_BINDINGS: &str = "[\"acme.axle::Plate.thickness\"]\nlocators = [ { locator = \"Plate_Thk\", kind = \"global\", native_unit = \"mm\" } ]\n";

    /// A project with a bound `sw-equations` adapter whose lock last synced `thickness` as `last`.
    fn synced_project(last: &str, kinds: &str) -> Dir {
        use cadetta_obligations::lock::{Hex, Lock, Record, SyncedFile, SyncedValue};
        let dir = Dir::new();
        let toml = std::fs::read_to_string(dir.0.join("cadetta.toml")).unwrap();
        let adapter =
            "[[adapter]]\nname = \"sw-equations\"\nid = \"equations\"\nfiles = [\"part.FCStd\"]\n";
        std::fs::write(
            dir.0.join("cadetta.toml"),
            format!("{toml}{adapter}{kinds}"),
        )
        .unwrap();
        std::fs::write(dir.0.join("src/lib.cdt"), SYNC_SRC).unwrap();
        std::fs::create_dir_all(dir.0.join("bindings")).unwrap();
        std::fs::write(dir.0.join("bindings/equations.toml"), SYNC_BINDINGS).unwrap();
        let mut lock = Lock::new();
        lock.insert(Record::SyncedValue(SyncedValue {
            adapter: "equations".into(),
            path: "acme.axle::Plate.thickness".into(),
            locator: "Plate_Thk".into(),
            value: last.into(),
        }));
        lock.insert(Record::SyncedFile(SyncedFile {
            adapter: "equations".into(),
            file: "part.FCStd".into(),
            digest: Hex::new("0a").unwrap(),
        }));
        lock.save(&dir.0.join("cadetta.lock")).unwrap();
        dir
    }

    fn check_json(dir: &Dir, args: &[&str]) -> (u8, Value) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let globals = Globals {
            format: Format::Json,
            cwd: Some(dir.0.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = crate::cmd::check::run_check(&args, &globals, &mut out, &mut err);
        (code, serde_json::from_slice(&out).unwrap_or(Value::Null))
    }

    fn o004(v: &Value) -> Option<&Value> {
        v["diagnostics"]
            .as_array()?
            .iter()
            .find(|d| d["code"] == "O004")
    }

    // frob:ticket 01M4V1HYS6PKK0F82XMJET4TC0
    // frob:tests crates/cadetta-cli/src/universe.rs::sync_states
    #[test]
    fn changed_emitted_value_is_o004_info_on_the_native_file() {
        let (code, v) = check_json(&synced_project("5mm", ""), &[]);
        let d = o004(&v).unwrap_or_else(|| panic!("{v}"));
        assert_eq!(d["path"], "part.FCStd", "{v}");
        assert_eq!(d["severity"], "info", "{v}");
        assert_eq!(code, 0, "{v}");
    }

    // frob:ticket 01M4V1HYS6PKK0F82XMJET4TC0
    // frob:tests crates/cadetta-cli/src/universe.rs::sync_states
    #[test]
    fn unchanged_emitted_value_raises_no_o004() {
        let (_, v) = check_json(&synced_project("6mm", ""), &[]);
        assert!(o004(&v).is_none(), "{v}");
    }

    // frob:ticket 01M4V1HYS6PKK0F82XMJET4TC0
    // frob:tests crates/cadetta-cli/src/universe.rs::sync_states
    #[test]
    fn sync_kind_required_for_check_makes_o004_gate() {
        let kinds = "[obligations.kinds.sync]\nrequired_for = \"check\"\n";
        let (code, v) = check_json(&synced_project("5mm", kinds), &[]);
        assert_eq!(
            o004(&v).unwrap_or_else(|| panic!("{v}"))["severity"],
            "error",
            "{v}"
        );
        assert_eq!(code, 1, "{v}");
    }

    const FIT_SRC: &str = "model Axle {\n  bore = 20mm\n  fit seat = H7/p6 on bore\n}\n";

    // frob:ticket 01M4V1HYN215BC5W2R6Y0NVZAF
    // frob:tests crates/cadetta-cli/src/universe.rs::candidates
    // frob:tests crates/cadetta-cli/src/universe.rs::ack_candidates
    #[test]
    fn fit_obligation_is_listed_signed_off_and_stales_on_edit() {
        let dir = Dir::new();
        std::fs::write(dir.0.join("src/lib.cdt"), FIT_SRC).unwrap();
        let toml = std::fs::read_to_string(dir.0.join("cadetta.toml")).unwrap();
        let roles = "[obligations.roles.signoff]\ncovers = [\"fit:acme.axle::**\"]\n";
        std::fs::write(dir.0.join("cadetta.toml"), format!("{toml}{roles}")).unwrap();
        let fit = "fit:acme.axle::Axle.seat";
        let v = run(obligations::run_obligations, &dir, &[]);
        let row = &v["obligations"][0];
        assert_eq!(
            (&row["id"], &row["state"]),
            (&Value::from(fit), &Value::from("open")),
            "{v}"
        );
        let text = run(crate::cmd::check::run_check, &dir, &[]).to_string();
        assert!(text.contains("O002") && text.contains(fit), "{text}");
        let sign = &[fit, "--role", "signoff", "--reason", "ok", "--as", "ana@x"];
        run(ack::run_ack, &dir, sign);
        let v = run(obligations::run_obligations, &dir, &[]);
        assert!(v["obligations"].as_array().unwrap().is_empty(), "{v}");
        std::fs::write(dir.0.join("src/lib.cdt"), FIT_SRC.replace("H7/p6", "H7/g6")).unwrap();
        let v = run(obligations::run_obligations, &dir, &[]);
        assert_eq!(v["obligations"][0]["state"], "stale", "{v}");
    }

    fn ir_with_requires() -> IrProject {
        use cadetta_ir::model::{Flavour, PackageId, Verdicts};
        use cadetta_ir::path::{Path, PathContext};
        let mut ir = IrProject::new(
            Flavour::Design,
            PackageId {
                name: "a".into(),
                version: "0.1.0".into(),
            },
        );
        for (name, state) in [("ok", VerdictState::Pass), ("bad", VerdictState::Fail)] {
            ir.checks.push(Check {
                path: Path::parse(&format!("a::M.{name}"), PathContext::General).unwrap(),
                expr: "x".into(),
                verdicts: Verdicts {
                    primary: Some(state),
                    ..Verdicts::default()
                },
                conservative: false,
                waived_for: Vec::new(),
            });
        }
        ir
    }

    // frob:ticket 01M4V1HYN215BC5W2R6Y0NVZAF
    // frob:tests crates/cadetta-cli/src/universe.rs::Universe.of_with
    // frob:tests crates/cadetta-cli/src/universe.rs::candidates
    #[test]
    fn require_paths_make_require_obligations_and_only_a_holding_one_is_signable() {
        let ir = ir_with_requires();
        let reqs: BTreeSet<String> = ["a::M.ok", "a::M.bad"].map(String::from).into();
        let u = super::Universe::of_with(&ir, &reqs).unwrap();
        let ids: Vec<String> = super::candidates(&[u])
            .iter()
            .map(|c| c.id.to_string())
            .collect();
        assert_eq!(ids, vec!["require:a::M.ok"]);
        let plain = super::Universe::of(&ir).unwrap();
        assert_eq!(plain.checks[0].id.to_string(), "check:a::M.ok");
    }
}
