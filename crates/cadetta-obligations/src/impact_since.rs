//! frob:ticket 01M4SWN7GJW88YVBJ2XVR00EKX
//! The diff of two evaluations behind `cadetta impact --since REV` / `--worktree`
//! (obligations.md 5): changed nodes and flipped verdicts.

use std::collections::BTreeMap;

use cadetta_ir::model::{Check, Member, Project as Ir, Solver, VerdictState};

/// What a changed node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeKind {
    /// A parameter, `let` or other member.
    Member,
    /// A `check` assertion.
    Check,
    /// A solver run.
    Solver,
}

impl NodeKind {
    /// The kind word used in text and JSON output.
    pub fn word(self) -> &'static str {
        match self {
            NodeKind::Member => "member",
            NodeKind::Check => "check",
            NodeKind::Solver => "solver",
        }
    }
}

/// How a node differs between the two evaluations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    /// Only in the newer evaluation.
    Added,
    /// Only in the older evaluation.
    Removed,
    /// In both, with a different value, expression, reads or inputs.
    Modified,
}

impl Change {
    /// The change word used in text and JSON output.
    pub fn word(self) -> &'static str {
        match self {
            Change::Added => "added",
            Change::Removed => "removed",
            Change::Modified => "modified",
        }
    }
}

/// One node that differs between the two evaluations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedNode {
    /// Canonical path.
    pub path: String,
    /// What the node is.
    pub kind: NodeKind,
    /// How it differs.
    pub change: Change,
}

/// One check whose primary verdict differs between the two evaluations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerdictFlip {
    /// Canonical check path.
    pub path: String,
    /// Verdict word in the older evaluation.
    pub old: &'static str,
    /// Verdict word in the newer evaluation.
    pub new: &'static str,
}

/// The difference between an older and a newer evaluation of one package.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Diff {
    /// Changed nodes, sorted by path.
    pub nodes: Vec<ChangedNode>,
    /// Flipped verdicts, sorted by path.
    pub flips: Vec<VerdictFlip>,
}

impl Diff {
    /// Paths of nodes present in the newer evaluation that changed or were added.
    pub fn live_paths(&self) -> Vec<&str> {
        self.nodes
            .iter()
            .filter(|n| n.change != Change::Removed)
            .map(|n| n.path.as_str())
            .collect()
    }
}

/// The verdict word of an optional primary verdict.
pub fn verdict_word(v: Option<VerdictState>) -> &'static str {
    match v {
        Some(VerdictState::Pass) => "pass",
        Some(VerdictState::Fail) => "fail",
        Some(VerdictState::Unknown) => "unknown",
        None => "not evaluated",
    }
}

fn member_differs(a: &Member, b: &Member) -> bool {
    a.kind != b.kind
        || a.value != b.value
        || a.expr != b.expr
        || a.deps != b.deps
        || a.bound != b.bound
        || a.role != b.role
}

fn solver_differs(a: &Solver, b: &Solver) -> bool {
    a.script != b.script
        || a.inputs != b.inputs
        || a.outputs != b.outputs
        || a.deps != b.deps
        || a.result_digest != b.result_digest
}

fn check_differs(a: &Check, b: &Check) -> bool {
    a.expr != b.expr || a.verdicts.primary != b.verdicts.primary
}

/// Compare two keyed node sets, pushing one `ChangedNode` per difference.
fn compare<T>(
    kind: NodeKind,
    old: BTreeMap<String, &T>,
    new: BTreeMap<String, &T>,
    differs: impl Fn(&T, &T) -> bool,
    out: &mut Vec<ChangedNode>,
) {
    for (path, n) in &new {
        let change = match old.get(path) {
            None => Change::Added,
            Some(o) if differs(o, n) => Change::Modified,
            Some(_) => continue,
        };
        out.push(ChangedNode {
            path: path.clone(),
            kind,
            change,
        });
    }
    for path in old.keys().filter(|p| !new.contains_key(*p)) {
        out.push(ChangedNode {
            path: path.clone(),
            kind,
            change: Change::Removed,
        });
    }
}

fn members(ir: &Ir) -> BTreeMap<String, &Member> {
    ir.components
        .iter()
        .flat_map(|c| &c.members)
        .map(|m| (m.path.to_string(), m))
        .collect()
}

fn solvers(ir: &Ir) -> BTreeMap<String, &Solver> {
    ir.solvers.iter().map(|s| (s.path.to_string(), s)).collect()
}

fn checks(ir: &Ir) -> BTreeMap<String, &Check> {
    ir.checks.iter().map(|c| (c.path.to_string(), c)).collect()
}

/// Diff `old` against `new`: changed members, solvers and checks, and flipped verdicts.
pub fn diff(old: &Ir, new: &Ir) -> Diff {
    let mut nodes = Vec::new();
    compare(
        NodeKind::Member,
        members(old),
        members(new),
        member_differs,
        &mut nodes,
    );
    compare(
        NodeKind::Solver,
        solvers(old),
        solvers(new),
        solver_differs,
        &mut nodes,
    );
    let (old_checks, new_checks) = (checks(old), checks(new));
    compare(
        NodeKind::Check,
        old_checks.clone(),
        new_checks.clone(),
        check_differs,
        &mut nodes,
    );
    nodes.sort_by(|a, b| a.path.cmp(&b.path));
    let flips: Vec<VerdictFlip> = new_checks
        .iter()
        .filter_map(|(path, n)| {
            let o = old_checks.get(path)?;
            (o.verdicts.primary != n.verdicts.primary).then(|| VerdictFlip {
                path: path.clone(),
                old: verdict_word(o.verdicts.primary),
                new: verdict_word(n.verdicts.primary),
            })
        })
        .collect();
    tracing::debug!(
        nodes = nodes.len(),
        flips = flips.len(),
        "impact since: diffed"
    );
    Diff { nodes, flips }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{Component, ComponentClass, Flavour, MemberKind, PackageId, Verdicts};
    use cadetta_ir::path::{Path, PathContext};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn ir(exprs: &[(&str, &str)], verdict: Option<VerdictState>) -> Ir {
        let mut ir = Ir::new(
            Flavour::Design,
            PackageId {
                name: "pkg".into(),
                version: "0.1.0".into(),
            },
        );
        let members = exprs
            .iter()
            .map(|(path, expr)| {
                let mut m = Member::new(p(path), MemberKind::Param);
                m.expr = Some((*expr).to_owned());
                m
            })
            .collect();
        ir.components.push(Component {
            path: p("pkg::X.a"),
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members,
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
            expr: "pkg::X.a < 9mm".into(),
            verdicts: Verdicts {
                primary: verdict,
                ..Verdicts::default()
            },
            conservative: false,
            waived_for: vec![],
        });
        ir
    }

    // frob:tests crates/cadetta-obligations/src/impact_since.rs::diff
    #[test]
    fn reports_added_removed_modified_and_flips() {
        let old = ir(
            &[("pkg::X.a", "1mm"), ("pkg::X.gone", "2mm")],
            Some(VerdictState::Pass),
        );
        let new = ir(
            &[("pkg::X.a", "3mm"), ("pkg::X.new", "2mm")],
            Some(VerdictState::Fail),
        );
        let d = diff(&old, &new);
        let got: Vec<_> = d
            .nodes
            .iter()
            .map(|n| (n.path.as_str(), n.change))
            .collect();
        assert_eq!(
            got,
            [
                ("pkg::X.a", Change::Modified),
                ("pkg::X.gone", Change::Removed),
                ("pkg::X.new", Change::Added),
                ("pkg::X.ok", Change::Modified),
            ]
        );
        assert_eq!(
            d.flips,
            [VerdictFlip {
                path: "pkg::X.ok".into(),
                old: "pass",
                new: "fail"
            }]
        );
        assert_eq!(d.live_paths(), ["pkg::X.a", "pkg::X.new", "pkg::X.ok"]);
    }

    // frob:tests crates/cadetta-obligations/src/impact_since.rs::diff
    #[test]
    fn identical_evaluations_have_an_empty_diff() {
        let a = ir(&[("pkg::X.a", "1mm")], Some(VerdictState::Pass));
        assert_eq!(diff(&a, &a.clone()), Diff::default());
    }
}
