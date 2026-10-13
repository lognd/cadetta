//! frob:ticket 01M4ST8F5G8E0FMC3KZJG8A95T
//! `cadetta why PATH`: the upstream derivation chain of a value, back to citations, solvers
//! and checks (obligations.md 5, config-cli.md 3).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use cadetta_ir::model::{Check, Member, Project as Ir, Solver};
use serde_json::{Value as Json, json};

use crate::output::Format;
use crate::project::Project;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Schema tag of the JSON report; the schema is `docs/schemas/why.json`.
pub const SCHEMA: &str = "cadetta.why/1";

const USAGE: &str = "usage: cadetta why PATH";

/// What a node of the derivation chain is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// A parameter, `let` or other member.
    Member,
    /// A `check` assertion.
    Check,
    /// A solver run that produced a value.
    Solver,
    /// A citation: the source of a value.
    Citation,
    /// A path the IR does not define (a table, a dependency or a stale name).
    Unresolved,
}

impl NodeKind {
    /// The kind word used in text and JSON output.
    pub fn word(self) -> &'static str {
        match self {
            NodeKind::Member => "member",
            NodeKind::Check => "check",
            NodeKind::Solver => "solver",
            NodeKind::Citation => "citation",
            NodeKind::Unresolved => "unresolved",
        }
    }
}

/// One node of the chain, with what it reads one level further up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Canonical path, or the citation name.
    pub path: String,
    /// What the node is.
    pub kind: NodeKind,
    /// The value as authored, when the node has one.
    pub value: Option<String>,
    /// The normalized expression, when the node is computed.
    pub expr: Option<String>,
    /// A one-line detail: citation target and locator, solver script, check verdict.
    pub detail: Option<String>,
    /// True when the node already appeared earlier in the chain and is not expanded again.
    pub repeated: bool,
    /// The nodes this one reads, citations first, then sorted by path.
    pub upstream: Vec<Node>,
}

/// `PATH` names nothing the IR defines.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("`{0}` is not a member, check or solver of this design")]
pub struct NotFound(pub String);

struct Index<'a> {
    members: BTreeMap<String, &'a Member>,
    checks: BTreeMap<String, &'a Check>,
    solvers: BTreeMap<String, &'a Solver>,
    /// Member path to the solver that outputs it.
    produced_by: BTreeMap<String, String>,
}

impl<'a> Index<'a> {
    fn new(ir: &'a Ir) -> Self {
        let mut idx = Index {
            members: BTreeMap::new(),
            checks: BTreeMap::new(),
            solvers: BTreeMap::new(),
            produced_by: BTreeMap::new(),
        };
        for m in ir.components.iter().flat_map(|c| &c.members) {
            idx.members.insert(m.path.to_string(), m);
        }
        for c in &ir.checks {
            idx.checks.insert(c.path.to_string(), c);
        }
        for s in &ir.solvers {
            let sp = s.path.to_string();
            for out in s.outputs.values() {
                idx.produced_by.insert(out.clone(), sp.clone());
            }
            idx.solvers.insert(sp, s);
        }
        idx
    }

    fn build(&self, path: &str, seen: &mut BTreeSet<String>) -> Node {
        let mut node = Node {
            path: path.to_owned(),
            kind: NodeKind::Unresolved,
            value: None,
            expr: None,
            detail: None,
            repeated: false,
            upstream: Vec::new(),
        };
        let known = self.members.contains_key(path)
            || self.checks.contains_key(path)
            || self.solvers.contains_key(path);
        if known && !seen.insert(path.to_owned()) {
            tracing::trace!(path, "why: already expanded");
            node.repeated = true;
            node.kind = self.kind_of(path);
            return node;
        }
        let mut reads: Vec<String> = Vec::new();
        if let Some(m) = self.members.get(path) {
            node.kind = NodeKind::Member;
            node.value = m.value.as_ref().map(|v| v.authored.clone());
            node.expr = m.expr.clone();
            if let Some(c) = &m.citation {
                let target = match &c.locator {
                    Some(l) => format!("{} ({l})", c.target),
                    None => c.target.clone(),
                };
                node.upstream.push(Node {
                    path: c.name.clone(),
                    kind: NodeKind::Citation,
                    value: None,
                    expr: None,
                    detail: Some(format!("{target} [{:?}]", c.kind).to_lowercase()),
                    repeated: false,
                    upstream: Vec::new(),
                });
            }
            reads.extend(m.deps.iter().map(|d| d.to_string()));
            if let Some(solver) = self.produced_by.get(path) {
                reads.push(solver.clone());
            }
        } else if let Some(c) = self.checks.get(path) {
            node.kind = NodeKind::Check;
            node.expr = Some(c.expr.clone());
            node.detail = Some(format!("primary verdict {:?}", c.verdicts.primary).to_lowercase());
            reads.extend(
                self.members
                    .keys()
                    .filter(|m| c.expr.contains(m.as_str()))
                    .cloned(),
            );
        } else if let Some(s) = self.solvers.get(path) {
            node.kind = NodeKind::Solver;
            node.detail = Some(format!(
                "script {}{}",
                s.script,
                if s.stale { " (result stale)" } else { "" }
            ));
            reads.extend(s.deps.iter().map(|d| d.to_string()));
            reads.extend(s.inputs.values().cloned());
        }
        reads.sort();
        reads.dedup();
        reads.retain(|r| r != path);
        for r in reads {
            let child = self.build(&r, seen);
            node.upstream.push(child);
        }
        tracing::debug!(
            path,
            kind = node.kind.word(),
            upstream = node.upstream.len(),
            "why: node"
        );
        node
    }

    fn kind_of(&self, path: &str) -> NodeKind {
        if self.members.contains_key(path) {
            NodeKind::Member
        } else if self.checks.contains_key(path) {
            NodeKind::Check
        } else {
            NodeKind::Solver
        }
    }
}

/// Trace `path` upstream through the IR: members, citations, solvers and checks.
pub fn trace(ir: &Ir, path: &str) -> Result<Node, NotFound> {
    let idx = Index::new(ir);
    if !idx.members.contains_key(path)
        && !idx.checks.contains_key(path)
        && !idx.solvers.contains_key(path)
    {
        tracing::warn!(path, "why: path not in the IR");
        return Err(NotFound(path.to_owned()));
    }
    Ok(idx.build(path, &mut BTreeSet::new()))
}

fn node_json(n: &Node) -> Json {
    json!({
        "path": n.path,
        "kind": n.kind.word(),
        "value": n.value,
        "expr": n.expr,
        "detail": n.detail,
        "repeated": n.repeated,
        "upstream": n.upstream.iter().map(node_json).collect::<Vec<_>>(),
    })
}

fn text_lines(n: &Node, depth: usize, out: &mut dyn Write) {
    let mut line = format!("{}{} [{}]", "  ".repeat(depth), n.path, n.kind.word());
    if let Some(v) = &n.value {
        line.push_str(&format!(" = {v}"));
    }
    if let Some(e) = &n.expr {
        line.push_str(&format!(" := {e}"));
    }
    if let Some(d) = &n.detail {
        line.push_str(&format!(" ({d})"));
    }
    if n.repeated {
        line.push_str(" (see above)");
    }
    let _ = writeln!(out, "{line}");
    for u in &n.upstream {
        text_lines(u, depth + 1, out);
    }
}

/// Render the chain as indented text or the `cadetta.why/1` JSON document.
pub fn render(root: &Node, format: Format, out: &mut dyn Write) {
    if format == Format::Json {
        let doc = json!({
            "schema": SCHEMA,
            "path": root.path,
            "chain": node_json(root),
        });
        let _ = writeln!(out, "{doc}");
    } else {
        text_lines(root, 0, out);
    }
}

/// The evaluated IR of the project.
///
/// The source-to-IR lowering is not wired into the CLI yet (as for `obligations`), so a
/// project offers none and every path is unknown.
fn project_ir(project: &Project) -> Option<Ir> {
    tracing::warn!(
        root = %project.root.display(),
        "why: no IR pipeline wired into the CLI yet; nothing to trace"
    );
    None
}

/// Run `cadetta why`; `args` are the words after the verb. Returns the exit code.
pub fn run_why(args: &[String], globals: &Globals, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "why failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let [path] = args else {
        return fail(err, USAGE);
    };
    if path.starts_with("--") {
        return fail(err, &format!("unknown flag `{path}`; {USAGE}"));
    }
    let project = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok((project, _scope)) => project,
        Err(e) => return fail(err, &e.to_string()),
    };
    let Some(ir) = project_ir(&project) else {
        return fail(err, &NotFound(path.clone()).to_string());
    };
    match trace(&ir, path) {
        Ok(root) => {
            render(&root, globals.format, out);
            EXIT_OK
        }
        Err(e) => fail(err, &e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{
        Citation, CitationKind, Component, ComponentClass, Flavour, MemberKind, PackageId,
        Value as IrValue,
    };
    use cadetta_ir::model::{Exact, Nominal};
    use cadetta_ir::path::{Path, PathContext};
    use cadetta_units::{Dim, Magnitude, Rational};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn member(path: &str, deps: &[&str], cite: bool) -> Member {
        let mut m = Member::new(p(path), MemberKind::Param);
        m.deps = deps.iter().map(|d| p(d)).collect();
        m.expr = (!deps.is_empty()).then(|| "a + b".to_owned());
        m.value = Some(IrValue::new(
            Nominal::Exact(Exact(Magnitude::new(Rational::from_int(5), 0))),
            Dim::default(),
            "5mm",
        ));
        if cite {
            m.citation = Some(Citation {
                name: "iso286".into(),
                target: "ISO 286-1".into(),
                locator: Some("tab 1".into()),
                kind: CitationKind::Standard,
                file_digest: None,
            });
        }
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
        let mut comp = Component {
            path: p("pkg::X.a").clone(),
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members: vec![
                member("pkg::X.a", &[], true),
                member("pkg::X.b", &["pkg::X.a"], false),
                member("pkg::X.c", &["pkg::X.a", "pkg::X.b"], false),
            ],
            parts: vec![],
            fits: vec![],
            span: None,
            digest: None,
        };
        comp.path = p("pkg::X.a");
        ir.components.push(comp);
        ir
    }

    // frob:tests crates/cadetta-cli/src/cmd/why.rs::trace
    #[test]
    fn chain_reaches_citation_and_marks_repeats() {
        let root = trace(&ir(), "pkg::X.c").unwrap();
        assert_eq!(root.kind, NodeKind::Member);
        let paths: Vec<_> = root.upstream.iter().map(|n| n.path.as_str()).collect();
        assert_eq!(paths, ["pkg::X.a", "pkg::X.b"]);
        let a = &root.upstream[0];
        assert_eq!(a.upstream[0].kind, NodeKind::Citation);
        assert_eq!(a.upstream[0].path, "iso286");
        // `a` is read again by `b`; the second visit is not expanded.
        assert!(root.upstream[1].upstream[0].repeated);
        assert!(root.upstream[1].upstream[0].upstream.is_empty());
    }

    // frob:tests crates/cadetta-cli/src/cmd/why.rs::trace
    #[test]
    fn unknown_path_is_not_found() {
        assert_eq!(
            trace(&ir(), "pkg::X.nope"),
            Err(NotFound("pkg::X.nope".into()))
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/why.rs::render
    #[test]
    fn text_is_indented_chain() {
        let root = trace(&ir(), "pkg::X.b").unwrap();
        let mut out = Vec::new();
        render(&root, Format::Text, &mut out);
        let s = String::from_utf8(out).unwrap();
        let lines: Vec<_> = s.lines().collect();
        assert!(lines[0].starts_with("pkg::X.b [member] = 5mm := a + b"));
        assert!(lines[1].starts_with("  pkg::X.a [member]"));
        assert!(lines[2].starts_with("    iso286 [citation]"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/why.rs::render
    #[test]
    fn json_validates_against_why_schema() {
        let root = trace(&ir(), "pkg::X.c").unwrap();
        let mut out = Vec::new();
        render(&root, Format::Json, &mut out);
        let v: Json = serde_json::from_slice(&out).unwrap();
        let schema: Json =
            serde_json::from_str(include_str!("../../../../docs/schemas/why.json")).unwrap();
        assert_eq!(v["schema"], SCHEMA);
        assert_eq!(schema["properties"]["schema"]["const"], SCHEMA);
        check_node(&v["chain"], &schema["$defs"]["Node"]);
        let required: Vec<_> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap())
            .collect();
        for key in required {
            assert!(v.get(key).is_some(), "missing top-level `{key}`");
        }
        assert_eq!(v["chain"]["upstream"][0]["upstream"][0]["kind"], "citation");
    }

    /// Check a node against the schema's `Node` definition: required keys, kind enum, recursion.
    fn check_node(n: &Json, def: &Json) {
        for key in def["required"].as_array().unwrap() {
            assert!(n.get(key.as_str().unwrap()).is_some(), "missing {key}");
        }
        let kinds = def["properties"]["kind"]["enum"].as_array().unwrap();
        assert!(kinds.contains(&n["kind"]));
        let props = def["properties"].as_object().unwrap();
        for k in n.as_object().unwrap().keys() {
            assert!(props.contains_key(k), "undeclared `{k}`");
        }
        for u in n["upstream"].as_array().unwrap() {
            check_node(u, def);
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/why.rs::run_why
    #[test]
    fn usage_errors_exit_two() {
        let g = Globals {
            format: Format::Text,
            cwd: None,
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run_why(&[], &g, &mut out, &mut err), EXIT_USAGE);
        assert_eq!(
            run_why(&["--stale".into()], &g, &mut out, &mut err),
            EXIT_USAGE
        );
        assert!(out.is_empty());
    }
}
