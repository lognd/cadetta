//! Emit policy: which tolerance point is written, and Q004 precision
//! (adapters.md 4 rules 1 and 3; quantities.md 5.6).
//!
//! Policy is decided upstream: an adapter receives an [`EmitRequest`] whose
//! value, unit and point are already fixed and only formats it. A value that
//! does not fit the declared precision is Q004 with the error, never rounded.

use std::fmt;

/// Diagnostic code for a value not representable at the declared precision.
pub const Q004: &str = "Q004";
/// Diagnostic code for `mmc`/`lmc` on a parameter with no role.
pub const E010: &str = "E010";

/// An exact rational with a positive denominator, in lowest terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    num: i128,
    den: i128,
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Ratio {
    /// Builds `num/den` in lowest terms; `den` must be nonzero.
    pub fn new(num: i128, den: i128) -> Self {
        assert!(den != 0, "zero denominator");
        let g = gcd(num, den).max(1);
        let s = if den < 0 { -1 } else { 1 };
        Self {
            num: s * num / g,
            den: s * den / g,
        }
    }

    /// Exact midpoint of two ratios.
    pub fn midpoint(self, other: Self) -> Self {
        Self::new(
            self.num * other.den + other.num * self.den,
            2 * self.den * other.den,
        )
    }
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

/// Which point of a toleranced value is written to the tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmitPoint {
    /// The designed nominal (default).
    Nominal,
    /// Midpoint of the limits.
    Mid,
    /// Maximum material condition.
    Mmc,
    /// Least material condition.
    Lmc,
}

/// Role of a parameter (quantities.md 5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Internal feature: material is at its minimum limit at MMC.
    Hole,
    /// External feature: material is at its maximum limit at MMC.
    Shaft,
}

/// The designed value of a parameter: nominal and its limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Designed {
    /// `Value.nominal`.
    pub nominal: Ratio,
    /// Lower limit.
    pub min: Ratio,
    /// Upper limit.
    pub max: Ratio,
}

/// A policy failure carrying its diagnostic code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    /// `mmc`/`lmc` requested on a parameter with no role (E010).
    NoRole,
    /// Value not representable at the declared precision (Q004).
    Inexact {
        /// The exact value that was withheld.
        value: Ratio,
        /// Declared decimal places.
        precision: u32,
        /// Exact difference between the value and its nearest representable one.
        error: Ratio,
    },
}

impl PolicyError {
    /// The diagnostic code of this error.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoRole => E010,
            Self::Inexact { .. } => Q004,
        }
    }
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRole => write!(f, "{E010}: mmc/lmc needs a role (hole or shaft)"),
            Self::Inexact {
                value,
                precision,
                error,
            } => write!(
                f,
                "{Q004}: {value} is not representable at {precision} decimals (error {error})"
            ),
        }
    }
}

impl std::error::Error for PolicyError {}

/// Chooses the value to emit from the designed value, point and role.
pub fn select_point(
    d: &Designed,
    point: EmitPoint,
    role: Option<Role>,
) -> Result<Ratio, PolicyError> {
    let v = match (point, role) {
        (EmitPoint::Nominal, _) => d.nominal,
        (EmitPoint::Mid, _) => d.min.midpoint(d.max),
        (EmitPoint::Mmc, Some(Role::Hole)) | (EmitPoint::Lmc, Some(Role::Shaft)) => d.min,
        (EmitPoint::Mmc, Some(Role::Shaft)) | (EmitPoint::Lmc, Some(Role::Hole)) => d.max,
        (EmitPoint::Mmc | EmitPoint::Lmc, None) => {
            tracing::error!(?point, "mmc/lmc without a role (E010)");
            return Err(PolicyError::NoRole);
        }
    };
    tracing::debug!(?point, ?role, value = %v, "emit point selected");
    Ok(v)
}

/// Resolves a parameter's point: own attribute, else the referent's, else the default.
///
/// `chain` lists the pure-reference chain from the parameter to its root referent.
pub fn resolve_point(chain: &[Option<EmitPoint>], default: EmitPoint) -> EmitPoint {
    let p = chain.iter().copied().flatten().next().unwrap_or(default);
    tracing::debug!(?p, depth = chain.len(), "emit point resolved");
    p
}

/// Formats `v` as a decimal with at most `precision` places, trailing zeros trimmed.
///
/// Returns Q004 with the exact error when `v` is not representable.
pub fn to_decimal(v: Ratio, precision: u32) -> Result<String, PolicyError> {
    let scale = 10i128.pow(precision);
    let scaled_num = v.num * scale;
    if scaled_num % v.den != 0 {
        // Nearest representable value, only to report the exact error.
        let q = (2 * scaled_num + v.den).div_euclid(2 * v.den);
        let error = Ratio::new(scaled_num - q * v.den, v.den * scale);
        tracing::warn!(value = %v, precision, "value not representable (Q004)");
        return Err(PolicyError::Inexact {
            value: v,
            precision,
            error,
        });
    }
    let n = scaled_num / v.den;
    let (sign, n) = if n < 0 { ("-", -n) } else { ("", n) };
    let (int, frac) = (n / scale, n % scale);
    if frac == 0 {
        return Ok(format!("{sign}{int}"));
    }
    let frac = format!("{frac:0width$}", width = precision as usize);
    Ok(format!("{sign}{int}.{}", frac.trim_end_matches('0')))
}

/// What an adapter receives: point, value and unit all decided upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitRequest {
    /// Canonical path of the parameter.
    pub path: String,
    /// Point that was selected.
    pub point: EmitPoint,
    /// Exact value at that point.
    pub value: Ratio,
    /// Unit the value is written in.
    pub unit: String,
}

/// Builds a request, deciding the point and value so the adapter decides nothing.
pub fn build_request(
    path: &str,
    d: &Designed,
    point: EmitPoint,
    role: Option<Role>,
    unit: &str,
) -> Result<EmitRequest, PolicyError> {
    let value = select_point(d, point, role)?;
    Ok(EmitRequest {
        path: path.to_string(),
        point,
        value,
        unit: unit.to_string(),
    })
}

/// The part of the sync record that decides whether a path is re-pushed (ir.md 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRecord {
    /// Emit point at the last push.
    pub point: EmitPoint,
    /// Value pushed.
    pub value: Ratio,
}

/// True when the request differs from the sync record, including a changed emit attribute.
pub fn needs_push(record: Option<&SyncRecord>, req: &EmitRequest) -> bool {
    let push = record.is_none_or(|r| r.point != req.point || r.value != req.value);
    tracing::debug!(path = req.path.as_str(), push, "sync record compared");
    push
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i128, d: i128) -> Ratio {
        Ratio::new(n, d)
    }

    fn des() -> Designed {
        Designed {
            nominal: r(10, 1),
            min: r(99, 10),
            max: r(104, 10),
        }
    }

    #[test]
    fn mid_selects_midpoint() {
        let v = select_point(&des(), EmitPoint::Mid, None).unwrap();
        assert_eq!(v, r(203, 20));
    }

    #[test]
    fn inexact_is_q004_with_error() {
        let e = to_decimal(r(1, 3), 6).unwrap_err();
        assert_eq!(e.code(), "Q004");
        match e {
            PolicyError::Inexact { error, .. } => assert_eq!(error, r(1, 3_000_000)),
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(to_decimal(r(-5, 4), 6).unwrap(), "-1.25");
        assert_eq!(to_decimal(r(6, 1), 6).unwrap(), "6");
    }

    // frob:ticket 01M4V7WDK6DPPA5Z9FEEPB7ZY9
    // frob:tests crates/cadetta-adapter/src/policy.rs::to_decimal
    #[test]
    fn whole_numbers_render_without_a_fraction() {
        assert_eq!(to_decimal(r(10, 1), 6).unwrap(), "10");
        assert_eq!(to_decimal(r(-3, 1), 6).unwrap(), "-3");
        assert_eq!(to_decimal(r(0, 1), 6).unwrap(), "0");
        assert_eq!(to_decimal(r(127, 20), 6).unwrap(), "6.35");
    }

    #[test]
    fn mmc_lmc_without_role_is_e010() {
        for p in [EmitPoint::Mmc, EmitPoint::Lmc] {
            let e = select_point(&des(), p, None).unwrap_err();
            assert_eq!(e, PolicyError::NoRole);
            assert_eq!(e.code(), "E010");
        }
        assert_eq!(
            select_point(&des(), EmitPoint::Mmc, Some(Role::Hole)).unwrap(),
            des().min
        );
        assert_eq!(
            select_point(&des(), EmitPoint::Mmc, Some(Role::Shaft)).unwrap(),
            des().max
        );
        assert_eq!(
            select_point(&des(), EmitPoint::Lmc, Some(Role::Hole)).unwrap(),
            des().max
        );
    }

    #[test]
    fn alias_inherits_referent_point() {
        // alias has no attribute; its referent (a fit side) is #[emit(mid)].
        let p = resolve_point(&[None, Some(EmitPoint::Mid)], EmitPoint::Nominal);
        assert_eq!(p, EmitPoint::Mid);
        let own = resolve_point(
            &[Some(EmitPoint::Lmc), Some(EmitPoint::Mid)],
            EmitPoint::Nominal,
        );
        assert_eq!(own, EmitPoint::Lmc);
        assert_eq!(
            resolve_point(&[None], EmitPoint::Nominal),
            EmitPoint::Nominal
        );
    }

    #[test]
    fn request_carries_decided_point_and_unit() {
        let q = build_request("A/b", &des(), EmitPoint::Mid, None, "mm").unwrap();
        assert_eq!(q.point, EmitPoint::Mid);
        assert_eq!(q.unit, "mm");
        assert_eq!(q.value, r(203, 20));
    }

    #[test]
    fn emit_point_follows_policy_and_change_repushes() {
        let nom = build_request("A/b", &des(), EmitPoint::Nominal, None, "mm").unwrap();
        let rec = SyncRecord {
            point: nom.point,
            value: nom.value,
        };
        assert!(!needs_push(Some(&rec), &nom));
        assert!(needs_push(None, &nom));
        let mid = build_request("A/b", &des(), EmitPoint::Mid, None, "mm").unwrap();
        assert!(needs_push(Some(&rec), &mid));
        // Same value, changed attribute: still re-pushed.
        let d = Designed {
            nominal: r(10, 1),
            min: r(9, 1),
            max: r(11, 1),
        };
        let a = build_request("A/b", &d, EmitPoint::Nominal, None, "mm").unwrap();
        let b = build_request("A/b", &d, EmitPoint::Mid, None, "mm").unwrap();
        let rec = SyncRecord {
            point: a.point,
            value: a.value,
        };
        assert!(needs_push(Some(&rec), &b));
    }
}
