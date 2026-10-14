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
use cadetta_obligations::graph::{ObligationId, ObligationKind, Verdict};
use cadetta_obligations::lock::Attest;
use cadetta_obligations::staleness::Judgement;
use cadetta_obligations::why_stale::{WhyStale, why_stale};

use crate::cmd::ack::{Candidate as AckCandidate, Subject};
use crate::cmd::obligations::Candidate;
use crate::pipeline::Analysis;

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
    /// One entry per IR check, in IR order.
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
    /// Build the graph and the check obligations of `ir`; errors on a dependency cycle.
    pub fn of(ir: &IrProject) -> Result<Self, DigestError> {
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
            tracing::debug!(%key, deps = deps.len(), "universe: check obligation");
            checks.push(CheckEntry {
                id: ObligationId::new(ObligationKind::Check, key.clone()),
                key,
                covered,
                deps,
                verdict: verdict_of(c),
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
        .filter_map(|p| match Universe::of(&p.ir) {
            Ok(u) => Some(u),
            Err(error) => {
                tracing::warn!(package = %p.name, %error, "universe: digest graph unavailable");
                None
            }
        })
        .collect()
}

/// The `obligations` candidates: every IR check, every `ots` component and every cite item an
/// `ots` reaches, judged against closure digests (obligations.md 2.2).
// frob:ticket 01M4TA3C5QWZ2NH70V6QN4PR7X
pub fn candidates(universes: &[Universe]) -> Vec<Candidate> {
    let checks = universes.iter().flat_map(|u| &u.checks).map(|e| Candidate {
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

// frob:ticket 01M4TA3CEMSKPPZTY1CM2AFDDQ
#[cfg(test)]
mod tests {
    use crate::cmd::{ack, impact, ir, obligations, why};
    use crate::output::Format;
    use crate::{EXIT_OK, Globals};
    use serde_json::Value;

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
}
