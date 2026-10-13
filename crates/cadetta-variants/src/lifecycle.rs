// frob:ticket 01M4ST8EGHZA7YAZT8AEVQJJSN
//! The `lifecycle` attribute and replacement candidates (language.md 3.2, 3.11).
//!
//! `#[lifecycle(active|nrnd|eol|obsolete, "date")]` is read into a
//! [`Lifecycle`]. [`analyse`] then turns a primary's lifecycle and its
//! approved alternates into L001/L002 facts (lints.md L): an `eol` or
//! `obsolete` primary with no viable alternate is L001, an error once its
//! date has passed `--as-of`; an `nrnd` primary is L002. The facts carry
//! the viable alternates as replacement candidates. For `active` the date
//! means "last verified" and never raises a fact.

use std::fmt;

use cadetta_syntax::ast::{Attr, AttrArg, ExprKind, Span};
use tracing::{debug, trace, warn};

/// The diagnostic code for an end-of-life primary with no viable alternate.
pub const L001: &str = "L001";
/// The diagnostic code for a not-recommended-for-new-design primary.
pub const L002: &str = "L002";

/// A part's lifecycle status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// In production; the date is when this was last verified.
    Active,
    /// Not recommended for new designs; the date is when that took effect.
    Nrnd,
    /// End of life; the date is the last-time buy.
    Eol,
    /// No longer available; the date is when it stopped being so.
    Obsolete,
}

impl Status {
    /// Parses the status word as written in the attribute.
    pub fn parse(word: &str) -> Option<Status> {
        match word {
            "active" => Some(Status::Active),
            "nrnd" => Some(Status::Nrnd),
            "eol" => Some(Status::Eol),
            "obsolete" => Some(Status::Obsolete),
            _ => None,
        }
    }

    /// The status word as written in source.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Nrnd => "nrnd",
            Status::Eol => "eol",
            Status::Obsolete => "obsolete",
        }
    }

    /// True for `eol` and `obsolete`, the statuses L001 reads.
    pub fn is_end_of_life(self) -> bool {
        matches!(self, Status::Eol | Status::Obsolete)
    }

    /// What the attribute's date means for this status (language.md 3.2).
    pub fn date_meaning(self) -> DateMeaning {
        match self {
            Status::Active => DateMeaning::LastVerified,
            Status::Nrnd | Status::Obsolete => DateMeaning::TakesEffect,
            Status::Eol => DateMeaning::LastTimeBuy,
        }
    }
}

/// What a lifecycle date means, which depends on the status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateMeaning {
    /// `active`: when the status was last verified; never expires.
    LastVerified,
    /// `nrnd`, `obsolete`: when the status takes effect.
    TakesEffect,
    /// `eol`: the last-time buy.
    LastTimeBuy,
}

/// A calendar date written `YYYY-MM` or `YYYY-MM-DD`.
///
/// A month-only date covers its whole month: it has passed only once the
/// as-of date is in a later month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifecycleDate {
    year: u16,
    month: u8,
    day: Option<u8>,
}

impl LifecycleDate {
    /// Parses `YYYY-MM` or `YYYY-MM-DD`, rejecting impossible months and days.
    pub fn parse(text: &str) -> Option<LifecycleDate> {
        let mut parts = text.split('-');
        let year = parts.next()?;
        let month = parts.next()?;
        let day = parts.next();
        if parts.next().is_some() || year.len() != 4 || month.len() != 2 {
            return None;
        }
        let digits = |s: &str| s.bytes().all(|c| c.is_ascii_digit());
        if !digits(year) || !digits(month) {
            return None;
        }
        let year: u16 = year.parse().ok()?;
        let month: u8 = month.parse().ok()?;
        if !(1..=12).contains(&month) {
            return None;
        }
        let day = match day {
            None => None,
            Some(d) if d.len() == 2 && digits(d) => {
                let d: u8 = d.parse().ok()?;
                if d == 0 || d > days_in_month(year, month) {
                    return None;
                }
                Some(d)
            }
            Some(_) => return None,
        };
        Some(LifecycleDate { year, month, day })
    }

    /// True when `as_of` is strictly after this date (a whole month for `YYYY-MM`).
    pub fn has_passed(&self, as_of: &LifecycleDate) -> bool {
        let this = (self.year, self.month, self.day.unwrap_or(0));
        match self.day {
            Some(_) => (as_of.year, as_of.month, as_of.day.unwrap_or(1)) > this,
            None => (as_of.year, as_of.month) > (self.year, self.month),
        }
    }
}

impl fmt::Display for LifecycleDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)?;
        if let Some(d) = self.day {
            write!(f, "-{d:02}")?;
        }
        Ok(())
    }
}

fn days_in_month(y: u16, m: u8) -> u8 {
    let leap = y.is_multiple_of(400) || (y.is_multiple_of(4) && !y.is_multiple_of(100));
    match m {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A parsed `#[lifecycle(status, "date")]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lifecycle {
    /// The status.
    pub status: Status,
    /// The date; its meaning is [`Status::date_meaning`].
    pub date: LifecycleDate,
    /// Where the attribute is.
    pub span: Span,
}

/// Why a `lifecycle` attribute could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleError {
    /// The attribute is not `lifecycle`.
    NotLifecycle,
    /// It does not have exactly a status and a date string.
    Shape,
    /// The status word is not active, nrnd, eol or obsolete.
    BadStatus(String),
    /// The date is not `YYYY-MM` or `YYYY-MM-DD`.
    BadDate(String),
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleError::NotLifecycle => write!(f, "not a lifecycle attribute"),
            LifecycleError::Shape => {
                write!(f, "lifecycle takes a status and a date string")
            }
            LifecycleError::BadStatus(s) => {
                write!(f, "`{s}` is not one of active, nrnd, eol, obsolete")
            }
            LifecycleError::BadDate(s) => {
                write!(f, "`{s}` is not a YYYY-MM or YYYY-MM-DD date")
            }
        }
    }
}

impl std::error::Error for LifecycleError {}

/// Reads a `lifecycle` attribute into a [`Lifecycle`].
pub fn parse_attr(attr: &Attr) -> Result<Lifecycle, LifecycleError> {
    if attr.name.name != "lifecycle" {
        return Err(LifecycleError::NotLifecycle);
    }
    let [AttrArg::Expr(status), AttrArg::Expr(date)] = attr.args.as_slice() else {
        warn!(span = ?attr.span, "lifecycle attribute has the wrong shape");
        return Err(LifecycleError::Shape);
    };
    let ExprKind::Path(path) = &status.kind else {
        return Err(LifecycleError::Shape);
    };
    let ExprKind::Str(date_text) = &date.kind else {
        return Err(LifecycleError::Shape);
    };
    let word = path.text();
    let status = Status::parse(&word).ok_or_else(|| {
        warn!(%word, "unknown lifecycle status");
        LifecycleError::BadStatus(word.clone())
    })?;
    let date = LifecycleDate::parse(date_text).ok_or_else(|| {
        warn!(date = %date_text, "unreadable lifecycle date");
        LifecycleError::BadDate(date_text.clone())
    })?;
    debug!(status = status.as_str(), %date, "lifecycle parsed");
    Ok(Lifecycle {
        status,
        date,
        span: attr.span,
    })
}

/// Finds and reads the `lifecycle` attribute among `attrs`, if any.
pub fn find(attrs: &[Attr]) -> Option<Result<Lifecycle, LifecycleError>> {
    attrs
        .iter()
        .find(|a| a.name.name == "lifecycle")
        .map(parse_attr)
}

/// An approved alternate as lifecycle analysis sees it.
#[derive(Clone, Debug)]
pub struct Alternate {
    /// The alternate's name.
    pub name: String,
    /// Its lifecycle, when it declares one.
    pub lifecycle: Option<Lifecycle>,
    /// True when it meets every bound, check and fit under variant evaluation.
    pub viable: bool,
}

impl Alternate {
    /// Viable and not itself at end of life: a usable replacement.
    pub fn is_candidate(&self) -> bool {
        self.viable && !self.lifecycle.is_some_and(|l| l.status.is_end_of_life())
    }
}

/// How serious a lifecycle fact is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactSeverity {
    /// L002.
    Info,
    /// L001 before its date.
    Warning,
    /// L001 once its date has passed.
    Error,
}

/// An L001 or L002 fact about one primary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleFact {
    /// `L001` or `L002`.
    pub code: &'static str,
    /// Effective severity before config overrides.
    pub severity: FactSeverity,
    /// The primary the fact is about.
    pub primary: String,
    /// Its status.
    pub status: Status,
    /// The date as written.
    pub date: LifecycleDate,
    /// True when the date has passed the as-of date.
    pub date_passed: bool,
    /// Viable alternates that could replace the primary (L002 lists them too).
    pub candidates: Vec<String>,
    /// Where the lifecycle attribute is.
    pub span: Span,
}

/// Names the alternates that could replace a primary, in declaration order.
pub fn replacement_candidates(alternates: &[Alternate]) -> Vec<String> {
    alternates
        .iter()
        .filter(|a| a.is_candidate())
        .map(|a| a.name.clone())
        .collect()
}

/// Records the L001/L002 fact for a primary, if its lifecycle raises one.
///
/// `as_of` is the fixed date every date-dependent rule reads.
pub fn analyse(
    primary: &str,
    lifecycle: Option<&Lifecycle>,
    alternates: &[Alternate],
    as_of: &LifecycleDate,
) -> Option<LifecycleFact> {
    let lc = lifecycle?;
    let candidates = replacement_candidates(alternates);
    let date_passed = lc.date.has_passed(as_of);
    let (code, severity) = match lc.status {
        Status::Active => {
            trace!(primary, "active primary; date is last verified");
            return None;
        }
        Status::Nrnd => (L002, FactSeverity::Info),
        Status::Eol | Status::Obsolete if !candidates.is_empty() => {
            trace!(
                primary,
                ?candidates,
                "end-of-life primary has a viable alternate"
            );
            return None;
        }
        Status::Eol | Status::Obsolete if date_passed => (L001, FactSeverity::Error),
        Status::Eol | Status::Obsolete => (L001, FactSeverity::Warning),
    };
    debug!(
        primary,
        code,
        ?severity,
        date_passed,
        "lifecycle fact recorded"
    );
    Some(LifecycleFact {
        code,
        severity,
        primary: primary.to_string(),
        status: lc.status,
        date: lc.date,
        date_passed,
        candidates,
        span: lc.span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::{Expr, Ident, Path};

    fn ident(s: &str) -> Ident {
        Ident {
            name: s.to_string(),
            span: Span::default(),
        }
    }

    fn attr(status: &str, date: &str) -> Attr {
        let status_expr = Expr {
            kind: ExprKind::Path(Path {
                segments: vec![ident(status)],
                seps: vec![],
                span: Span::default(),
            }),
            span: Span::default(),
        };
        let date_expr = Expr {
            kind: ExprKind::Str(date.to_string()),
            span: Span::default(),
        };
        Attr {
            name: ident("lifecycle"),
            args: vec![AttrArg::Expr(status_expr), AttrArg::Expr(date_expr)],
            inner: false,
            span: Span::default(),
        }
    }

    fn lc(status: Status, date: &str) -> Lifecycle {
        Lifecycle {
            status,
            date: LifecycleDate::parse(date).unwrap(),
            span: Span::default(),
        }
    }

    fn d(s: &str) -> LifecycleDate {
        LifecycleDate::parse(s).unwrap()
    }

    fn alt(name: &str, viable: bool, status: Option<Status>) -> Alternate {
        Alternate {
            name: name.to_string(),
            lifecycle: status.map(|s| lc(s, "2030-01")),
            viable,
        }
    }

    // frob:tests crates/cadetta-variants/src/lifecycle.rs::parse_attr kind=unit
    #[test]
    fn parses_status_and_date() {
        let got = parse_attr(&attr("eol", "2027-06")).unwrap();
        assert_eq!(got.status, Status::Eol);
        assert_eq!(got.date, d("2027-06"));
        for (w, s) in [
            ("active", Status::Active),
            ("nrnd", Status::Nrnd),
            ("obsolete", Status::Obsolete),
        ] {
            assert_eq!(parse_attr(&attr(w, "2026-01-15")).unwrap().status, s);
        }
        assert_eq!(
            parse_attr(&attr("dead", "2027-06")),
            Err(LifecycleError::BadStatus("dead".into()))
        );
        assert!(matches!(
            parse_attr(&attr("eol", "2027-13")),
            Err(LifecycleError::BadDate(_))
        ));
    }

    // frob:tests crates/cadetta-variants/src/lifecycle.rs::analyse kind=unit
    #[test]
    fn eol_without_alternate_is_l001_error_after_date() {
        let p = lc(Status::Eol, "2027-06");
        let early = analyse("P", Some(&p), &[], &d("2027-06-30")).unwrap();
        assert_eq!((early.code, early.severity), (L001, FactSeverity::Warning));
        let late = analyse("P", Some(&p), &[], &d("2027-07-01")).unwrap();
        assert_eq!((late.code, late.severity), (L001, FactSeverity::Error));
        assert!(late.date_passed);
        let alts = [alt("A", false, None), alt("B", true, Some(Status::Eol))];
        assert!(analyse("P", Some(&p), &alts, &d("2028-01-01")).is_some());
        let obs = lc(Status::Obsolete, "2020-01-01");
        let alts = [alt("A", true, Some(Status::Active))];
        assert!(analyse("P", Some(&obs), &alts, &d("2028-01-01")).is_none());
    }

    // frob:tests crates/cadetta-variants/src/lifecycle.rs::analyse kind=unit
    #[test]
    fn nrnd_is_l002_with_candidates() {
        let p = lc(Status::Nrnd, "2026-01");
        let alts = [alt("A", true, None), alt("B", false, None)];
        let f = analyse("P", Some(&p), &alts, &d("2026-10-13")).unwrap();
        assert_eq!((f.code, f.severity), (L002, FactSeverity::Info));
        assert_eq!(f.candidates, vec!["A".to_string()]);
    }

    // frob:tests crates/cadetta-variants/src/lifecycle.rs::analyse kind=unit
    #[test]
    fn active_date_is_last_verified_and_raises_nothing() {
        let p = lc(Status::Active, "2020-01");
        assert_eq!(p.status.date_meaning(), DateMeaning::LastVerified);
        assert!(analyse("P", Some(&p), &[], &d("2030-01-01")).is_none());
        assert!(analyse("P", None, &[], &d("2030-01-01")).is_none());
    }
}
