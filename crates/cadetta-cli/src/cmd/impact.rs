//! frob:ticket 01M4ST8F7B9VYMFSCJD8MVBJNS
//! `cadetta impact PATH`: everything downstream of a value that goes stale or may change
//! verdict if it changes (obligations.md 5, config-cli.md 3).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Write;

use cadetta_ir::model::Project as Ir;
use serde_json::{Value as Json, json};

use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::Project;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Schema tag of the JSON report; the schema is `docs/schemas/impact.json`.
pub const SCHEMA: &str = "cadetta.impact/1";

const USAGE: &str = "usage: cadetta impact PATH";

/// What a downstream node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A parameter, `let` or other member.
    Member,
    /// A `check` assertion.
    Check,
    /// A solver run.
    Solver,
}

impl Kind {
    /// The kind word used in text and JSON output.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Member => "member",
            Kind::Check => "check",
            Kind::Solver => "solver",
        }
    }
}

/// One downstream node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependent {
    /// Canonical path.
    pub path: String,
    /// What the node is.
    pub kind: Kind,
    /// The node it reads that leads back to the queried path.
    pub via: String,
    /// Reads between this node and the queried path (at least 1).
    pub depth: usize,
}

impl Dependent {
    /// The effect word: members change, solver results go stale, checks may flip.
    pub fn effect(&self) -> &'static str {
        match self.kind {
            Kind::Member => "changes",
            Kind::Solver => "stale",
            Kind::Check => "may_flip",
        }
    }
}

/// The impact of changing one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Impact {
    /// The path asked about.
    pub path: String,
    /// Downstream nodes, nearest first, then sorted by path.
    pub dependents: Vec<Dependent>,
    /// Obligation ids (`solver:PATH` stale, `check:PATH` may flip), sorted.
    pub obligations: Vec<(String, &'static str)>,
}

/// `PATH` names nothing the IR defines.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("`{0}` is not a member, check or solver of this design")]
pub struct NotFound(pub String);

/// Walk the reverse read-graph of `ir` from `path`, breadth first.
pub fn downstream(ir: &Ir, path: &str) -> Result<Impact, NotFound> {
    let mut kinds: BTreeMap<String, Kind> = BTreeMap::new();
    // Reader lists: node -> nodes that read it.
    let mut readers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let members: Vec<String> = ir
        .components
        .iter()
        .flat_map(|c| &c.members)
        .map(|m| m.path.to_string())
        .collect();
    for m in ir.components.iter().flat_map(|c| &c.members) {
        let mp = m.path.to_string();
        kinds.insert(mp.clone(), Kind::Member);
        for d in &m.deps {
            readers.entry(d.to_string()).or_default().insert(mp.clone());
        }
    }
    for s in &ir.solvers {
        let sp = s.path.to_string();
        kinds.insert(sp.clone(), Kind::Solver);
        let reads = s
            .deps
            .iter()
            .map(|d| d.to_string())
            .chain(s.inputs.values().cloned());
        for r in reads {
            readers.entry(r).or_default().insert(sp.clone());
        }
        // A solver output is read from the solver.
        for out in s.outputs.values() {
            readers.entry(sp.clone()).or_default().insert(out.clone());
        }
    }
    for c in &ir.checks {
        let cp = c.path.to_string();
        kinds.insert(cp.clone(), Kind::Check);
        for m in members
            .iter()
            .filter(|m| crate::universe::check_reads(&cp, &c.expr, m))
        {
            readers.entry(m.clone()).or_default().insert(cp.clone());
        }
    }
    if !kinds.contains_key(path) {
        tracing::warn!(path, "impact: path not in the IR");
        return Err(NotFound(path.to_owned()));
    }
    let mut seen: BTreeSet<String> = BTreeSet::from([path.to_owned()]);
    let mut queue: VecDeque<(String, usize)> = VecDeque::from([(path.to_owned(), 0)]);
    let mut dependents = Vec::new();
    while let Some((node, depth)) = queue.pop_front() {
        for r in readers.get(&node).into_iter().flatten() {
            if !seen.insert(r.clone()) {
                continue;
            }
            let Some(&kind) = kinds.get(r) else { continue };
            tracing::trace!(node, reader = %r, depth = depth + 1, "impact: reader");
            dependents.push(Dependent {
                path: r.clone(),
                kind,
                via: node.clone(),
                depth: depth + 1,
            });
            queue.push_back((r.clone(), depth + 1));
        }
    }
    dependents.sort_by(|a, b| (a.depth, &a.path).cmp(&(b.depth, &b.path)));
    let mut obligations: Vec<(String, &'static str)> = dependents
        .iter()
        .filter_map(|d| match d.kind {
            Kind::Solver => Some((format!("solver:{}", d.path), "stale")),
            Kind::Check => Some((format!("check:{}", d.path), "may_flip")),
            Kind::Member => None,
        })
        .collect();
    obligations.sort();
    tracing::debug!(path, dependents = dependents.len(), "impact: computed");
    Ok(Impact {
        path: path.to_owned(),
        dependents,
        obligations,
    })
}

/// Render the impact as text or the `cadetta.impact/1` JSON document.
pub fn render(impact: &Impact, format: Format, out: &mut dyn Write) {
    if format == Format::Json {
        let doc = json!({
            "schema": SCHEMA,
            "path": impact.path,
            "dependents": impact.dependents.iter().map(|d| json!({
                "path": d.path,
                "kind": d.kind.word(),
                "via": d.via,
                "depth": d.depth,
                "effect": d.effect(),
            })).collect::<Vec<Json>>(),
            "obligations": impact.obligations.iter()
                .map(|(id, effect)| json!({"id": id, "effect": effect}))
                .collect::<Vec<Json>>(),
        });
        let _ = writeln!(out, "{doc}");
        return;
    }
    if impact.dependents.is_empty() {
        let _ = writeln!(out, "{}: no dependents", impact.path);
        return;
    }
    let _ = writeln!(
        out,
        "{}: {} dependents",
        impact.path,
        impact.dependents.len()
    );
    for d in &impact.dependents {
        let _ = writeln!(
            out,
            "  {} [{}] {} (via {})",
            d.path,
            d.kind.word(),
            d.effect(),
            d.via
        );
    }
    for (id, effect) in &impact.obligations {
        let _ = writeln!(out, "  obligation {id}: {effect}");
    }
}

/// Run `cadetta impact`; `args` are the words after the verb. Returns the exit code.
pub fn run_impact(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "impact failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let [path] = args else {
        return fail(err, USAGE);
    };
    if path.starts_with("--") {
        return fail(err, &format!("unknown flag `{path}`; {USAGE}"));
    }
    let (project, members) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref())
    {
        Ok(o) => o,
        Err(e) => return fail(err, &e.to_string()),
    };
    let analysis = analyze(&project, &members);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    for package in &analysis.packages {
        if let Ok(impact) = downstream(&package.ir, path) {
            render(&impact, globals.format, out);
            return EXIT_OK;
        }
    }
    fail(err, &NotFound(path.clone()).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{
        Check, Component, ComponentClass, Flavour, Member, MemberKind, PackageId, Solver, Verdicts,
    };
    use cadetta_ir::path::{Path, PathContext};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn member(path: &str, deps: &[&str]) -> Member {
        let mut m = Member::new(p(path), MemberKind::Param);
        m.deps = deps.iter().map(|d| p(d)).collect();
        m
    }

    fn ir() -> Ir {
        let mut ir = Ir::new(
            Flavour::Design,
            PackageId {
                name: "pkg".into(),
                version: "0.1.0".into(),
            },
        );
        ir.components.push(Component {
            path: p("pkg::X.a"),
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members: vec![
                member("pkg::X.a", &[]),
                member("pkg::X.b", &["pkg::X.a"]),
                member("pkg::X.c", &["pkg::X.b"]),
                member("pkg::X.lonely", &[]),
            ],
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        });
        ir.checks.push(Check {
            path: p("pkg::X.ok"),
            expr: "pkg::X.c < 9mm".into(),
            verdicts: Verdicts::default(),
            conservative: false,
            waived_for: vec![],
        });
        ir.solvers.push(Solver {
            path: p("pkg::X.fea"),
            script: "fea.py".into(),
            citation: None,
            inputs: BTreeMap::from([("w".into(), "pkg::X.b".into())]),
            outputs: BTreeMap::new(),
            deps: vec![],
            result_digest: None,
            stale: false,
        });
        ir
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact.rs::downstream
    #[test]
    fn lists_members_checks_and_solvers_downstream() {
        let i = downstream(&ir(), "pkg::X.a").unwrap();
        let got: Vec<_> = i
            .dependents
            .iter()
            .map(|d| (d.path.as_str(), d.depth))
            .collect();
        assert_eq!(
            got,
            [
                ("pkg::X.b", 1),
                ("pkg::X.c", 2),
                ("pkg::X.fea", 2),
                ("pkg::X.ok", 3)
            ]
        );
        assert_eq!(
            i.obligations,
            [
                ("check:pkg::X.ok".to_owned(), "may_flip"),
                ("solver:pkg::X.fea".to_owned(), "stale")
            ]
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact.rs::render
    #[test]
    fn no_dependents_is_said_plainly() {
        let i = downstream(&ir(), "pkg::X.lonely").unwrap();
        assert!(i.dependents.is_empty());
        let mut out = Vec::new();
        render(&i, Format::Text, &mut out);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "pkg::X.lonely: no dependents\n"
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact.rs::downstream
    #[test]
    fn unknown_path_is_not_found() {
        assert_eq!(
            downstream(&ir(), "pkg::X.nope"),
            Err(NotFound("pkg::X.nope".into()))
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact.rs::render
    #[test]
    fn json_validates_against_impact_schema() {
        let schema: Json =
            serde_json::from_str(include_str!("../../../../docs/schemas/impact.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for path in ["pkg::X.a", "pkg::X.lonely"] {
            let mut out = Vec::new();
            render(&downstream(&ir(), path).unwrap(), Format::Json, &mut out);
            let v: Json = serde_json::from_slice(&out).unwrap();
            assert!(validator.is_valid(&v), "{v}");
            assert_eq!(v["schema"], SCHEMA);
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact.rs::run_impact
    #[test]
    fn usage_errors_exit_two() {
        let g = Globals {
            format: Format::Text,
            cwd: None,
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run_impact(&[], &g, &mut out, &mut err), EXIT_USAGE);
        assert_eq!(
            run_impact(&["--swap".into()], &g, &mut out, &mut err),
            EXIT_USAGE
        );
        assert!(out.is_empty());
    }
}
