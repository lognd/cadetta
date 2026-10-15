// frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
//! Evaluates `sum|max|min(self, m)` and `m(self)` roll-ups (language.md 3.5).
//!
//! The pure reduction lives in [`crate::rollup`]; this module flattens the part tree of the
//! enclosing component into contributions, one per part instance, and maps a failed reduction
//! to an `EVAL` diagnostic. A part that has the member contributes it and the walk stops
//! there; a part without it but with parts of its own is searched through; a leaf without it
//! contributes nothing (a missing contribution, an error unless the `?` form is used).

use std::rc::Rc;

use cadetta_ir::canon::PartSeg;
use cadetta_syntax::ast::{Component, MemberKind, Part};
use cadetta_syntax::span::Span;
use tracing::{debug, trace};

use super::{Evaluator, Fail, Quant, R, Scope, Val, diag, instances::InstInfo};
use crate::parts::{self, ArgSource};
use crate::resolve::{CanonPath, Ctx, MemberSeg};
use crate::rollup::{self, Contribution, RollOp};

/// The component whose parts are being walked, and how to evaluate expressions written in it.
struct Owner<'p> {
    /// The component definition's path.
    path: CanonPath,
    /// The component.
    comp: &'p Component,
    /// The context its own expressions are evaluated in.
    ctx: Ctx,
    /// The instance the owner is, when it is a parameterized callee.
    inst: Option<Rc<InstInfo>>,
    /// The instance path shown in messages.
    label: String,
}

/// What a roll-up found at one part slot.
enum Found {
    /// The member's value.
    Value(Quant),
    /// The part does not have the member.
    Lacks,
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// Evaluates a roll-up over every part instance of the enclosing component.
    // frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
    pub(super) fn roll_up_call(
        &mut self,
        sc: &Scope,
        op: RollOp,
        param: &str,
        optional: bool,
        span: Span,
    ) -> R<Val> {
        let resolver = self.resolver;
        let Some(item) = sc.ctx.component.as_deref() else {
            return Err(diag(
                "EVAL",
                span,
                format!(
                    "`{}(self, {param})` needs an enclosing component",
                    op.name()
                ),
            ));
        };
        let path = CanonPath::item(&sc.ctx.package, &sc.ctx.module, item);
        let Some(comp) = resolver.component(&path) else {
            return Err(diag("EVAL", span, format!("no component `{item}`")));
        };
        let owner = Owner {
            path,
            comp,
            ctx: Ctx {
                locals: Vec::new(),
                ..sc.ctx.clone()
            },
            inst: self.cur_inst.clone(),
            label: item.to_owned(),
        };
        let mut items = Vec::new();
        self.roll_walk(&owner, param, &mut items, span)?;
        debug!(
            op = op.name(),
            param,
            parts = items.len(),
            "roll-up flattened"
        );
        match rollup::roll_up(op, param, optional, &items) {
            Ok(r) => {
                if let Some(note) = r.note(param) {
                    debug!(%note, "roll-up note");
                }
                Ok(Val::Quantity(r.value))
            }
            Err(e) => Err(diag("EVAL", span, e.to_string())),
        }
    }

    /// Appends one contribution per instance of every part of `owner`, searching below parts
    /// that lack `param`.
    fn roll_walk(
        &mut self,
        owner: &Owner<'p>,
        param: &str,
        out: &mut Vec<Contribution>,
        span: Span,
    ) -> R<()> {
        for m in &owner.comp.members {
            let MemberKind::Part(part) = &m.kind else {
                continue;
            };
            let counted = part.count.is_some();
            let n = self.slot_count(owner, part, span)?;
            for i in 0..n {
                let label = if counted {
                    format!("{}/{}[{i}]", owner.label, part.name.name)
                } else {
                    format!("{}/{}", owner.label, part.name.name)
                };
                self.roll_slot(owner, part, param, label, out, span)?;
            }
        }
        Ok(())
    }

    /// The number of instances a `part` slot makes (1 without a count).
    fn slot_count(&mut self, owner: &Owner<'p>, part: &Part, span: Span) -> R<u32> {
        let Some(e) = &part.count else {
            return Ok(1);
        };
        let mut sc = Scope {
            ctx: owner.ctx.clone(),
            locals: Vec::new(),
        };
        let saved = std::mem::replace(&mut self.cur_inst, owner.inst.clone());
        let q = self.quant(&mut sc, e);
        self.cur_inst = saved;
        parts::array_count(&q?).map_err(|e| diag("EVAL", span, e.to_string()))
    }

    /// One instance of one slot: its value, or a search below it.
    fn roll_slot(
        &mut self,
        owner: &Owner<'p>,
        part: &Part,
        param: &str,
        label: String,
        out: &mut Vec<Contribution>,
        span: Span,
    ) -> R<()> {
        let resolver = self.resolver;
        // An instance of a parameterized callee, materialised under `Owner/part`.
        let mut prefix = owner.path.clone();
        prefix.parts.push(PartSeg {
            name: part.name.name.clone(),
            index: None,
        });
        let (found, child) = if owner.inst.is_none()
            && let Some((info, comp, ctx)) = self.instances.lookup(&prefix)
        {
            let mut key = prefix.clone();
            key.members = vec![MemberSeg::Name(param.to_owned())];
            let found = self.roll_member(&key, param, span)?;
            let child = Owner {
                path: info.callee.clone(),
                comp,
                ctx,
                inst: Some(info),
                label: label.clone(),
            };
            (found, Some(child))
        } else {
            let Some((cpath, ccomp)) =
                resolver.slot_component(&owner.path.package, &owner.path.modules, part)
            else {
                trace!(slot = %part.name.name, "roll-up: slot type unknown");
                out.push(Contribution {
                    path: label,
                    value: None,
                });
                return Ok(());
            };
            let found = match (
                &owner.inst,
                ccomp.params.iter().any(|p| p.name.name == param),
            ) {
                (Some(info), true) => self.arg_value(owner, info.clone(), part, ccomp, param),
                _ => {
                    let mut key = cpath.clone();
                    key.members = vec![MemberSeg::Name(param.to_owned())];
                    self.roll_member(&key, param, span)?
                }
            };
            let child = ccomp.params.is_empty().then(|| Owner {
                ctx: Ctx {
                    package: cpath.package.clone(),
                    module: cpath.modules.clone(),
                    component: Some(ccomp.name.name.clone()),
                    locals: Vec::new(),
                },
                path: cpath,
                comp: ccomp,
                inst: None,
                label: label.clone(),
            });
            (found, child)
        };
        match found {
            Found::Value(q) => out.push(Contribution {
                path: label,
                value: Some(q),
            }),
            Found::Lacks => match child {
                Some(c)
                    if c.comp
                        .members
                        .iter()
                        .any(|m| matches!(m.kind, MemberKind::Part(_))) =>
                {
                    self.roll_walk(&c, param, out, span)?;
                }
                _ => out.push(Contribution {
                    path: label,
                    value: None,
                }),
            },
        }
        Ok(())
    }

    /// The quantity defined at `key`, if the member exists and is present.
    fn roll_member(&mut self, key: &CanonPath, param: &str, span: Span) -> R<Found> {
        if !self.defs.contains_key(key) || self.absent.contains(key) {
            return Ok(Found::Lacks);
        }
        match self.value_of(key).map(|e| e.value) {
            Some(Val::Quantity(q)) => Ok(Found::Value(q)),
            Some(_) => Err(diag(
                "EVAL",
                span,
                format!("roll-up member `{param}` is not a quantity"),
            )),
            None => Err(Fail::Silent),
        }
    }

    /// The argument a nested part of an instance passes for `param`, evaluated in the owner.
    fn arg_value(
        &mut self,
        owner: &Owner<'p>,
        info: Rc<InstInfo>,
        part: &Part,
        ccomp: &'p Component,
        param: &str,
    ) -> Found {
        let args = part.args.as_deref().unwrap_or(&[]);
        let Ok(bound) = parts::bind_args(&ccomp.name.name, &ccomp.params, args) else {
            return Found::Lacks;
        };
        let Some(ArgSource::Given(expr, _)) = bound
            .into_iter()
            .find(|b| b.name == param)
            .map(|b| b.source)
        else {
            return Found::Lacks;
        };
        let mut sc = Scope {
            ctx: owner.ctx.clone(),
            locals: Vec::new(),
        };
        let saved = self.cur_inst.replace(info);
        let r = self.quant(&mut sc, expr);
        self.cur_inst = saved;
        match r {
            Ok(q) => Found::Value(q),
            Err(_) => {
                debug!(param, slot = %part.name.name, "roll-up: argument not available");
                Found::Lacks
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{file, run};
    use super::super::{Evaluation, Val};
    use crate::resolve::{CanonPath, MemberSeg};
    use cadetta_units::{Magnitude, Rational};

    fn at(comp: &str, member: &str) -> CanonPath {
        let mut p = CanonPath::item("p", &[], comp);
        p.members.push(MemberSeg::Name(member.to_owned()));
        p
    }

    /// The nominal of `Box.<member>` in metres, as thousandths.
    fn milli(ev: &Evaluation, member: &str) -> Option<Magnitude> {
        match &ev.get(&at("Box", member))?.value {
            Val::Quantity(q) => Some(q.nominal.clone()),
            _ => None,
        }
    }

    fn m(n: i64, d: i64) -> Option<Magnitude> {
        Some(Magnitude::new(Rational::new(n, d).expect("denominator"), 0))
    }

    fn eval_errors(ev: &Evaluation) -> usize {
        ev.diagnostics.iter().filter(|d| d.code == "EVAL").count()
    }

    const LEAVES: &str = "assembly Two { unit_cost = 2mm }\n\
        assembly Five { unit_cost = 5mm }\n\
        assembly Bare { x = 1mm }\n";

    // frob:tests sum_counts_every_instance_of_an_array
    // frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
    #[test]
    fn sum_counts_every_instance_of_an_array() {
        let f = file(&format!(
            "{LEAVES}assembly Box {{\n  part a[4]: Two\n  part b: Five\n  \
             let total = sum(self, unit_cost)\n}}\n"
        ));
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(milli(&ev, "total"), m(13, 1000));
    }

    // frob:tests max_and_min_pick_the_winner
    // frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
    #[test]
    fn max_and_min_pick_the_winner() {
        let f = file(&format!(
            "{LEAVES}assembly Box {{\n  part a: Two\n  part b: Five\n  \
             let hi = max(self, unit_cost)\n  let lo = min(self, unit_cost)\n}}\n"
        ));
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(milli(&ev, "hi"), m(5, 1000));
        assert_eq!(milli(&ev, "lo"), m(2, 1000));
    }

    // frob:tests misspelled_member_is_eval_not_e002
    // frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
    #[test]
    fn misspelled_member_is_eval_not_e002() {
        let f = file(&format!(
            "{LEAVES}assembly Box {{\n  part a: Bare\n  let hi = max(self, unit_cost)\n}}\n"
        ));
        let ev = run(&f);
        assert_eq!(eval_errors(&ev), 1, "{:?}", ev.diagnostics);
        assert!(ev.diagnostics.iter().all(|d| d.code != "E002"));
    }

    // frob:tests optional_form_skips_parts_without_the_member
    // frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
    #[test]
    fn optional_form_skips_parts_without_the_member() {
        let f = file(&format!(
            "{LEAVES}assembly Box {{\n  part a: Two\n  part b: Bare\n  \
             let strict = sum(self, unit_cost)\n  let loose = sum?(self, unit_cost)\n}}\n"
        ));
        let ev = run(&f);
        assert_eq!(eval_errors(&ev), 1, "{:?}", ev.diagnostics);
        assert_eq!(milli(&ev, "loose"), m(2, 1000));
        assert!(milli(&ev, "strict").is_none());
    }

    // frob:tests optional_form_with_nothing_to_count_is_eval
    // frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
    #[test]
    fn optional_form_with_nothing_to_count_is_eval() {
        let f = file(&format!(
            "{LEAVES}assembly Box {{\n  part a: Bare\n  let none = sum?(self, unit_cost)\n}}\n"
        ));
        let ev = run(&f);
        assert_eq!(eval_errors(&ev), 1, "{:?}", ev.diagnostics);
    }

    // frob:tests mixed_dimensions_are_eval
    // frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
    #[test]
    fn mixed_dimensions_are_eval() {
        let f = file(
            "assembly Len { unit_cost = 2mm }\nassembly Mass { unit_cost = 2kg }\n\
             assembly Box {\n  part a: Len\n  part b: Mass\n  \
             let total = sum(self, unit_cost)\n}\n",
        );
        let ev = run(&f);
        assert_eq!(eval_errors(&ev), 1, "{:?}", ev.diagnostics);
    }

    // frob:tests search_continues_below_parts_without_the_member
    // frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
    #[test]
    fn search_continues_below_parts_without_the_member() {
        let f = file(&format!(
            "{LEAVES}assembly Sub {{ part a[2]: Two }}\n\
             assembly Box {{\n  part s: Sub\n  part b: Five\n  \
             let total = sum(self, unit_cost)\n}}\n"
        ));
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(milli(&ev, "total"), m(9, 1000));
    }

    const JOINT: &str = "assembly Screw(unit_cost?: Length) { x = 1mm }\n\
        assembly Joint(n: Length, screw_cost?: Length) {\n  \
        part screw[2]: Screw(unit_cost = screw_cost)\n}\n";

    // frob:tests parameterized_part_contributes_the_nested_argument
    // frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
    #[test]
    fn parameterized_part_contributes_the_nested_argument() {
        let f = file(&format!(
            "{JOINT}assembly Box {{\n  part j: Joint(n = 1mm, screw_cost = 3mm)\n  \
             let total = sum(self, unit_cost)\n}}\n"
        ));
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(milli(&ev, "total"), m(6, 1000));
    }

    // frob:tests parameterized_part_without_the_argument_is_eval
    // frob:ticket 01M4YSGF4CF0MBQG2K4A50KFBS
    #[test]
    fn parameterized_part_without_the_argument_is_eval() {
        let f = file(&format!(
            "{JOINT}assembly Box {{\n  part j: Joint(n = 1mm)\n  \
             let total = sum(self, unit_cost)\n}}\n"
        ));
        let ev = run(&f);
        assert_eq!(eval_errors(&ev), 1, "{:?}", ev.diagnostics);
    }
}
