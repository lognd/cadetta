//! Bolt float lowering: a non-locating `ScrewJoint`'s `at`/`onto` become an `EdgeFloat`
//! (geometry.md 5.2).
//!
//! The geometry crate evaluates [`EdgeFloat`]s; this module builds them. A joint contributes to
//! the placement edge whose two parts are the part its `at` pattern lies on and the part its
//! `onto` pattern lies on, whichever of the two is the placement's parent and whichever datum
//! the placement names. A joint between parts that are not parent and child adds nothing.
//!
//! Decisions made here: the `onto` points are re-expressed through the float-free nominal frame
//! table into the child's placement datum, the frame the geometry crate applies the float in;
//! the three source variables of a joint (x, y, rotation) are allocated on first use and kept,
//! so every frame table of one evaluation reads the same variables; a joint whose operands do
//! not evaluate adds no float here (its own `aligned` check reports the failure).
//!
//! A joint inside a sub-assembly component floats the placement edge of that sub-assembly's own
//! [`Assembly`]: the sub-assembly is lowered with the same root frame table, its datum ids
//! prefixed by the path of the node that holds it.

// frob:ticket 01M4Y2Z95B99FTVX68T4XTSTXR
// frob:ticket 01M4ZFQ6AH3V8MVCZY6SC8DQHP
use std::rc::Rc;

use cadetta_geometry::float::{EdgeFloat, JointFloat};
use cadetta_geometry::placement::{Anchor, Assembly};
use cadetta_geometry::point::DatumId;
use cadetta_geometry::reexpress::{FrameTable, Reexpressed};
use cadetta_geometry::transform::Coord;
use cadetta_syntax::ast::{Component, MemberKind};
use cadetta_units::Affine;
use tracing::{debug, trace};

use super::geom::{Ref, datum_id, to_pos};
use super::instances::InstInfo;
use super::{Evaluator, Fail, R, Scope, Val, diag};
use crate::resolve::{CanonPath, MemberSeg};

/// The std component whose instances float (geometry.md 5.2).
const JOINT: &str = "ScrewJoint";

/// True when `info` instantiates the standard `ScrewJoint`.
pub(super) fn is_joint(info: &InstInfo) -> bool {
    info.callee.package == "std" && info.callee.item == JOINT
}

/// The key of member `name` of the instance `prefix`.
fn member_key(prefix: &CanonPath, name: &str) -> CanonPath {
    let mut key = prefix.clone();
    key.members = vec![MemberSeg::Name(name.to_owned())];
    key
}

/// Every point of a re-expressed pattern as (x, y) in the target datum.
fn flatten(r: &Reexpressed, out: &mut Vec<[Affine; 2]>) -> Option<()> {
    match r {
        Reexpressed::Point(l) => {
            let exact = |c: &Coord| match c {
                Coord::Exact(a) => Some(a.clone()),
                Coord::Approx(_) => None,
            };
            out.push([exact(l.coords.first()?)?, exact(l.coords.get(1)?)?]);
            Some(())
        }
        Reexpressed::Array(xs) => xs.iter().try_for_each(|x| flatten(x, out)),
    }
}

/// Every float of `a` and of its sub-assemblies.
pub(super) fn all_floats(a: &Assembly) -> Vec<EdgeFloat> {
    let mut out = a.floats.clone();
    for sub in a.nodes.iter().filter_map(|n| n.sub.as_deref()) {
        out.extend(all_floats(sub));
    }
    out
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// The float of every placement edge of `a` that a `ScrewJoint` of `comp` lies across.
    ///
    /// `nominal` is the frame table of `a` read without floats; joints on one edge are all kept,
    /// for `float::select` to choose between. The floats of sub-assemblies are installed in
    /// place (`a.floats` itself is left to the caller).
    pub(super) fn bolt_floats(
        &mut self,
        here: &CanonPath,
        comp: &'p Component,
        a: &mut Assembly,
        nominal: &FrameTable,
    ) -> Vec<EdgeFloat> {
        self.lower_floats(here, comp, a, nominal, &[])
    }

    /// [`Self::bolt_floats`] for the assembly `a` of `comp` at node path `at` below the root.
    fn lower_floats(
        &mut self,
        here: &CanonPath,
        comp: &'p Component,
        a: &mut Assembly,
        nominal: &FrameTable,
        at: &[String],
    ) -> Vec<EdgeFloat> {
        for i in 0..a.nodes.len() {
            let name = a.nodes[i].name.clone();
            if a.nodes[i].sub.is_none() {
                continue;
            }
            let Some(part) = comp.members.iter().find_map(|m| match &m.kind {
                MemberKind::Part(p) if p.name.name == name => Some(p),
                _ => None,
            }) else {
                continue;
            };
            let Some((callee, sub_comp)) =
                self.resolver
                    .part_type(&here.package, &here.modules, &part.ty)
            else {
                continue;
            };
            let mut path = at.to_vec();
            path.push(name);
            let mut sub = a.nodes[i].sub.take();
            if let Some(s) = sub.as_deref_mut() {
                let floats = self.lower_floats(&callee, sub_comp, s, nominal, &path);
                debug!(sub = %path.join("/"), edges = floats.len(), "bolt float: sub-assembly lowered");
                s.floats = floats;
            }
            a.nodes[i].sub = sub;
        }
        let joints: Vec<Rc<InstInfo>> = self
            .instances
            .members()
            .filter(|(i, ..)| {
                is_joint(i)
                    && i.prefix.package == here.package
                    && i.prefix.modules == here.modules
                    && i.prefix.item == here.item
                    && i.prefix.parts.len() == 1
            })
            .map(|(i, ..)| i)
            .collect();
        let mut edges: Vec<EdgeFloat> = Vec::new();
        for info in joints {
            let found = match self.joint_float(comp, a, nominal, at, &info) {
                Ok(f) => f,
                Err(Fail::Silent) | Err(Fail::Diag(_)) => {
                    debug!(joint = %info.prefix, "bolt float: operands do not evaluate; no float");
                    None
                }
            };
            let Some((node, joint)) = found else { continue };
            match edges.iter_mut().find(|e| e.node == node) {
                Some(e) => e.joints.push(joint),
                None => edges.push(EdgeFloat {
                    node,
                    joints: vec![joint],
                }),
            }
        }
        debug!(edges = edges.len(), "bolt float: edges lowered");
        edges
    }

    /// The joint's claim on its edge as (child node, claim); `None` when it lies on no edge.
    fn joint_float(
        &mut self,
        comp: &'p Component,
        a: &Assembly,
        nominal: &FrameTable,
        at: &[String],
        info: &Rc<InstInfo>,
    ) -> R<Option<(String, JointFloat)>> {
        let (Some((ea, ctx)), Some((eb, _))) = (
            self.given_arg(&info.prefix, "at"),
            self.given_arg(&info.prefix, "onto"),
        ) else {
            trace!(joint = %info.prefix, "bolt float: joint has no at/onto");
            return Ok(None);
        };
        let (Some((Ref::Value { parts: ap, .. }, _)), Some((Ref::Value { parts: bp, tag }, _))) = (
            self.walk_ref(comp, &ctx, ea, ea.span)?,
            self.walk_ref(comp, &ctx, eb, eb.span)?,
        ) else {
            return Ok(None);
        };
        let (Some(x), Some(y)) = (ap.first(), bp.first()) else {
            return Ok(None);
        };
        let edge = a.placements.iter().find(|p| match &p.at {
            Anchor::Datum { node, .. } => {
                (p.node == *x && node == y) || (p.node == *y && node == x)
            }
            Anchor::Origin => false,
        });
        let Some(p) = edge else {
            trace!(joint = %info.prefix, "bolt float: parts are not parent and child");
            return Ok(None);
        };
        let mut caller = Scope {
            ctx,
            locals: Vec::new(),
        };
        // The caller's operands are read where the caller wrote them, outside any instance.
        let saved = self.cur_inst.take();
        let onto = self.eval(&mut caller, eb);
        self.cur_inst = saved;
        let rooted = |parts: &[String]| [at, parts].concat();
        let pos = to_pos(
            &onto?,
            &DatumId::new(datum_id(&rooted(&bp), &tag)),
            true,
            eb.span,
        )?;
        let target = DatumId::new(datum_id(&rooted(std::slice::from_ref(&p.node)), &p.datum));
        let moved = nominal
            .reexpress(&pos, &target)
            .map_err(|e| diag("EVAL", eb.span, e.to_string()))?;
        let mut points = Vec::new();
        flatten(&moved, &mut points)
            .ok_or_else(|| diag("EVAL", eb.span, "an inexact onto point has no float"))?;
        let float = match self.value_of(&member_key(&info.prefix, "float")) {
            Some(super::Evaluated {
                value: Val::Quantity(q),
                ..
            }) => q.form,
            _ => return Err(Fail::Silent),
        };
        let locating = matches!(
            self.value_of(&member_key(&info.prefix, "locating")),
            Some(super::Evaluated {
                value: Val::Bool(true),
                ..
            })
        );
        let vars = self.joint_vars(&info.prefix);
        debug!(joint = %info.prefix, child = %p.node, points = points.len(), locating, "bolt float: joint lowered");
        Ok(Some((
            p.node.clone(),
            JointFloat {
                path: info.prefix.to_string(),
                float,
                locating,
                onto: points,
                vars,
            },
        )))
    }

    /// The joint's x, y and rotation source variables, allocated once.
    fn joint_vars(&mut self, prefix: &CanonPath) -> [cadetta_units::VarId; 3] {
        if let Some(v) = self.float_vars.get(prefix) {
            return *v;
        }
        let v = [self.fresh_var(), self.fresh_var(), self.fresh_var()];
        self.float_vars.insert(prefix.clone(), v);
        v
    }
}
