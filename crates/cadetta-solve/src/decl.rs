//! Solver declarations: checking and lowering to the IR `Solver` record (solvers.md 1 and 2, ir.md 2).
//!
//! Decisions made here (written into solvers.md 1): an `in` expression must be a
//! quantity-valued expression built from quantity literals, name paths, field
//! reads, calls, indexing, unary minus, `+ - * / ^`, `as` and tolerances; a bare
//! number is Q001, and any other form (string, bool, array, `if`, `match`,
//! block, comparison, logic, range, `satisfies`, ISO class) is S007. The closure
//! of a solver is the sorted set of paths its inputs read plus the files whose
//! digests count as inputs (the script and each `deps` file). Outputs are
//! `bound` until a driving use is recorded with [`SolverDecl::mark_driving`].

// frob:ticket 01M4STAMX649K4GQYAXR2FAFVY

use std::collections::{BTreeMap, BTreeSet};

use cadetta_syntax::ast::{
    BinOp, CiteName, Expr, ExprKind, Solver, SolverMemberKind, Type, TypeKind, UnOp,
};
use cadetta_syntax::span::Span;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

/// How a design uses a solver output (solvers.md 2).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputRole {
    /// Compared by a check; re-solving moves geometry only on violation.
    Bound,
    /// Feeds a value; every changed re-solve creates sync obligations.
    Driving,
}

/// One solver input: its authored expression and the paths it reads.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SolverInput {
    /// The expression rendered from source.
    pub expr: String,
    /// Sorted, unique paths the expression reads.
    pub reads: Vec<String>,
}

/// One solver output: its declared kind and role.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SolverOutput {
    /// The kind or kind expression as written (`Length`, `Volume / Time`).
    pub kind: String,
    /// Bound (default) or driving.
    pub role: OutputRole,
}

/// What a solver result depends on besides its input values.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct SolverClosure {
    /// Sorted, unique paths read by any input.
    pub reads: Vec<String>,
    /// The script and each `deps` file, in declaration order, script first.
    pub files: Vec<String>,
}

// frob:ticket 01M4VB630NZKXA6Q9D7253FATR
/// A solver's method citation as written, resolved to a `CiteRef` when the IR is assembled.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DeclCitation {
    /// The cited path text, or the literal text.
    pub name: String,
    /// True for an `@"literal"` citation.
    pub literal: bool,
    /// The locator string, when given.
    pub locator: Option<String>,
}

/// The IR `Solver` record before results are attached (ir.md 2).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SolverDecl {
    /// The canonical path, such as `acme.rover::wall_sizing`.
    pub path: String,
    /// The script path from the `script` statement (its digest is an input).
    pub script: String,
    // frob:ticket 01M4VB630NZKXA6Q9D7253FATR
    /// The method citation, when written; provenance only, never an input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<DeclCitation>,
    /// Inputs by name.
    pub inputs: BTreeMap<String, SolverInput>,
    /// Outputs by name.
    pub outputs: BTreeMap<String, SolverOutput>,
    /// Extra files whose digests count as inputs.
    pub deps: Vec<String>,
    /// What the solver reads and the files it hashes.
    pub closure: SolverClosure,
}

impl SolverDecl {
    /// Mark `output` driving (it feeds a value); returns false when no such output exists.
    pub fn mark_driving(&mut self, output: &str) -> bool {
        match self.outputs.get_mut(output) {
            Some(o) => {
                debug!(solver = %self.path, output, "output marked driving");
                o.role = OutputRole::Driving;
                true
            }
            None => false,
        }
    }
}

/// A declaration diagnostic with its fixed code.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub struct SolverDeclError {
    /// `Q001`, `S007` or `E003`.
    pub code: &'static str,
    /// The message.
    pub message: String,
    /// The primary span.
    pub span: Span,
}

fn fail(code: &'static str, span: Span, message: String) -> SolverDeclError {
    warn!(code, %message, "solver declaration error");
    SolverDeclError {
        code,
        message,
        span,
    }
}

/// Render a kind or kind expression as written.
fn render_kind(ty: &Type) -> Option<String> {
    match &ty.kind {
        TypeKind::Path(p) => Some(p.text()),
        TypeKind::KindExpr(k) => {
            let mut out = String::new();
            for (i, t) in k.terms.iter().enumerate() {
                if i > 0 {
                    out.push_str(match t.op {
                        cadetta_syntax::ast::KindOp::Mul => " * ",
                        cadetta_syntax::ast::KindOp::Div => " / ",
                    });
                }
                out.push_str(&t.name.text());
                if let Some(e) = t.exp {
                    out.push_str(&format!("^{e}"));
                }
            }
            Some(out)
        }
        _ => None,
    }
}

/// The dotted read path of a pure name/field chain, if `e` is one.
fn chain(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Path(p) => Some(p.text()),
        ExprKind::Field(b, f) => chain(b).map(|s| format!("{s}.{}", f.name)),
        _ => None,
    }
}

/// Check that `e` is quantity-valued and collect the paths it reads.
fn check_expr(e: &Expr, reads: &mut BTreeSet<String>) -> Result<(), SolverDeclError> {
    let bad = |what: &str| {
        Err(fail(
            "S007",
            e.span,
            format!("solver input is not a quantity expression: {what}"),
        ))
    };
    match &e.kind {
        ExprKind::Quantity(_) => Ok(()),
        ExprKind::Number(n) => Err(fail(
            "Q001",
            e.span,
            format!("bare number `{n}` as a solver input; write a quantity with a unit"),
        )),
        ExprKind::Path(_) | ExprKind::Field(..) => {
            reads.insert(chain(e).expect("path or field chain"));
            Ok(())
        }
        ExprKind::Paren(x) | ExprKind::As(x, _) => check_expr(x, reads),
        ExprKind::Unary(UnOp::Neg, x) => check_expr(x, reads),
        ExprKind::Unary(UnOp::Not, _) => bad("`not`"),
        ExprKind::Binary(op, l, r) => match op {
            BinOp::Add | BinOp::Sub => {
                check_expr(l, reads)?;
                check_expr(r, reads)
            }
            BinOp::Mul | BinOp::Div | BinOp::Pow => {
                check_operand(l, reads)?;
                check_operand(r, reads)
            }
            _ => bad("logic, range or membership operator"),
        },
        ExprKind::Tolerance(x, _) | ExprKind::Position(x, _) => check_expr(x, reads),
        ExprKind::Index(x, i) => {
            check_expr(x, reads)?;
            // The index only selects; it is read like any value.
            collect_loose(i, reads);
            Ok(())
        }
        ExprKind::Call { callee, args, .. } => {
            // The callee is a function name, not a read.
            if chain(callee).is_none() {
                return bad("call of a non-name");
            }
            for a in args {
                check_operand(&a.value, reads)?;
            }
            Ok(())
        }
        ExprKind::Str(_) => bad("string"),
        ExprKind::Bool(_) => bad("boolean"),
        ExprKind::Array(_) => bad("array"),
        ExprKind::Compare { .. } => bad("comparison"),
        ExprKind::Satisfies(..) => bad("`satisfies`"),
        ExprKind::IsoClass(..) => bad("ISO class"),
        ExprKind::If { .. } => bad("`if`"),
        ExprKind::Match { .. } => bad("`match`"),
        ExprKind::Block(_) => bad("block"),
    }
}

/// Like [`check_expr`] but a bare number is a legal scalar factor, exponent or argument.
fn check_operand(e: &Expr, reads: &mut BTreeSet<String>) -> Result<(), SolverDeclError> {
    match &e.kind {
        ExprKind::Number(_) => Ok(()),
        ExprKind::Unary(UnOp::Neg, x) if matches!(x.kind, ExprKind::Number(_)) => Ok(()),
        _ => check_expr(e, reads),
    }
}

/// Record name reads in an index expression without enforcing quantity form.
fn collect_loose(e: &Expr, reads: &mut BTreeSet<String>) {
    if let Some(c) = chain(e) {
        reads.insert(c);
    }
}

/// Render an input expression back to source-like text for the IR.
fn render_expr(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Quantity(segs) => format!("{segs:?}"),
        _ => chain(e).unwrap_or_else(|| "<expr>".to_string()),
    }
}

/// Check a `solver` declaration and lower it to a [`SolverDecl`] at canonical `path`.
pub fn lower_solver(s: &Solver, path: &str) -> Result<SolverDecl, Vec<SolverDeclError>> {
    debug!(path, "lowering solver declaration");
    let mut errs = Vec::new();
    let script = s.script.clone();
    // frob:ticket 01M4VB630NZKXA6Q9D7253FATR
    let citation = s.citation.as_ref().map(|c| DeclCitation {
        name: match &c.name {
            CiteName::Literal(t) => t.clone(),
            CiteName::Named(p) => p.text(),
        },
        literal: matches!(c.name, CiteName::Literal(_)),
        locator: c.locator.clone(),
    });
    let mut inputs = BTreeMap::new();
    let mut outputs = BTreeMap::new();
    let mut deps = Vec::new();
    let mut all_reads = BTreeSet::new();
    for m in &s.members {
        match &m.kind {
            SolverMemberKind::In(binds) => {
                for b in binds {
                    let mut reads = BTreeSet::new();
                    if let Err(e) = check_expr(&b.value, &mut reads) {
                        errs.push(e);
                        continue;
                    }
                    if inputs.contains_key(&b.name.name) {
                        errs.push(fail(
                            "E003",
                            b.span,
                            format!("duplicate solver input `{}`", b.name.name),
                        ));
                        continue;
                    }
                    trace!(input = %b.name.name, reads = reads.len(), "solver input accepted");
                    all_reads.extend(reads.iter().cloned());
                    inputs.insert(
                        b.name.name.clone(),
                        SolverInput {
                            expr: render_expr(&b.value),
                            reads: reads.into_iter().collect(),
                        },
                    );
                }
            }
            SolverMemberKind::Out { name, ty } => {
                let Some(kind) = ty.as_ref().and_then(render_kind) else {
                    errs.push(fail(
                        "S007",
                        ty.as_ref().map_or(m.span, |t| t.span),
                        format!(
                            "solver output `{}` needs a kind or kind expression",
                            name.name
                        ),
                    ));
                    continue;
                };
                if outputs.contains_key(&name.name) {
                    errs.push(fail(
                        "E003",
                        name.span,
                        format!("duplicate solver output `{}`", name.name),
                    ));
                    continue;
                }
                outputs.insert(
                    name.name.clone(),
                    SolverOutput {
                        kind,
                        role: OutputRole::Bound,
                    },
                );
            }
            SolverMemberKind::Deps(f) => deps.push(f.clone()),
            // Checks on a solver's own outputs are evaluated like any other check (solvers.md 5).
            SolverMemberKind::Check(_) | SolverMemberKind::Error => {}
        }
    }
    if !errs.is_empty() {
        return Err(errs);
    }
    let mut files = vec![script.clone()];
    files.extend(deps.iter().cloned());
    Ok(SolverDecl {
        path: path.to_string(),
        script,
        citation,
        inputs,
        outputs,
        deps,
        closure: SolverClosure {
            reads: all_reads.into_iter().collect(),
            files,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::ItemKind;
    use cadetta_syntax::parser::parse;

    fn solver(src: &str) -> Solver {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        p.file
            .entries
            .iter()
            .find_map(|e| match e {
                cadetta_syntax::ast::FileEntry::Item(i) => match &i.kind {
                    ItemKind::Solver(s) => Some(s.clone()),
                    _ => None,
                },
                _ => None,
            })
            .expect("solver")
    }

    const OK: &str = "solver wall_sizing {\n  script \"analysis/wall_sizing.py\"\n  in load = Frame.max_load, span = Frame.span\n  out min_wall: Length\n  out q: Volume / Time\n  deps \"analysis/material_db.csv\"\n}\n";

    // frob:ticket 01M4VB630NZKXA6Q9D7253FATR
    // frob:tests crates/cadetta-solve/src/decl.rs::lower_solver kind=unit
    #[test]
    fn citation_is_provenance_and_not_an_input() {
        let d = lower_solver(
            &solver("solver s @roark:\"eq 3\" {\n  script \"s.py\"\n  out o: Length\n}\n"),
            "p::s",
        )
        .unwrap();
        let c = d.citation.expect("citation");
        assert_eq!((c.name.as_str(), c.literal), ("roark", false));
        assert_eq!(c.locator.as_deref(), Some("eq 3"));
        assert_eq!(d.closure.files, vec!["s.py".to_string()]);
        let d = lower_solver(&solver("solver s {\n  script \"s.py\"\n}\n"), "p::s").unwrap();
        assert!(d.citation.is_none());
    }

    // frob:tests crates/cadetta-solve/src/decl.rs::lower_solver kind=unit
    #[test]
    fn lowers_with_closure() {
        let d = lower_solver(&solver(OK), "acme.rover::wall_sizing").expect("ok");
        assert_eq!(d.path, "acme.rover::wall_sizing");
        assert_eq!(d.script, "analysis/wall_sizing.py");
        assert_eq!(d.inputs.len(), 2);
        assert_eq!(d.outputs["q"].kind, "Volume / Time");
        assert_eq!(d.closure.reads, ["Frame.max_load", "Frame.span"]);
        assert_eq!(
            d.closure.files,
            ["analysis/wall_sizing.py", "analysis/material_db.csv"]
        );
    }

    // frob:tests crates/cadetta-solve/src/decl.rs::lower_solver kind=unit
    #[test]
    fn rejects_non_quantity_inputs() {
        for (src, code) in [
            ("a = 3", "Q001"),
            ("a = \"x\"", "S007"),
            ("a = true", "S007"),
            ("a = [1mm, 2mm]", "S007"),
            ("a = 1mm < 2mm", "S007"),
        ] {
            let text = format!("solver s {{\n  script \"s.py\"\n  in {src}\n  out o: Length\n}}\n");
            let errs = lower_solver(&solver(&text), "p::s").expect_err(src);
            assert_eq!(errs[0].code, code, "{src}");
        }
        let ok = "solver s {\n  script \"s.py\"\n  in a = 2 * x.y + 1mm [+-0.1mm], b = f(z.w)\n  out o: Length\n}\n";
        assert!(lower_solver(&solver(ok), "p::s").is_ok());
    }

    // frob:tests crates/cadetta-solve/src/decl.rs::SolverDecl.mark_driving kind=unit
    #[test]
    fn outputs_are_bound_until_driving() {
        let mut d = lower_solver(&solver(OK), "p::s").expect("ok");
        assert!(d.outputs.values().all(|o| o.role == OutputRole::Bound));
        assert!(d.mark_driving("min_wall"));
        assert_eq!(d.outputs["min_wall"].role, OutputRole::Driving);
        assert_eq!(d.outputs["q"].role, OutputRole::Bound);
        assert!(!d.mark_driving("nope"));
    }

    // frob:tests crates/cadetta-solve/src/decl.rs::lower_solver kind=unit
    #[test]
    fn duplicates_are_e003() {
        let t = "solver s {\n  script \"s.py\"\n  in a = 1mm, a = 2mm\n  out o: Length\n  out o: Length\n}\n";
        let errs = lower_solver(&solver(t), "p::s").expect_err("dup");
        assert_eq!(errs.len(), 2);
        assert!(errs.iter().all(|e| e.code == "E003"));
    }
}
