//! frob:ticket 01M4V0NSCWCW8W0P7DG628M8AP
//! Corpus discovery, the known-gaps allowlist and the per-fixture verdicts.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::exec::execute;
use crate::expect::{Block, Expect};
use crate::verify::verify;

/// One allowlist entry.
#[derive(Debug, Clone, Deserialize)]
pub struct Gap {
    /// `<doc>/<NAME>`.
    pub fixture: String,
    /// Ticket handle that closes the gap.
    pub ticket: String,
    /// Why the implementation lags the spec.
    pub reason: String,
}

#[derive(Deserialize, Default)]
struct GapFile {
    #[serde(default)]
    gap: Vec<Gap>,
}

/// Fixtures allowed to fail, keyed by `<doc>/<NAME>`.
#[derive(Debug, Default)]
pub struct KnownGaps(BTreeMap<String, Gap>);

impl KnownGaps {
    /// Add an entry (used by tests).
    pub fn insert(&mut self, fixture: &str, ticket: &str, reason: &str) {
        self.0.insert(
            fixture.to_owned(),
            Gap {
                fixture: fixture.to_owned(),
                ticket: ticket.to_owned(),
                reason: reason.to_owned(),
            },
        );
    }
}

/// Load `known-gaps.toml`; a missing file is an empty list.
pub fn load_known_gaps(path: &Path) -> Result<KnownGaps, String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(KnownGaps::default()),
        Err(e) => return Err(e.to_string()),
    };
    let file: GapFile = toml::from_str(&text).map_err(|e| e.to_string())?;
    let mut gaps = BTreeMap::new();
    for gap in file.gap {
        if gap.ticket.trim().is_empty() || gap.reason.trim().is_empty() {
            return Err(format!("{}: ticket and reason are required", gap.fixture));
        }
        gaps.insert(gap.fixture.clone(), gap);
    }
    Ok(KnownGaps(gaps))
}

/// The verdict for one fixture pair.
#[derive(Debug)]
pub enum Outcome {
    /// Both fixtures matched.
    Pass,
    /// Mismatches (also listed in the report failures unless allowlisted).
    Fail(Vec<String>),
    /// Failed, but allowlisted.
    KnownGap,
}

/// Everything a corpus run learned.
#[derive(Debug, Default)]
pub struct Report {
    /// Verdict per `<doc>/<NAME>`.
    pub outcomes: BTreeMap<String, Outcome>,
    /// Failure lines, each naming fixture, doc and the mismatch.
    pub failures: Vec<String>,
    /// Informational lines (tolerated gaps).
    pub notes: Vec<String>,
}

fn json_error<T>(r: Result<T, impl std::fmt::Display>, what: &str) -> Result<T, String> {
    r.map_err(|e| format!("{what}: {e}"))
}

fn fixture_path(dir: &Path, name: &str, which: &str, kind: &str) -> Option<std::path::PathBuf> {
    let ext = if kind == "source" { "cdt" } else { "json" };
    [format!("{name}.{which}.{ext}"), format!("{name}.{which}")]
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.exists())
}

fn check_pair(dir: &Path, name: &str, doc: &str) -> Result<Outcome, String> {
    let text = json_error(
        fs::read_to_string(dir.join(format!("{name}.expect.json"))),
        "expect.json",
    )?;
    let expect: Expect = json_error(serde_json::from_str(&text), "expect.json")?;
    if expect.doc != doc {
        return Err(format!(
            "expect.json doc `{}` != directory `{doc}`",
            expect.doc
        ));
    }
    let mut problems = Vec::new();
    for (which, block) in [
        ("must-fire", &expect.must_fire),
        ("must-not-fire", &expect.must_not_fire),
    ] {
        problems.extend(run_block(dir, name, which, block, &expect)?);
    }
    Ok(if problems.is_empty() {
        Outcome::Pass
    } else {
        Outcome::Fail(problems)
    })
}

/// The unused-private-item lint, ignored by fixtures that do not list it.
const UNUSED_PRIVATE: &str = "U002";

/// True when either block of `expect` lists a diagnostic with `code`.
fn lists_code(expect: &Expect, code: &str) -> bool {
    [&expect.must_fire, &expect.must_not_fire]
        .iter()
        .any(|b| b.diagnostics.iter().any(|d| d.code == code))
}

fn run_block(
    dir: &Path,
    name: &str,
    which: &str,
    block: &Block,
    expect: &Expect,
) -> Result<Vec<String>, String> {
    let path = fixture_path(dir, name, which, &expect.kind)
        .ok_or_else(|| format!("{which} fixture file is missing"))?;
    let prefix = |m: String| format!("{which}: {m}");
    match execute(expect, &path, block) {
        Ok(mut facts) => {
            if !lists_code(expect, UNUSED_PRIVATE) {
                // Fixture components are private and instantiated nowhere by design, so U002
                // is style noise unless the fixture is about U002 itself.
                facts.produced.retain(|p| p.code != UNUSED_PRIVATE);
            }
            Ok(verify(block, &facts).into_iter().map(prefix).collect())
        }
        Err(message) => Ok(vec![prefix(message)]),
    }
}

/// Fixture names (`NAME`) with an `expect.json` in `dir`, sorted.
fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            e.file_name()
                .to_str()?
                .strip_suffix(".expect.json")
                .map(str::to_owned)
        })
        .collect();
    names.sort();
    names
}

/// Collect doc names (`language`, `emitters/sw-equations`) under `dir`, recursing
/// into nested directories but never into `NAME.must-fire/` package fixtures.
fn collect_docs(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !path.is_dir() || name.ends_with(".must-fire") || name.ends_with(".must-not-fire") {
            continue;
        }
        let doc = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        collect_docs(&path, &doc, out);
        out.push(doc);
    }
}

/// Discover and run every fixture pair under `root/<doc>/` (nested docs included); tolerates no corpus.
pub fn run_corpus(root: &Path, allow: &KnownGaps) -> Report {
    let mut report = Report::default();
    let mut docs = Vec::new();
    collect_docs(root, "", &mut docs);
    docs.sort();
    for doc in docs {
        for name in names_in(&root.join(&doc)) {
            let key = format!("{doc}/{name}");
            let outcome = match check_pair(&root.join(&doc), &name, &doc) {
                Ok(o) => o,
                Err(m) => Outcome::Fail(vec![m]),
            };
            let gap = allow.0.get(&key);
            let outcome = match (outcome, gap) {
                (Outcome::Fail(p), Some(g)) => {
                    report.notes.push(format!(
                        "{key} [{doc}]: known gap {} ({}): {}",
                        g.ticket,
                        g.reason,
                        p.join("; ")
                    ));
                    Outcome::KnownGap
                }
                (Outcome::Pass, Some(g)) => {
                    let m = format!(
                        "{key} [{doc}]: stale known-gaps entry (ticket {}); the fixture passes, remove the entry",
                        g.ticket
                    );
                    report.failures.push(m.clone());
                    Outcome::Fail(vec![m])
                }
                (o, _) => o,
            };
            match &outcome {
                Outcome::Fail(p) if !p.iter().any(|m| m.contains("stale known-gaps")) => {
                    for m in p {
                        report.failures.push(format!("{key} [{doc}]: {m}"));
                    }
                }
                _ => {}
            }
            report.outcomes.insert(key, outcome);
        }
    }
    for fixture in allow.0.keys() {
        if !report.outcomes.contains_key(fixture) {
            report
                .notes
                .push(format!("{fixture}: known-gaps entry names no fixture here"));
        }
    }
    report
}
