//! Geometry verification and frame driving (adapters.md 3.4): compare a datum's frame or a
//! placement's instance transform with the composed design transform, and plan the
//! `datums = "create"` push of a composed datum into a coordinate system.
//!
//! The transforms here are plain numeric 3x4 matrices in the adapter's native unit: the caller
//! turns `cadetta_geometry`'s composed `Transform` (and the observed `frames[]`/`instances[]`
//! values) into [`Mat34`] and this module only compares, so no transform is composed here
//! beyond the one instance-times-datum product the spec names.
//!
//! Composition rule (to move into adapters.md 3.4): a datum's frame is its parent's frame followed
//! by its ops left to right, each op in the running local frame (`offset` translates, `rotate`
//! uses the angle Member, `flip` is a half turn); `<Type>.origin` is the identity; translations
//! are divided by the adapter's native unit. A placement's anchor frame is the frame of its `at`
//! datum followed by the placement ops. When `at` is a datum of another part, that part is the
//! unique placement in the same component whose placed datum has that type (matching the array
//! index for an indexed placement); its node frame is its anchor frame times the inverse of its
//! placed datum's frame. An ambiguous or missing node is skipped with a warning, never guessed.
//! Variants compose with the base ops.
// frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN

use cadetta_ir::geometry::{Axis, Op, Placement};
use cadetta_ir::model::{Component, LocatorKind, Nominal, Project, Value};
use cadetta_ir::path::{Path, Seg};
use serde::Deserialize;

use crate::bindings::mangle;
use crate::classify::R001;
use crate::rpc::Feature;
use crate::sync::{Diag, Entry};

/// The `[[adapter]] datums` policy (config-cli.md 2, adapters.md 3.4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatumPolicy {
    /// Datums are verified by transform only (default).
    #[default]
    Verify,
    /// Missing coordinate systems are created and driven from the composed transform.
    Create,
}

/// What the host does with datum frames for one adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatumAction {
    /// Compare only; never write.
    Verify,
    /// Create missing frames and push the composed transform like a value.
    Drive,
}

/// Resolves the datum action: `create` takes effect only with the `create_frames` feature.
pub fn datum_action(policy: DatumPolicy, features: &[Feature]) -> DatumAction {
    match policy {
        DatumPolicy::Create if features.contains(&Feature::CreateFrames) => DatumAction::Drive,
        DatumPolicy::Create => {
            tracing::debug!("datums = create has no effect: adapter lacks create_frames");
            DatumAction::Verify
        }
        DatumPolicy::Verify => DatumAction::Verify,
    }
}

/// True when a locator kind may ever be pushed: an `instance` (a placement) is verify-only.
pub fn kind_pushable(kind: LocatorKind) -> bool {
    !matches!(kind, LocatorKind::Instance)
}

/// A rotation source op, to decide whether a mate or dimension may be pushed for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationOp {
    /// `flip(axis)`, a half turn.
    Flip,
    /// `rotate(axis = angle)` with the angle in degrees.
    Rotate {
        /// The angle in degrees.
        degrees: f64,
    },
}

/// True when the rotation has a native dimension to push; `flip` and exact 90deg turns never do.
pub fn rotation_pushable(op: RotationOp) -> bool {
    match op {
        RotationOp::Flip => false,
        RotationOp::Rotate { degrees } => (degrees / 90.0 - (degrees / 90.0).round()).abs() > 1e-9,
    }
}

/// A 3x4 transform: three rows of rotation columns 0..3 then the translation in column 3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat34(pub [[f64; 4]; 3]);

/// A malformed observed transform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatError {
    /// The rows are not 3x4.
    Shape,
    /// A frame literal could not be parsed.
    Literal(String),
}

impl std::fmt::Display for MatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shape => write!(f, "transform must be 3 rows of 4 entries"),
            Self::Literal(e) => write!(f, "frame literal is not a 3x4 array of numbers: {e}"),
        }
    }
}

impl std::error::Error for MatError {}

impl Mat34 {
    /// Builds from observed rows; translations (column 3) are divided by `native_scale`, the SI
    /// length of one native unit (0.001 for mm), so they read in the native unit.
    pub fn from_observed(rows: &[Vec<Value>], native_scale: f64) -> Result<Self, MatError> {
        if rows.len() != 3 || rows.iter().any(|r| r.len() != 4) {
            return Err(MatError::Shape);
        }
        let mut m = [[0.0; 4]; 3];
        for (i, row) in rows.iter().enumerate() {
            for (j, v) in row.iter().enumerate() {
                let x = match &v.nominal {
                    Nominal::Exact(e) => e.0.to_f64(),
                    Nominal::Inexact(f) => f.f64,
                };
                m[i][j] = if j == 3 { x / native_scale } else { x };
            }
        }
        Ok(Self(m))
    }

    /// The composition `self * other` (apply `other` first), as the instance transform times a
    /// part's datum frame.
    pub fn compose(&self, other: &Mat34) -> Mat34 {
        let (a, b) = (&self.0, &other.0);
        let mut out = [[0.0; 4]; 3];
        for i in 0..3 {
            for j in 0..4 {
                out[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum::<f64>()
                    + if j == 3 { a[i][3] } else { 0.0 };
            }
        }
        Mat34(out)
    }

    /// Fixed-precision literal, a JSON array of three rows; the compared and recorded form.
    pub fn literal(&self, precision: u32) -> String {
        let p = precision as usize;
        let num = |x: f64| {
            let s = format!("{x:.p$}");
            // normalize -0.000 so equal transforms print equal
            if s.trim_start_matches('-')
                .chars()
                .all(|c| c == '0' || c == '.')
            {
                s.trim_start_matches('-').to_owned()
            } else {
                s
            }
        };
        let rows: Vec<String> = self
            .0
            .iter()
            .map(|r| {
                format!(
                    "[{}]",
                    r.iter().map(|x| num(*x)).collect::<Vec<_>>().join(", ")
                )
            })
            .collect();
        format!("[{}]", rows.join(", "))
    }

    /// Parses a [`Mat34::literal`] back.
    pub fn parse_literal(text: &str) -> Result<Self, MatError> {
        let rows: Vec<Vec<f64>> =
            serde_json::from_str(text).map_err(|e| MatError::Literal(e.to_string()))?;
        if rows.len() != 3 || rows.iter().any(|r| r.len() != 4) {
            return Err(MatError::Shape);
        }
        let mut m = [[0.0; 4]; 3];
        for (i, r) in rows.iter().enumerate() {
            m[i].copy_from_slice(r);
        }
        Ok(Self(m))
    }

    /// True when every entry matches within `0.5 * 10^-precision` (adapters.md 3.4).
    pub fn matches(&self, read: &Mat34, precision: u32) -> bool {
        let tol = 0.5 * 10f64.powi(-(precision as i32));
        let slack = tol * (1.0 + 1e-9);
        self.0
            .iter()
            .flatten()
            .zip(read.0.iter().flatten())
            .all(|(d, r)| (d - r).abs() <= slack)
    }
}

/// What a transform comparison is about, for the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    /// A datum's frame.
    Datum,
    /// A placement's instance.
    Placement,
}

impl Subject {
    fn word(self) -> &'static str {
        match self {
            Self::Datum => "datum frame",
            Self::Placement => "placement",
        }
    }
}

/// R001 on `path` when `read` differs from `design`, showing both transforms; else `None`.
pub fn compare(
    subject: Subject,
    path: &str,
    design: &Mat34,
    read: &Mat34,
    precision: u32,
) -> Option<Diag> {
    if design.matches(read, precision) {
        tracing::debug!(path, "transform matches");
        return None;
    }
    tracing::warn!(path, code = R001, "transform mismatch");
    Some(Diag {
        code: R001,
        message: format!(
            "{path}: {} transform differs: design {} read {}",
            subject.word(),
            design.literal(precision),
            read.literal(precision)
        ),
    })
}

/// Verifies a datum: its read frame against the composed datum; a missing frame is not judged here.
pub fn verify_datum(
    path: &str,
    composed: &Mat34,
    read: Option<&Mat34>,
    precision: u32,
) -> Option<Diag> {
    compare(Subject::Datum, path, composed, read?, precision)
}

/// Verifies a placement: `instance * part_datum` (the placed datum's frame in the assembly, the
/// part's datum read when bound, else composed) against the anchor's composed frame.
pub fn verify_placement(
    path: &str,
    instance: &Mat34,
    part_datum: &Mat34,
    anchor: &Mat34,
    precision: u32,
) -> Option<Diag> {
    let placed = instance.compose(part_datum);
    compare(Subject::Placement, path, anchor, &placed, precision)
}

/// Where a datum frame lives: the sync coordinates of one datum.
#[derive(Debug, Clone, Copy)]
pub struct DatumSite<'a> {
    /// The adapter name.
    pub adapter: &'a str,
    /// The native file.
    pub file: &'a str,
    /// The design path of the datum.
    pub path: &'a str,
    /// The coordinate system's name (the datum's created name).
    pub frame: &'a str,
    /// The last synced literal, if any.
    pub base: Option<&'a str>,
}

/// The sync entry of a datum frame under `datums = "create"`.
///
/// `ours` is the composed transform literal, `theirs` the read frame or the empty string when the
/// frame is missing (so a first sync pushes and creates it). A read frame equal within precision
/// is recorded as equal, so rounding noise is never drift.
pub fn datum_entry(
    site: DatumSite<'_>,
    composed: &Mat34,
    read: Option<&Mat34>,
    precision: u32,
) -> Entry {
    let ours = composed.literal(precision);
    let theirs = match read {
        Some(r) if composed.matches(r, precision) => ours.clone(),
        Some(r) => r.literal(precision),
        None => String::new(),
    };
    tracing::debug!(
        path = site.path,
        frame = site.frame,
        missing = read.is_none(),
        "datum frame entry"
    );
    Entry {
        adapter: site.adapter.to_owned(),
        file: site.file.to_owned(),
        path: site.path.to_owned(),
        locator: site.frame.to_owned(),
        base: site.base.map(str::to_owned),
        ours,
        theirs,
        replaces: None,
        ours_tol: None,
        theirs_tol: None,
    }
}

// ---------------------------------------------------------------------------------------------
// Glue: evaluated IR -> composed Mat34 -> verification and datum entries (adapters.md 3.4).
// ---------------------------------------------------------------------------------------------

const IDENTITY: Mat34 = Mat34([
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
]);

/// Longest datum or placement chain followed before the glue gives up (cycle guard).
const MAX_DEPTH: usize = 64;

impl Mat34 {
    /// The inverse of a rigid transform (rotation transposed, translation negated and rotated).
    // frob:ticket 01M4VTJ6W0X6YHWEEHT645FHJZ
    pub fn inverse(&self) -> Mat34 {
        let a = &self.0;
        let mut out = [[0.0; 4]; 3];
        for i in 0..3 {
            for j in 0..3 {
                out[i][j] = a[j][i];
            }
            out[i][3] = -(0..3).map(|k| a[k][i] * a[k][3]).sum::<f64>();
        }
        Mat34(out)
    }

    /// The pure rotation by `angle` radians about `axis`.
    // frob:ticket 01M4VTJ6W0X6YHWEEHT645FHJZ
    pub fn rotation(axis: Axis, angle: f64) -> Mat34 {
        let (s, c) = angle.sin_cos();
        let m = match axis {
            Axis::X => [[1.0, 0.0, 0.0, 0.0], [0.0, c, -s, 0.0], [0.0, s, c, 0.0]],
            Axis::Y => [[c, 0.0, s, 0.0], [0.0, 1.0, 0.0, 0.0], [-s, 0.0, c, 0.0]],
            Axis::Z => [[c, -s, 0.0, 0.0], [s, c, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]],
        };
        Mat34(m)
    }
}

fn nominal_f64(v: &Value) -> f64 {
    match &v.nominal {
        Nominal::Exact(e) => e.0.to_f64(),
        Nominal::Inexact(f) => f.f64,
    }
}

/// Composes datum frames and placement anchors from the evaluated IR into [`Mat34`]s in the
/// adapter's native unit (geometry.md 2 and 4; the chain is applied left to right).
#[derive(Debug, Clone, Copy)]
pub struct Composer<'a> {
    components: &'a [Component],
    native_scale: f64,
}

impl<'a> Composer<'a> {
    /// A composer over `components`; `native_scale` is the SI length of one native unit.
    // frob:ticket 01M4VTJ6W0X6YHWEEHT645FHJZ
    pub fn new(components: &'a [Component], native_scale: f64) -> Self {
        Self {
            components,
            native_scale,
        }
    }

    fn type_key(p: &Path) -> (String, Vec<String>, String) {
        (p.package.clone(), p.modules.clone(), p.item.clone())
    }

    fn component_of(&self, p: &Path) -> Option<&'a Component> {
        let key = Self::type_key(p);
        self.components
            .iter()
            .find(|c| Self::type_key(&c.path) == key)
    }

    fn is_origin(p: &Path) -> bool {
        matches!(p.members.as_slice(), [Seg::Name(n)] if n == "origin")
    }

    fn member_f64(&self, c: &Component, path: &Path) -> Option<f64> {
        let v = c.members.iter().find(|m| &m.path == path)?.value.as_ref()?;
        Some(nominal_f64(v))
    }

    fn arg(&self, c: &Component, path: &Option<Path>, length: bool) -> Option<f64> {
        match path {
            None => Some(0.0),
            Some(p) => {
                let x = self.member_f64(c, p);
                if x.is_none() {
                    tracing::warn!(path = %p, "op argument has no evaluated value");
                }
                x.map(|x| if length { x / self.native_scale } else { x })
            }
        }
    }

    fn apply_ops(&self, c: &Component, start: Mat34, ops: &[Op]) -> Option<Mat34> {
        let mut m = start;
        for op in ops {
            let step = match op {
                Op::Offset(o) => {
                    let mut t = IDENTITY;
                    t.0[0][3] = self.arg(c, &o.x, true)?;
                    t.0[1][3] = self.arg(c, &o.y, true)?;
                    t.0[2][3] = self.arg(c, &o.z, true)?;
                    t
                }
                Op::Rotate { axis, angle } => Mat34::rotation(
                    *axis,
                    self.member_f64(c, angle).or_else(|| {
                        tracing::warn!(path = %angle, "rotate angle has no evaluated value");
                        None
                    })?,
                ),
                Op::Flip { axis } => Mat34::rotation(*axis, std::f64::consts::PI),
            };
            m = m.compose(&step);
        }
        Some(m)
    }

    /// The frame of `datum` in its own component's coordinates; the origin is the identity.
    // frob:ticket 01M4VTJ6W0X6YHWEEHT645FHJZ
    pub fn datum_frame(&self, datum: &Path) -> Option<Mat34> {
        self.datum_frame_at(datum, 0)
    }

    fn datum_frame_at(&self, datum: &Path, depth: usize) -> Option<Mat34> {
        if Self::is_origin(datum) {
            return Some(IDENTITY);
        }
        if depth > MAX_DEPTH {
            tracing::warn!(path = %datum, "datum chain too deep");
            return None;
        }
        let c = self.component_of(datum)?;
        let Some(d) = c.datums.iter().find(|d| &d.path == datum) else {
            tracing::warn!(path = %datum, "datum not found in the evaluated IR");
            return None;
        };
        let parent = self.datum_frame_at(&d.parent, depth + 1)?;
        self.apply_ops(c, parent, &d.ops)
    }

    /// The composed frame a placement drives its placed datum to: the frame of `at`
    /// (in the enclosing component) followed by the placement ops.
    // frob:ticket 01M4VTJ6W0X6YHWEEHT645FHJZ
    pub fn anchor_frame(&self, owner: &Component, pl: &Placement) -> Option<Mat34> {
        self.anchor_frame_at(owner, pl, 0)
    }

    fn anchor_frame_at(&self, owner: &Component, pl: &Placement, depth: usize) -> Option<Mat34> {
        if depth > MAX_DEPTH {
            tracing::warn!(path = %pl.path, "placement chain too deep");
            return None;
        }
        let at = if Self::type_key(&pl.at) == Self::type_key(&owner.path) {
            self.datum_frame(&pl.at)?
        } else {
            // `at` is a datum of another part: that part's node frame times the datum frame.
            let node = self.node_frame(owner, pl, depth)?;
            node.compose(&self.datum_frame(&pl.at)?)
        };
        self.apply_ops(owner, at, &pl.ops)
    }

    /// The frame of the part node a placement's `at` refers to (`placement of the target part`).
    fn node_frame(&self, owner: &Component, pl: &Placement, depth: usize) -> Option<Mat34> {
        let want = Self::type_key(&pl.at);
        let index = pl.path.steps.last().and_then(|s| s.index.clone());
        let mut cands: Vec<&Placement> = owner
            .placements
            .iter()
            .filter(|q| q.path != pl.path && Self::type_key(&q.datum) == want)
            .collect();
        if cands.len() > 1 && index.is_some() {
            let same: Vec<&Placement> = cands
                .iter()
                .copied()
                .filter(|q| q.path.steps.last().and_then(|s| s.index.clone()) == index)
                .collect();
            if !same.is_empty() {
                cands = same;
            }
        }
        let [q] = cands.as_slice() else {
            tracing::warn!(
                path = %pl.path,
                candidates = cands.len(),
                "cannot resolve the anchor node of a placement uniquely"
            );
            return None;
        };
        let anchor = self.anchor_frame_at(owner, q, depth + 1)?;
        Some(anchor.compose(&self.datum_frame(&q.datum)?.inverse()))
    }
}

/// Verifies every bound frame and instance of `adapter` against the composed design.
///
/// A `frame` locator is compared as a datum frame (`verify_datum`); an `instance` locator as a
/// placement (`verify_placement`), the part's datum read from its bound frame when the adapter
/// reported one, else composed. Unmatched or unresolvable items are skipped with a warning.
// frob:ticket 01M4VTJ6W0X6YHWEEHT645FHJZ
pub fn verify_geometry(
    project: &Project,
    adapter: &str,
    native_scale: f64,
    precision: u32,
) -> Vec<Diag> {
    let comp = Composer::new(&project.components, native_scale);
    let mut out = Vec::new();
    let frame_of = |path: &Path| -> Option<Mat34> {
        let b = project
            .bindings
            .iter()
            .find(|b| b.adapter == adapter && &b.path == path)?;
        b.locators
            .iter()
            .filter(|l| l.kind == LocatorKind::Frame)
            .find_map(|l| {
                let f = project.frames.iter().find(|f| f.name == l.locator)?;
                Mat34::from_observed(&f.transform, native_scale).ok()
            })
    };
    for b in project.bindings.iter().filter(|b| b.adapter == adapter) {
        let path = b.path.to_string();
        for l in &b.locators {
            match l.kind {
                LocatorKind::Frame => {
                    let Some(read) = frame_of(&b.path) else {
                        tracing::debug!(%path, "no observed frame for datum");
                        continue;
                    };
                    let Some(composed) = comp.datum_frame(&b.path) else {
                        continue;
                    };
                    out.extend(verify_datum(&path, &composed, Some(&read), precision));
                }
                LocatorKind::Instance => {
                    let found = project.components.iter().find_map(|c| {
                        c.placements
                            .iter()
                            .find(|p| p.path == b.path)
                            .map(|p| (c, p))
                    });
                    let Some((owner, pl)) = found else {
                        tracing::warn!(%path, "instance bound to no placement");
                        continue;
                    };
                    let Some(obs) = project.instances.iter().find(|i| i.component == l.locator)
                    else {
                        tracing::debug!(%path, "no observed instance for placement");
                        continue;
                    };
                    let Ok(inst) = Mat34::from_observed(&obs.transform, native_scale) else {
                        tracing::warn!(%path, "observed instance transform is malformed");
                        continue;
                    };
                    let Some(anchor) = comp.anchor_frame(owner, pl) else {
                        continue;
                    };
                    let Some(part_datum) =
                        frame_of(&pl.datum).or_else(|| comp.datum_frame(&pl.datum))
                    else {
                        continue;
                    };
                    out.extend(verify_placement(
                        &path,
                        &inst,
                        &part_datum,
                        &anchor,
                        precision,
                    ));
                }
                _ => {}
            }
        }
    }
    out
}

/// Sync entries for every datum of `file` under `datums = "create"` (adapters.md 3.4).
///
/// Returns nothing when the policy does not drive (`datum_action` is `Verify`). The frame name
/// is the bound `frame` locator, else the datum's mangled path; `base` supplies the lock value.
// frob:ticket 01M4VTJ6W0X6YHWEEHT645FHJZ
#[allow(clippy::too_many_arguments)]
pub fn datum_entries(
    project: &Project,
    adapter: &str,
    file: &str,
    policy: DatumPolicy,
    features: &[Feature],
    native_scale: f64,
    precision: u32,
    base: &dyn Fn(&str, &str) -> Option<String>,
) -> Vec<Entry> {
    if datum_action(policy, features) != DatumAction::Drive {
        return Vec::new();
    }
    let comp = Composer::new(&project.components, native_scale);
    let mut out = Vec::new();
    for c in &project.components {
        for d in &c.datums {
            let Some(composed) = comp.datum_frame(&d.path) else {
                continue;
            };
            let bound = project
                .bindings
                .iter()
                .filter(|b| b.adapter == adapter && b.path == d.path)
                .flat_map(|b| b.locators.iter())
                .find(|l| l.kind == LocatorKind::Frame)
                .map(|l| l.locator.clone());
            let frame = bound.unwrap_or_else(|| mangle(&d.path, ""));
            let read = project
                .frames
                .iter()
                .find(|f| f.name == frame && f.document == file)
                .and_then(|f| Mat34::from_observed(&f.transform, native_scale).ok());
            let path = d.path.to_string();
            let last = base(&path, &frame);
            let site = DatumSite {
                adapter,
                file,
                path: &path,
                frame: &frame,
                base: last.as_deref(),
            };
            out.push(datum_entry(site, &composed, read.as_ref(), precision));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atomic::Document;
    use crate::classify::Class;
    use crate::sync::{Backend, Lock, Options, PushItem, ReadBack, Verb, run};

    fn translate(x: f64, y: f64, z: f64) -> Mat34 {
        Mat34([[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]])
    }

    // R001 text shows both transforms.
    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    // frob:tests mismatch_is_r001_showing_both_transforms
    #[test]
    fn mismatch_is_r001_showing_both_transforms() {
        let d = verify_datum(
            "Rail.site",
            &translate(10.0, 0.0, 0.0),
            Some(&translate(10.1, 0.0, 0.0)),
            3,
        )
        .unwrap();
        assert_eq!(d.code, R001);
        assert!(d.message.starts_with("Rail.site:"));
        assert!(d.message.contains("10.000") && d.message.contains("10.100"));
        assert!(verify_datum("Rail.site", &translate(1.0, 0.0, 0.0), None, 3).is_none());
    }

    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    // frob:tests half_unit_of_precision_boundary
    #[test]
    fn half_unit_of_precision_boundary() {
        let d = translate(1.0, 0.0, 0.0);
        assert!(d.matches(&translate(1.0005, 0.0, 0.0), 3));
        assert!(d.matches(&translate(0.9995, 0.0, 0.0), 3));
        assert!(!d.matches(&translate(1.0006, 0.0, 0.0), 3));
        // rotation entries are plain numbers, same rule
        let mut r = d;
        r.0[0][0] = 1.0006;
        assert!(!d.matches(&r, 3));
    }

    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    // frob:tests placement_is_instance_times_part_datum
    #[test]
    fn placement_is_instance_times_part_datum() {
        let inst = translate(100.0, 0.0, 0.0);
        let datum = translate(0.0, 5.0, 0.0);
        let anchor = translate(100.0, 5.0, 0.0);
        assert!(verify_placement("Mount/bracket.place", &inst, &datum, &anchor, 3).is_none());
        let d = verify_placement(
            "Mount/bracket.place",
            &inst,
            &datum,
            &translate(100.0, 6.0, 0.0),
            3,
        )
        .unwrap();
        assert_eq!(d.code, R001);
        assert!(d.message.contains("placement"));
        // a rotated instance carries the datum offset with it
        let rot = Mat34([
            [0.0, -1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ]);
        let got = rot.compose(&translate(2.0, 0.0, 0.0));
        assert_eq!(got.0[1][3], 2.0);
        assert_eq!(got.0[0][3], 0.0);
    }

    fn site(base: Option<&str>) -> DatumSite<'_> {
        DatumSite {
            adapter: "sw",
            file: "rail.sldprt",
            path: "Rail.site",
            frame: "Rail__site",
            base,
        }
    }

    #[derive(Default)]
    struct Fake {
        pushed: Vec<PushItem>,
    }

    struct FakeDoc<'a>(&'a mut Fake);

    impl Document for FakeDoc<'_> {
        fn read_current(&mut self, items: &[PushItem]) -> Vec<ReadBack> {
            items
                .iter()
                .map(|i| ReadBack {
                    locator: i.locator.clone(),
                    value: self
                        .0
                        .pushed
                        .iter()
                        .rev()
                        .find(|p| p.locator == i.locator)
                        .map(|p| p.value.clone()),
                    tolerance: None,
                })
                .collect()
        }
        fn write(&mut self, items: &[PushItem]) -> Result<(), String> {
            self.0.pushed.extend_from_slice(items);
            Ok(())
        }
        fn rebuild(&mut self) -> Result<(), String> {
            Ok(())
        }
    }

    impl Backend for Fake {
        fn document<'a>(&'a mut self, _: &str, _: &str) -> Box<dyn Document + 'a> {
            Box::new(FakeDoc(self))
        }
    }

    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    // frob:tests create_requests_missing_frame_and_records_lock
    #[test]
    fn create_requests_missing_frame_and_records_lock() {
        let composed = translate(10.0, 20.0, 30.0);
        let e = datum_entry(site(None), &composed, None, 3);
        let lit = composed.literal(3);
        let mut lock = Lock::new();
        let mut fake = Fake::default();
        let r = run(&[e], &Options::default(), &mut fake, &mut lock);
        assert_eq!(r.records[0].class, Class::FirstPush);
        assert_eq!(r.records[0].verb, Verb::Push);
        assert_eq!(fake.pushed[0].locator, "Rail__site");
        let back = Mat34::parse_literal(&fake.pushed[0].value).unwrap();
        assert!(back.matches(&composed, 3));
        assert_eq!(
            lock.get(&("sw".into(), "Rail.site".into(), "Rail__site".into())),
            Some(&composed.literal(3))
        );
        // the next sync reads the created frame back: nothing to push
        let e2 = datum_entry(
            site(Some(&lit)),
            &composed,
            Some(&translate(10.0004, 20.0, 30.0)),
            3,
        );
        let r = run(&[e2], &Options::default(), &mut fake, &mut lock);
        assert_eq!(r.records[0].verb, Verb::Skip);
        assert_eq!(fake.pushed.len(), 1);
    }

    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    // frob:tests frame_moved_in_cad_is_r001_under_create
    #[test]
    fn frame_moved_in_cad_is_r001_under_create() {
        let composed = translate(10.0, 0.0, 0.0);
        let lit = composed.literal(3);
        let e = datum_entry(
            site(Some(&lit)),
            &composed,
            Some(&translate(12.0, 0.0, 0.0)),
            3,
        );
        let mut lock = Lock::new();
        let r = run(&[e], &Options::default(), &mut Fake::default(), &mut lock);
        assert!(r.has_r001());
    }

    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    // frob:tests flip_and_exact_quarter_turns_never_pushed
    #[test]
    fn flip_and_exact_quarter_turns_never_pushed() {
        assert!(!rotation_pushable(RotationOp::Flip));
        for d in [90.0, 180.0, -90.0, 270.0, 0.0] {
            assert!(!rotation_pushable(RotationOp::Rotate { degrees: d }), "{d}");
        }
        assert!(rotation_pushable(RotationOp::Rotate { degrees: 45.0 }));
        assert!(!kind_pushable(LocatorKind::Instance));
        assert!(kind_pushable(LocatorKind::Frame));
    }

    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    // frob:tests create_needs_the_create_frames_feature
    #[test]
    fn create_needs_the_create_frames_feature() {
        use DatumAction::*;
        assert_eq!(
            datum_action(DatumPolicy::Verify, &[Feature::CreateFrames]),
            Verify
        );
        assert_eq!(
            datum_action(DatumPolicy::Create, &[Feature::Frames]),
            Verify
        );
        assert_eq!(
            datum_action(DatumPolicy::Create, &[Feature::CreateFrames]),
            Drive
        );
    }

    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    // frob:tests literal_round_trips_and_normalizes_negative_zero
    #[test]
    fn literal_round_trips_and_normalizes_negative_zero() {
        let m = Mat34([
            [1.0, -0.0, 0.0, -0.0001],
            [0.0, 1.0, 0.0, 2.5],
            [0.0, 0.0, 1.0, 0.0],
        ]);
        let l = m.literal(3);
        assert!(!l.contains("-0.000"), "{l}");
        assert!(Mat34::parse_literal(&l).unwrap().matches(&m, 3));
        assert_eq!(Mat34::parse_literal("[[1]]"), Err(MatError::Shape));
    }

    // --- glue: IR -> Mat34 -> verify / create (ticket 645FHJZ) ---

    use cadetta_ir::geometry::{Datum, OffsetArgs};
    use cadetta_ir::model::{
        Binding, ComponentClass, Flavour, Locator, LocatorMode, Member, MemberKind, ObservedFrame,
        ObservedInstance, PackageId,
    };
    use cadetta_ir::path::PathContext;
    use cadetta_units::{Dim, Magnitude, Rational};

    fn pth(t: &str) -> Path {
        Path::parse(t, PathContext::General).unwrap()
    }

    fn metres(path: &str, milli: i64) -> Member {
        let mut m = Member::new(pth(path), MemberKind::Datum);
        m.value = Some(Value::new(
            Nominal::Exact(cadetta_ir::model::Exact(Magnitude::new(
                Rational::new(milli, 1000).unwrap(),
                0,
            ))),
            Dim::default(),
            "x",
        ));
        m
    }

    fn off_x(path: &str) -> Op {
        Op::Offset(Box::new(OffsetArgs {
            x: Some(pth(path)),
            ..Default::default()
        }))
    }

    fn component(path: &str) -> Component {
        let mut cpath = pth(&format!("{path}.x"));
        cpath.members.clear();
        Component {
            path: cpath,
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members: vec![],
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        }
    }

    fn mm_rows(x: i64) -> Vec<Vec<Value>> {
        let v = |n: i64| {
            Value::new(
                Nominal::Exact(cadetta_ir::model::Exact(Magnitude::new(
                    Rational::new(n, 1000).unwrap(),
                    0,
                ))),
                Dim::default(),
                "x",
            )
        };
        let one = Value::new(
            Nominal::Exact(cadetta_ir::model::Exact(Magnitude::new(
                Rational::new(1, 1).unwrap(),
                0,
            ))),
            Dim::default(),
            "1",
        );
        let zero = v(0);
        vec![
            vec![one.clone(), zero.clone(), zero.clone(), v(x)],
            vec![zero.clone(), one.clone(), zero.clone(), zero.clone()],
            vec![zero.clone(), zero, one, v(0)],
        ]
    }

    fn locator(name: &str, kind: LocatorKind) -> Locator {
        Locator {
            locator: name.into(),
            kind,
            mode: LocatorMode::Value,
            native_unit: "mm".into(),
            ref_to: None,
            for_alternate: None,
            config: None,
            sign: None,
            component: None,
            implicit: None,
            tolerance: None,
        }
    }

    fn project(frame_x: i64, inst_x: i64) -> Project {
        let mut pr = Project::new(
            Flavour::Design,
            PackageId {
                name: "a.b".into(),
                version: "1".into(),
            },
        );
        let mut rail = component("a.b::Rail");
        rail.members.push(metres("a.b::Rail.site.offset.x", 10));
        rail.datums.push(Datum::new(
            pth("a.b::Rail.site"),
            pth("a.b::Rail.origin"),
            vec![off_x("a.b::Rail.site.offset.x")],
        ));
        let mut asm = component("a.b::Asm");
        asm.members.push(metres("a.b::Asm/rail.place.0.x", 100));
        asm.placements.push(Placement {
            path: pth("a.b::Asm/rail.place"),
            part: pth("a.b::Asm/rail.part"),
            datum: pth("a.b::Rail.site"),
            at: pth("a.b::Asm.origin"),
            ops: vec![off_x("a.b::Asm/rail.place.0.x")],
            variants: None,
            digest: None,
            closure: None,
        });
        pr.components = vec![rail, asm];
        pr.bindings = vec![
            Binding {
                path: pth("a.b::Rail.site"),
                adapter: "fc".into(),
                locators: vec![locator("Rail__site", LocatorKind::Frame)],
            },
            Binding {
                path: pth("a.b::Asm/rail.place"),
                adapter: "fc".into(),
                locators: vec![locator("Rail_1", LocatorKind::Instance)],
            },
        ];
        pr.frames = vec![ObservedFrame {
            name: "Rail__site".into(),
            document: "rail.FCStd".into(),
            transform: mm_rows(frame_x),
        }];
        pr.instances = vec![ObservedInstance {
            component: "Rail_1".into(),
            document: "asm.FCStd".into(),
            reference: None,
            transform: mm_rows(inst_x),
        }];
        pr
    }

    // Composed datum frames are converted to native units and compared; mismatch is R001.
    // frob:ticket 01M4VTJ6W0X6YHWEEHT645FHJZ
    // frob:tests composed_frames_and_instances_verify_through_geometry_bind
    #[test]
    fn composed_frames_and_instances_verify_through_geometry_bind() {
        // frame at 10 mm, instance at 90 mm: 90 + 10 == the 100 mm anchor
        assert!(verify_geometry(&project(10, 90), "fc", 0.001, 3).is_empty());
        // a moved frame is R001 on the datum
        let d = verify_geometry(&project(12, 90), "fc", 0.001, 3);
        assert!(
            d.iter()
                .any(|x| x.code == R001 && x.message.starts_with("a.b::Rail.site:"))
        );
        // the placement uses the read frame, so it also disagrees (12 + 90 != 100)
        assert!(d.iter().any(|x| x.message.contains("placement")));
        // a moved instance is R001 on the placement only
        let d = verify_geometry(&project(10, 95), "fc", 0.001, 3);
        assert_eq!(d.len(), 1);
        assert!(d[0].message.starts_with("a.b::Asm/rail.place:"));
        // other adapters see nothing
        assert!(verify_geometry(&project(12, 95), "sw", 0.001, 3).is_empty());
    }

    // datums = create pushes a missing frame as the composed transform, once.
    // frob:ticket 01M4VTJ6W0X6YHWEEHT645FHJZ
    // frob:tests datums_create_pushes_composed_frame_to_the_adapter
    #[test]
    fn datums_create_pushes_composed_frame_to_the_adapter() {
        let mut pr = project(10, 90);
        pr.frames.clear();
        let none = |_: &str, _: &str| None;
        let feats = [Feature::CreateFrames];
        // verify policy (or a missing feature) yields no entries
        assert!(
            datum_entries(
                &pr,
                "fc",
                "rail.FCStd",
                DatumPolicy::Verify,
                &feats,
                0.001,
                3,
                &none
            )
            .is_empty()
        );
        assert!(
            datum_entries(
                &pr,
                "fc",
                "rail.FCStd",
                DatumPolicy::Create,
                &[],
                0.001,
                3,
                &none
            )
            .is_empty()
        );
        let entries = datum_entries(
            &pr,
            "fc",
            "rail.FCStd",
            DatumPolicy::Create,
            &feats,
            0.001,
            3,
            &none,
        );
        assert_eq!(entries.len(), 1);
        let mut lock = Lock::new();
        let mut fake = Fake::default();
        let r = run(&entries, &Options::default(), &mut fake, &mut lock);
        assert_eq!(r.records[0].verb, Verb::Push);
        assert_eq!(fake.pushed[0].locator, "Rail__site");
        let pushed = Mat34::parse_literal(&fake.pushed[0].value).unwrap();
        assert!(pushed.matches(&translate(10.0, 0.0, 0.0), 3));
    }

    // A rigid transform times its inverse is the identity; a half turn flips the axes.
    // frob:ticket 01M4VTJ6W0X6YHWEEHT645FHJZ
    // frob:tests inverse_and_rotation_are_rigid
    #[test]
    fn inverse_and_rotation_are_rigid() {
        let m = Mat34::rotation(Axis::Z, 0.7).compose(&translate(1.0, 2.0, 3.0));
        assert!(m.compose(&m.inverse()).matches(&IDENTITY, 9));
        let flip = Mat34::rotation(Axis::X, std::f64::consts::PI);
        assert!((flip.0[1][1] + 1.0).abs() < 1e-12 && (flip.0[2][2] + 1.0).abs() < 1e-12);
    }
}
