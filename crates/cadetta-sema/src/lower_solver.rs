// frob:ticket 01M4ZJAYRJMG47610S0R0ANADA
//! Lowering of `solver` members and `waive` items into the IR (ir.md 3 `Solver`, `Waiver`;
//! solvers.md 1, 4; lints.md 2.1).
//!
//! A solver lowers to `PKG::Comp.name` with its `script`, one `inputs` entry per `in` binding
//! (the binding's expression text), one `outputs` entry per `out` (the path of the recorded
//! output member), the members its inputs read as `deps`, and its `@cite` as `citation`
//! (documentation only, never an input). The recorded-result state
//! (`result_digest`, `stale`) is the caller's, because only the caller reads the result file.

use std::collections::BTreeMap;

use cadetta_ir::entities::Waiver;
use cadetta_ir::model::{Digest, Solver};
use cadetta_ir::variant_name::{VariantError, VariantName};
use cadetta_syntax::ast;
use tracing::{debug, trace, warn};

use super::{Cx, SourceText, ir_path, member_path, text_of};
use crate::cite::CiteIndex;
use crate::resolve::CanonPath;

/// What the caller knows of one solver's recorded primary result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolverState {
    /// Digest of the recorded result; `None` when the solver has no recorded result.
    pub result_digest: Option<Digest>,
    /// True when the inputs, script or deps changed since the result was recorded (S003).
    pub stale: bool,
}

/// Lowers one `waive` item; the glob is kept as written.
pub fn lower_waiver(w: &ast::Waive) -> Result<Waiver, VariantError> {
    let for_alt = match &w.for_alt {
        None => None,
        Some(a) => Some(VariantName::parse(&format!(
            "{}={}",
            a.slot.text, a.alt.text
        ))?),
    };
    debug!(code = %w.code.name, glob = %w.glob.text, "lower: waiver");
    let mut out = Waiver::new(w.code.name.clone(), w.glob.text.clone(), w.because.clone());
    out.expires = w.expires.as_ref().map(|e| e.text.clone());
    out.for_alt = for_alt;
    Ok(out)
}

impl Cx<'_> {
    /// The solver members of component `comp`, in declaration order.
    pub(super) fn lower_solvers(
        &self,
        comp: &CanonPath,
        src: Option<&SourceText<'_>>,
        idx: &CiteIndex,
        module: (&str, &[String]),
        c: &ast::Component,
    ) -> Vec<Solver> {
        let mut out = Vec::new();
        for m in &c.members {
            let ast::MemberKind::Solver(sv) = &m.kind else {
                continue;
            };
            let canon = member_path(comp, &sv.name.name);
            let Some(path) = ir_path(&canon) else {
                continue;
            };
            let mut inputs = BTreeMap::new();
            let mut outputs = BTreeMap::new();
            for member in &sv.members {
                match &member.kind {
                    ast::SolverMemberKind::In(binds) => {
                        for b in binds {
                            inputs.insert(b.name.name.clone(), text_of(src, b.value.span));
                        }
                    }
                    ast::SolverMemberKind::Out { name, .. } => {
                        outputs.insert(
                            name.name.clone(),
                            member_path(&canon, &name.name).to_string(),
                        );
                    }
                    ast::SolverMemberKind::Deps(_)
                    | ast::SolverMemberKind::Check(_)
                    | ast::SolverMemberKind::Error => {}
                }
            }
            let deps = self
                .input
                .deps
                .deps_of(&canon)
                .flat_map(|d| self.solver_output_nodes(d))
                .filter_map(|d| self.dep_node(d))
                .filter_map(|d| ir_path(&d))
                .collect();
            let state = self.input.solver_states.get(&canon);
            if state.is_none() {
                trace!(solver = %canon, "lower: solver has no recorded result state");
            }
            debug!(solver = %canon, inputs = inputs.len(), outputs = outputs.len(), "lower: solver");
            let citation = sv.citation.as_ref().and_then(|cc| {
                idx.lower_ref(module.0, module.1, cc)
                    .map_err(|e| warn!(solver = %canon, code = e.code, "lower: solver citation unresolved"))
                    .ok()
            });
            out.push(Solver {
                path,
                script: sv.script.clone(),
                citation,
                inputs,
                outputs,
                deps,
                result_digest: state.and_then(|s| s.result_digest.clone()),
                stale: state.is_some_and(|s| s.stale),
            });
        }
        if out.is_empty() {
            trace!(component = %comp, "lower: component has no solvers");
        }
        out
    }
}

/// Logs a waiver that could not be lowered.
pub(super) fn skip_waiver(w: &ast::Waive, e: &VariantError) {
    warn!(code = %w.code.name, error = %e, "lower: waiver with a bad `for alt` skipped");
}
