// frob:ticket 01M4Z92C25BYMP5JYSX9WDH9XE
//! Lowering of part-array instance members (language.md 3.5; ir.md 2).
//!
//! `part axle[2]: Axle` over a component with no parameters has no per-instance definitions:
//! every instance shares the callee's arguments, so each instance member `Rover/axle[i].len`
//! carries the value of the definition `Axle.len`. They lower as members of the parent so the
//! emitters, bindings and `[*]` selectors see one path per instance (adapters.md 3). A
//! parameterized callee is not handled here.

use cadetta_ir::model::{Member, MemberKind, Provenance};
use cadetta_syntax::ast;
use tracing::{debug, trace};

use super::{Cx, evaluated_value, ir_path, member_path};
use crate::resolve::{CanonPath, Index, MemberSeg, PartSeg};

impl Cx<'_> {
    /// The members of every instance of the counted parts of `c` (`Rover/axle[1].len`).
    pub(super) fn lower_array_members(
        &self,
        canon: &CanonPath,
        package: &str,
        module: &[String],
        c: &ast::Component,
    ) -> Vec<Member> {
        let ev = self.input.evaluation;
        let mut out = Vec::new();
        for m in &c.members {
            let ast::MemberKind::Part(part) = &m.kind else {
                continue;
            };
            if part.count.is_none() || part.primary.is_some() {
                continue;
            }
            let Some((callee_path, callee)) = self.resolver.part_type(package, module, &part.ty)
            else {
                continue;
            };
            if !callee.params.is_empty() || callee_path.package != self.input.name {
                trace!(part = %part.name.name, "lower: array callee not lowered here");
                continue;
            }
            let mut slot = canon.clone();
            slot.members.push(MemberSeg::Name(part.name.name.clone()));
            let Some(&count) = ev.part_counts.get(&slot) else {
                debug!(part = %part.name.name, "lower: array members skipped, count unknown");
                continue;
            };
            for cm in &callee.members {
                let (name, private) = match &cm.kind {
                    ast::MemberKind::Param(p) => (&p.name.name, cm.private),
                    ast::MemberKind::Let(l) => (&l.name.name, cm.private),
                    _ => continue,
                };
                let def = member_path(&callee_path, name);
                let Some(e) = ev.get(&def) else { continue };
                for i in 0..count {
                    let mut key = canon.clone();
                    key.parts.push(PartSeg {
                        name: part.name.name.clone(),
                        index: Some(Index::At(i)),
                    });
                    key.members = vec![MemberSeg::Name(name.clone())];
                    let Some(path) = ir_path(&key) else { continue };
                    let Some(value) = evaluated_value(ev, &self.owners, &key, e, "") else {
                        continue;
                    };
                    let mut mem = Member::new(path, MemberKind::Param);
                    mem.private = private;
                    mem.analysis = e.analysis;
                    mem.value = Some(value);
                    mem.deps = ir_path(&def).into_iter().collect();
                    mem.provenance = Provenance::Derived;
                    out.push(mem);
                }
            }
            debug!(part = %part.name.name, count, "lower: array instance members");
        }
        out
    }
}
