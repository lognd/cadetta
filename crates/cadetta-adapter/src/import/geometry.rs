//! Import of geometry: observed frames become datums, observed instances become places
//! (adapters.md 7.2).
//!
//! A read 3x4 transform is written as the source expression `origin.offset(...).rotate(...)`:
//! one `offset` and the fewest `rotate`s. Rotations by whole multiples of 90deg are recognised and
//! written exactly (at most two turns); any other rotation is written as the three elementary
//! turns it decomposes into (z, then y, then x, each in the frame built so far) and flagged.
// frob:ticket 01M4VB64CJDA2CZ4WM4MNV70G6

use cadetta_ir::geometry::Axis;

use crate::geometry_bind::Mat34;

/// Entries closer than this to a signed-permutation value count as exact (read-back noise).
const EPS: f64 = 1e-6;

/// A transform written as source operations.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    /// `origin.offset(..).rotate(..)`, the right-hand side of a datum or the anchor of a place.
    pub expr: String,
    /// The rotation is not a whole multiple of 90deg, so `expr` carries it as read.
    pub inexact: bool,
}

/// Why a transform cannot be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteError {
    /// The rotation block is not orthonormal with determinant +1.
    NotRigid,
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRigid => write!(f, "transform is not a proper rotation"),
        }
    }
}

impl std::error::Error for WriteError {}

/// The SI length of one native unit, for the units an adapter reports (`None` when unknown).
pub fn unit_scale(unit: &str) -> Option<f64> {
    Some(match unit {
        "mm" => 0.001,
        "cm" => 0.01,
        "m" => 1.0,
        "um" => 1e-6,
        "in" => 0.0254,
        "ft" => 0.3048,
        _ => return None,
    })
}

fn axis_name(a: Axis) -> &'static str {
    match a {
        Axis::X => "x",
        Axis::Y => "y",
        Axis::Z => "z",
    }
}

/// A number with at most six decimals, no trailing zeros and no negative zero.
fn num(x: f64) -> String {
    let s = format!("{x:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

fn rot(m: &Mat34) -> [[f64; 3]; 3] {
    let a = &m.0;
    [
        [a[0][0], a[0][1], a[0][2]],
        [a[1][0], a[1][1], a[1][2]],
        [a[2][0], a[2][1], a[2][2]],
    ]
}

fn is_rigid(r: &[[f64; 3]; 3]) -> bool {
    for i in 0..3 {
        for j in 0..3 {
            let dot: f64 = (0..3).map(|k| r[k][i] * r[k][j]).sum();
            if (dot - f64::from(u8::from(i == j))).abs() > EPS {
                return false;
            }
        }
    }
    let det = r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
        - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
        + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0]);
    (det - 1.0).abs() <= EPS
}

fn is_quarter_turns(r: &[[f64; 3]; 3]) -> bool {
    r.iter()
        .flatten()
        .all(|x| [0.0, 1.0, -1.0].iter().any(|t| (x - t).abs() <= EPS))
}

fn same_rotation(a: &Mat34, b: &Mat34) -> bool {
    let (a, b) = (rot(a), rot(b));
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .all(|(x, y)| (x - y).abs() <= EPS)
}

/// The fewest single-axis turns (each 90, 180 or 270deg) whose product is `target`.
///
/// Every proper rotation of the cube needs at most two, so the search is short and its order
/// (x, y, z; 90, 180, 270) makes the answer deterministic.
fn fewest_turns(target: &Mat34) -> Vec<(Axis, u32)> {
    let turn = |a: Axis, d: u32| Mat34::rotation(a, f64::from(d).to_radians());
    let steps: Vec<(Axis, u32)> = [Axis::X, Axis::Y, Axis::Z]
        .into_iter()
        .flat_map(|a| [90, 180, 270].map(|d| (a, d)))
        .collect();
    let identity = Mat34::rotation(Axis::X, 0.0);
    if same_rotation(&identity, target) {
        return Vec::new();
    }
    for &(a, d) in &steps {
        if same_rotation(&turn(a, d), target) {
            return vec![(a, d)];
        }
    }
    for &(a, d) in &steps {
        for &(b, e) in &steps {
            if same_rotation(&turn(a, d).compose(&turn(b, e)), target) {
                return vec![(a, d), (b, e)];
            }
        }
    }
    unreachable!("every signed-permutation rotation is at most two quarter-turn steps")
}

/// ZYX elementary angles in degrees: `R = Rz(z) * Ry(y) * Rx(x)`, i.e. rotate z, then y, then x.
fn euler_zyx(r: &[[f64; 3]; 3]) -> [(Axis, f64); 3] {
    let y = (-r[2][0]).clamp(-1.0, 1.0).asin();
    let (z, x) = if r[2][0].abs() > 1.0 - EPS {
        ((-r[0][1]).atan2(r[1][1]), 0.0)
    } else {
        (r[1][0].atan2(r[0][0]), r[2][1].atan2(r[2][2]))
    };
    [
        (Axis::Z, z.to_degrees()),
        (Axis::Y, y.to_degrees()),
        (Axis::X, x.to_degrees()),
    ]
}

/// Writes `m` (translation in `unit`, rotation as read) as one `offset` then the fewest `rotate`s.
///
/// Zero offset components and zero rotations are left out; the identity is just `origin`.
pub fn write_transform(m: &Mat34, unit: &str) -> Result<Written, WriteError> {
    let r = rot(m);
    if !is_rigid(&r) {
        tracing::warn!("import: transform is not a proper rotation");
        return Err(WriteError::NotRigid);
    }
    let mut expr = String::from("origin");
    let offsets: Vec<String> = ["x", "y", "z"]
        .iter()
        .zip(m.0.iter())
        .filter(|(_, row)| num(row[3]) != "0")
        .map(|(a, row)| format!("{a} = {}{unit}", num(row[3])))
        .collect();
    if !offsets.is_empty() {
        expr.push_str(&format!(".offset({})", offsets.join(", ")));
    }
    let (angles, inexact) = if is_quarter_turns(&r) {
        let turns = fewest_turns(m)
            .into_iter()
            .map(|(a, d)| (a, f64::from(d)))
            .collect();
        (turns, false)
    } else {
        let all = euler_zyx(&r);
        let kept: Vec<(Axis, f64)> = all.into_iter().filter(|(_, d)| num(*d) != "0").collect();
        (kept, true)
    };
    for (a, d) in angles {
        expr.push_str(&format!(".rotate({} = {}deg)", axis_name(a), num(d)));
    }
    tracing::debug!(%expr, inexact, "import: transform written");
    Ok(Written { expr, inexact })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(rot: Mat34, t: [f64; 3]) -> Mat34 {
        let mut m = rot;
        for (i, v) in t.iter().enumerate() {
            m.0[i][3] = *v;
        }
        m
    }

    fn id() -> Mat34 {
        Mat34::rotation(Axis::X, 0.0)
    }

    // frob:tests identity_is_origin
    #[test]
    fn identity_is_origin() {
        let w = write_transform(&id(), "mm").unwrap();
        assert_eq!(w.expr, "origin");
        assert!(!w.inexact);
    }

    // frob:tests offset_is_one_op_and_zero_axes_dropped
    #[test]
    fn offset_is_one_op_and_zero_axes_dropped() {
        let w = write_transform(&at(id(), [12.5, 0.0, -3.0]), "mm").unwrap();
        assert_eq!(w.expr, "origin.offset(x = 12.5mm, z = -3mm)");
    }

    // frob:tests quarter_turns_are_exact_and_fewest
    #[test]
    fn quarter_turns_are_exact_and_fewest() {
        let rz = Mat34::rotation(Axis::Z, 90f64.to_radians());
        let w = write_transform(&at(rz, [1.0, 2.0, 3.0]), "mm").unwrap();
        assert_eq!(
            w.expr,
            "origin.offset(x = 1mm, y = 2mm, z = 3mm).rotate(z = 90deg)"
        );
        assert!(!w.inexact);
        let half = Mat34::rotation(Axis::Y, std::f64::consts::PI);
        assert_eq!(
            write_transform(&half, "mm").unwrap().expr,
            "origin.rotate(y = 180deg)"
        );
        // every one of the 24 cube rotations takes at most two turns and round-trips
        let mut seen = 0;
        for a in [Axis::X, Axis::Y, Axis::Z] {
            for b in [Axis::X, Axis::Y, Axis::Z] {
                for (d, e) in [(90.0, 90.0), (90.0, 180.0), (270.0, 90.0), (180.0, 270.0)] {
                    let m = Mat34::rotation(a, f64::to_radians(d))
                        .compose(&Mat34::rotation(b, f64::to_radians(e)));
                    let turns = fewest_turns(&m);
                    assert!(turns.len() <= 2);
                    let back = turns.iter().fold(id(), |acc, (ax, dg)| {
                        acc.compose(&Mat34::rotation(*ax, f64::from(*dg).to_radians()))
                    });
                    assert!(same_rotation(&back, &m));
                    seen += 1;
                }
            }
        }
        assert_eq!(seen, 36);
    }

    // frob:tests other_rotations_are_flagged_and_round_trip
    #[test]
    fn other_rotations_are_flagged_and_round_trip() {
        let m = Mat34::rotation(Axis::Z, 30f64.to_radians())
            .compose(&Mat34::rotation(Axis::X, 10f64.to_radians()));
        let w = write_transform(&m, "mm").unwrap();
        assert!(w.inexact);
        assert_eq!(w.expr, "origin.rotate(z = 30deg).rotate(x = 10deg)");
    }

    // frob:tests non_rigid_is_refused
    #[test]
    fn non_rigid_is_refused() {
        let mut m = id();
        m.0[0][0] = 2.0;
        assert_eq!(write_transform(&m, "mm"), Err(WriteError::NotRigid));
    }

    // frob:tests unit_scale_known_units
    #[test]
    fn unit_scale_known_units() {
        assert_eq!(unit_scale("in"), Some(0.0254));
        assert_eq!(unit_scale("furlong"), None);
    }
}
