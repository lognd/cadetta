//! Geometry verification and frame driving (adapters.md 3.4): compare a datum's frame or a
//! placement's instance transform with the composed design transform, and plan the
//! `datums = "create"` push of a composed datum into a coordinate system.
//!
//! The transforms here are plain numeric 3x4 matrices in the adapter's native unit: the caller
//! turns `cadetta_geometry`'s composed `Transform` (and the observed `frames[]`/`instances[]`
//! values) into [`Mat34`] and this module only compares, so no transform is composed here
//! beyond the one instance-times-datum product the spec names.
// frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN

use cadetta_ir::model::{LocatorKind, Nominal, Value};
use serde::Deserialize;

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

#[cfg(test)]
mod tests {
    use super::*;
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

    impl Backend for Fake {
        fn push(&mut self, _: &str, _: &str, items: &[PushItem], _: bool) -> Vec<ReadBack> {
            self.pushed.extend_from_slice(items);
            items
                .iter()
                .map(|i| ReadBack {
                    locator: i.locator.clone(),
                    value: Some(i.value.clone()),
                    tolerance: None,
                })
                .collect()
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
}
