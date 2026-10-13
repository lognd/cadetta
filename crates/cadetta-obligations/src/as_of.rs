//! The fixed as-of date read by every date-dependent rule.
//!
//! Specification: `docs/spec/obligations.md` section 5 and
//! `docs/spec/config-cli.md` section 4.
//! frob:ticket 01M4SWNH4WXPBR7FYSY7EAJ4K1

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use tracing::{debug, trace};

/// Where the as-of date came from; recorded in the report header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsOfSource {
    /// The `--as-of` command-line flag.
    Flag,
    /// The `[package] as_of` key of `cadetta.toml`.
    Config,
    /// The system clock (the default).
    Today,
}

impl AsOfSource {
    /// Stable lowercase name used in the JSON header.
    pub fn as_str(self) -> &'static str {
        match self {
            AsOfSource::Flag => "flag",
            AsOfSource::Config => "config",
            AsOfSource::Today => "today",
        }
    }
}

/// A calendar date (proleptic Gregorian), ordered chronologically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

/// An invalid as-of date string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("`{0}` is not a valid YYYY-MM-DD date")]
pub struct DateError(pub String);

impl Date {
    /// Parse a strict `YYYY-MM-DD` date, rejecting impossible days.
    pub fn parse(text: &str) -> Result<Date, DateError> {
        let bad = || DateError(text.to_string());
        let b = text.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return Err(bad());
        }
        let digits = |r: std::ops::Range<usize>| -> Result<u32, DateError> {
            let s = &text[r];
            if s.bytes().all(|c| c.is_ascii_digit()) {
                s.parse().map_err(|_| bad())
            } else {
                Err(bad())
            }
        };
        let (y, m, d) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
        if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
            return Err(bad());
        }
        Ok(Date {
            year: y as u16,
            month: m as u8,
            day: d as u8,
        })
    }

    /// The date `secs` seconds after the Unix epoch (UTC).
    pub fn from_unix_secs(secs: u64) -> Date {
        // Howard Hinnant's civil-from-days algorithm.
        let z = (secs / 86_400) as i64 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let y = (yoe + era * 400 + i64::from(m <= 2)) as u16;
        Date {
            year: y,
            month: m,
            day: d,
        }
    }

    /// Today's date from the system clock (UTC).
    pub fn today() -> Date {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Date::from_unix_secs(secs)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

fn days_in_month(y: u32, m: u32) -> u32 {
    let leap = y.is_multiple_of(400) || (y.is_multiple_of(4) && !y.is_multiple_of(100));
    match m {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The resolved as-of date and its provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsOf {
    /// The date every date-dependent rule reads.
    pub date: Date,
    /// Which input supplied it.
    pub source: AsOfSource,
}

impl AsOf {
    /// Resolve with precedence flag, then `[package] as_of`, then `today`.
    pub fn resolve(
        flag: Option<&str>,
        config: Option<&str>,
        today: Date,
    ) -> Result<AsOf, DateError> {
        let (date, source) = if let Some(f) = flag {
            (Date::parse(f)?, AsOfSource::Flag)
        } else if let Some(c) = config {
            (Date::parse(c)?, AsOfSource::Config)
        } else {
            (today, AsOfSource::Today)
        };
        debug!(date = %date, source = source.as_str(), "as-of date resolved");
        Ok(AsOf { date, source })
    }

    /// Whether `expires` has passed as of this date (W001): strictly before.
    pub fn has_passed(&self, expires: Date) -> bool {
        let passed = expires < self.date;
        trace!(expires = %expires, as_of = %self.date, passed, "expiry judged");
        passed
    }

    /// The `as_of` fields of the JSON report header.
    pub fn header_json(&self) -> String {
        format!(
            "{{\"as_of\":\"{}\",\"as_of_source\":\"{}\"}}",
            self.date,
            self.source.as_str()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    // frob:tests crates/cadetta-obligations/src/as_of.rs::tests.flag_wins_and_is_recorded
    #[test]
    fn flag_wins_and_is_recorded() {
        let a = AsOf::resolve(Some("2025-01-02"), Some("2026-10-13"), d("2030-01-01")).unwrap();
        assert_eq!(a.date, d("2025-01-02"));
        assert_eq!(a.source, AsOfSource::Flag);
        assert_eq!(
            a.header_json(),
            r#"{"as_of":"2025-01-02","as_of_source":"flag"}"#
        );
    }

    // frob:tests crates/cadetta-obligations/src/as_of.rs::tests.config_then_today
    #[test]
    fn config_then_today() {
        let a = AsOf::resolve(None, Some("2026-10-13"), d("2030-01-01")).unwrap();
        assert_eq!((a.date, a.source), (d("2026-10-13"), AsOfSource::Config));
        let t = AsOf::resolve(None, None, d("2030-01-01")).unwrap();
        assert_eq!((t.date, t.source), (d("2030-01-01"), AsOfSource::Today));
    }

    // frob:tests crates/cadetta-obligations/src/as_of.rs::tests.expiry_uses_fixed_date
    #[test]
    fn expiry_uses_fixed_date() {
        let a = AsOf::resolve(Some("2026-06-01"), None, d("2030-01-01")).unwrap();
        assert!(a.has_passed(d("2026-05-31")));
        assert!(!a.has_passed(d("2026-06-01")));
        assert!(!a.has_passed(d("2026-06-02")));
    }

    // frob:tests crates/cadetta-obligations/src/as_of.rs::tests.rejects_bad_dates
    #[test]
    fn rejects_bad_dates() {
        for bad in [
            "2026-02-30",
            "2026-13-01",
            "26-01-01",
            "2026/01/01",
            "2026-1-01",
            "",
        ] {
            assert!(Date::parse(bad).is_err(), "{bad}");
        }
        assert!(AsOf::resolve(Some("nope"), None, d("2030-01-01")).is_err());
        assert!(Date::parse("2028-02-29").is_ok());
    }

    // frob:tests crates/cadetta-obligations/src/as_of.rs::tests.unix_epoch_conversion
    #[test]
    fn unix_epoch_conversion() {
        assert_eq!(Date::from_unix_secs(0).to_string(), "1970-01-01");
        assert_eq!(
            Date::from_unix_secs(1_782_000_000).to_string(),
            "2026-06-21"
        );
        assert_eq!(Date::from_unix_secs(951_782_400).to_string(), "2000-02-29");
    }
}
