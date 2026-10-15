//! Lints O004 (stale sync obligation) and W005 (waiver or `eol` expiring soon).
//!
//! Specification: `docs/spec/lints.md` 2.1, 2.2 and 3 (O, W); `docs/spec/obligations.md` 2.3.

// frob:ticket 01M4SWKAMS533AP2HRCNQG03WD

use cadetta_ir::entities::Waiver;
use cadetta_obligations::graph::{ObligationId, ObligationKind, ObligationState};
use cadetta_obligations::kinds::{KindPolicy, RequiredFor};
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Severity};
use crate::engine::{LintConfig, waiver_id};

/// Rule code: the "CAD file reflects design" obligation is stale (info; gated by kind policy).
pub const O004: &str = "O004";
/// Rule code: a waiver or `eol` date is within `expiry_warn_days` of `--as-of` (info).
pub const W005: &str = "W005";

/// The default `[lint] expiry_warn_days`.
pub const DEFAULT_EXPIRY_WARN_DAYS: u32 = 30;

/// Run O004 over classified obligations; a `sync` kind with `required_for` gates as an error.
///
/// `release` is true for `check --release`, which also gates `required_for = "release"`.
pub fn check_o004(
    obligations: &[(ObligationId, ObligationState)],
    kinds: &KindPolicy,
    cfg: &LintConfig,
    release: bool,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (id, state) in obligations {
        if id.kind() != ObligationKind::Sync || *state != ObligationState::Stale {
            trace!(%id, ?state, "O004: not applicable");
            continue;
        }
        let gating = match kinds.required_for(ObligationKind::Sync) {
            Some(RequiredFor::Check) => true,
            Some(RequiredFor::Release) => release,
            None => false,
        };
        let severity = if gating {
            Severity::Error
        } else {
            cfg.severity(O004).expect("O004 is in the catalog")
        };
        debug!(%id, gating, ?severity, "O004: sync stale");
        let mut d = Diagnostic::new(
            O004,
            severity,
            format!("CAD file no longer reflects the design: `{id}` is stale; run `cadetta sync`"),
        );
        d.path = Some(id.path().to_owned());
        out.push(d);
    }
    out
}

/// The last day of month `m` in year `y`.
fn last_day(y: i64, m: i64) -> i64 {
    let leap = y % 400 == 0 || (y % 4 == 0 && y % 100 != 0);
    match m {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 for an ISO `YYYY-MM-DD` date, or `None` if malformed.
///
/// A month-only `YYYY-MM` date (a lifecycle date) counts as the last day of that month.
fn day_number(s: &str) -> Option<i64> {
    let mut it = s.split('-');
    let (y, m, d) = (it.next()?, it.next()?, it.next());
    if it.next().is_some() || y.len() != 4 || m.len() != 2 || d.is_some_and(|d| d.len() != 2) {
        return None;
    }
    let (y, m): (i64, i64) = (y.parse().ok()?, m.parse().ok()?);
    if !(1..=12).contains(&m) {
        return None;
    }
    let d: i64 = match d {
        Some(d) => d.parse().ok()?,
        None => last_day(y, m),
    };
    if !(1..=31).contains(&d) {
        return None;
    }
    // Days-from-civil (proleptic Gregorian).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// True when `date` is on or after `as_of` and at most `warn_days` later.
fn expiring(date: &str, as_of: &str, warn_days: u32) -> bool {
    match (day_number(date), day_number(as_of)) {
        (Some(d), Some(now)) => d >= now && d - now <= i64::from(warn_days),
        _ => false,
    }
}

/// Run W005 over waivers and `eol` dates; `allow` attributes carry no expiry and are excluded.
///
/// `eols` pairs a label (the entity path) with its `eol` date. Without `as_of` nothing fires.
pub fn check_w005(
    waivers: &[Waiver],
    eols: &[(String, String)],
    as_of: Option<&str>,
    warn_days: u32,
    cfg: &LintConfig,
) -> Vec<Diagnostic> {
    let Some(now) = as_of else {
        trace!("W005: no --as-of, skipped");
        return Vec::new();
    };
    let sev = cfg.severity(W005).expect("W005 is in the catalog");
    let mut out = Vec::new();
    for w in waivers {
        if let Some(exp) = w.expires.as_deref()
            && expiring(exp, now, warn_days)
        {
            let id = waiver_id(w);
            debug!(%id, exp, "W005: waiver expiring");
            out.push(Diagnostic::new(
                W005,
                sev,
                format!("`{id}` expires {exp}, within {warn_days} days of {now}"),
            ));
        }
    }
    for (path, eol) in eols {
        if expiring(eol, now, warn_days) {
            debug!(%path, eol, "W005: eol approaching");
            let mut d = Diagnostic::new(
                W005,
                sev,
                format!("`{path}` reaches end of life {eol}, within {warn_days} days of {now}"),
            );
            d.path = Some(path.clone());
            out.push(d);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> ObligationId {
        ObligationId::parse(s).unwrap()
    }

    fn waiver(expires: Option<&str>) -> Waiver {
        let mut w = Waiver::new("M001", "a::X.*", "because");
        w.expires = expires.map(Into::into);
        w
    }

    fn sync_kinds(rf: &str) -> KindPolicy {
        let mut k = KindPolicy::new();
        k.set("sync", rf).unwrap();
        k
    }

    fn o004(kinds: &KindPolicy, release: bool) -> Vec<Diagnostic> {
        check_o004(
            &[(id("sync:fc/a"), ObligationState::Stale)],
            kinds,
            &LintConfig::new(),
            release,
        )
    }

    // frob:tests crates/cadetta-lint/src/rules/o004_w005.rs::check_o004
    #[test]
    fn o004_is_info_by_default() {
        let d = o004(&KindPolicy::new(), false);
        assert_eq!(d.len(), 1);
        assert_eq!(
            (d[0].code.as_str(), d[0].severity),
            ("O004", Severity::Info)
        );
    }

    // frob:tests crates/cadetta-lint/src/rules/o004_w005.rs::check_o004
    #[test]
    fn o004_gated_by_required_for() {
        assert_eq!(
            o004(&sync_kinds("check"), false)[0].severity,
            Severity::Error
        );
        assert_eq!(
            o004(&sync_kinds("release"), false)[0].severity,
            Severity::Info
        );
        assert_eq!(
            o004(&sync_kinds("release"), true)[0].severity,
            Severity::Error
        );
    }

    // frob:tests crates/cadetta-lint/src/rules/o004_w005.rs::check_o004
    #[test]
    fn o004_ignores_non_stale_and_other_kinds() {
        let obs = [
            (id("sync:fc/a"), ObligationState::Discharged),
            (id("check:a::X.c"), ObligationState::Stale),
        ];
        let d = check_o004(&obs, &KindPolicy::new(), &LintConfig::new(), false);
        assert!(d.is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/o004_w005.rs::check_w005
    #[test]
    fn w005_fires_within_window_only() {
        let cfg = LintConfig::new();
        let run =
            |exp: &str| check_w005(&[waiver(Some(exp))], &[], Some("2026-10-13"), 30, &cfg).len();
        assert_eq!(run("2026-11-12"), 1);
        assert_eq!(run("2026-10-13"), 1);
        assert_eq!(run("2026-11-13"), 0);
        assert_eq!(run("2026-10-12"), 0);
        let d = check_w005(
            &[waiver(Some("2026-11-01"))],
            &[],
            Some("2026-10-13"),
            30,
            &cfg,
        );
        assert_eq!(d[0].severity, Severity::Info);
    }

    // frob:tests crates/cadetta-lint/src/rules/o004_w005.rs::check_w005
    #[test]
    fn w005_covers_eol_and_skips_open_ended_and_no_as_of() {
        let cfg = LintConfig::new();
        let eols = [("a::Bolt".to_owned(), "2026-10-20".to_owned())];
        let d = check_w005(&[waiver(None)], &eols, Some("2026-10-13"), 30, &cfg);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].path.as_deref(), Some("a::Bolt"));
        assert!(check_w005(&[], &eols, None, 30, &cfg).is_empty());
        assert!(check_w005(&[], &eols, Some("2026-10-13"), 3, &cfg).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/o004_w005.rs::check_w005
    #[test]
    fn day_number_spans_month_and_leap_boundaries() {
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(
            day_number("2028-03-01"),
            day_number("2028-02-29").map(|d| d + 1)
        );
        assert_eq!(
            day_number("2028-02"),
            day_number("2028-02-29"),
            "month-only is the month's last day"
        );
        assert_eq!(day_number("2026-13-01"), None);
        assert_eq!(day_number("26-1-1"), None);
    }
}
