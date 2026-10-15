//! The geometry push plan (adapters.md 3.4): which native object a geometry source form binds
//! to, whether the host ever writes it, what a read reports when the CAD tool disagrees, and the
//! sign applied on write and read.
// frob:ticket 01M4Z9WQS0B2QY3G3TZY25XAXV

use crate::geometry_bind::{RotationOp, rotation_pushable};

/// A geometry source form, as the adapters.md 3.4 table lists them.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceForm {
    /// A `datum`.
    Datum,
    /// A coordinate derived from a generator.
    DerivedCoordinate,
    /// A pattern generator member (pitch, radius, start, rotation, count).
    PatternMember,
    /// A placement (the component instance), verify-only.
    Placement,
    /// A placement with no pattern, bound through its mate.
    PlainPlacement,
    /// A placement offset member (`place.x`).
    PlaceOffset,
    /// `flip` or `rotate(axis = angle)`.
    Rotation(RotationOp),
    /// A `for` placement over a pattern.
    ArrayPlacement,
}

/// What the host reports on `read` when the observed native value differs from the design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadReport {
    /// The comparison is a blocking R001.
    R001,
    /// A drift note only; nothing blocks.
    DriftNote,
    /// Nothing is reported (the values agree).
    Nothing,
}

impl ReadReport {
    /// The word the conformance corpus uses for this report.
    pub fn word(self) -> &'static str {
        match self {
            Self::R001 => "R001",
            Self::DriftNote => "drift note",
            Self::Nothing => "none",
        }
    }
}

/// The plan for one source form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushPlan {
    /// The native object the form binds to, or `None` when it has no binding.
    pub binds_to: Option<&'static str>,
    /// True when the host may write the native object on `sync`.
    pub push_writes: bool,
    /// What `read` reports when the observed value differs from the design.
    pub on_drift: ReadReport,
}

/// A source-form word that [`parse_source_form`] does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownForm(pub String);

impl std::fmt::Display for UnknownForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown geometry source form `{}`", self.0)
    }
}

impl std::error::Error for UnknownForm {}

/// Pattern generator member names (geometry.md 6): `pitch*`, `radius`, `start`, `rotation`, `count`.
fn is_generator_member(name: &str) -> bool {
    ["pitch", "radius", "start", "rotation", "count"]
        .iter()
        .any(|g| name.starts_with(g))
}

/// Parse a source-form word: `datum`, `derived-coordinate`, `<owner>.<member>`, `place.<axis>`,
/// `placement`, `plain placement`, `flip`, `rotate <n>deg`, `for-placement over pattern`.
pub fn parse_source_form(word: &str) -> Result<SourceForm, UnknownForm> {
    let word = word.trim();
    let form = match word {
        "datum" => SourceForm::Datum,
        "derived-coordinate" => SourceForm::DerivedCoordinate,
        "placement" => SourceForm::Placement,
        "plain placement" => SourceForm::PlainPlacement,
        "for-placement over pattern" => SourceForm::ArrayPlacement,
        "flip" => SourceForm::Rotation(RotationOp::Flip),
        _ => {
            if let Some(angle) = word.strip_prefix("rotate ") {
                let degrees = angle
                    .trim()
                    .strip_suffix("deg")
                    .and_then(|n| n.trim().parse::<f64>().ok())
                    .ok_or_else(|| UnknownForm(word.to_owned()))?;
                SourceForm::Rotation(RotationOp::Rotate { degrees })
            } else if let Some((owner, member)) = word.split_once('.') {
                if owner == "place" {
                    SourceForm::PlaceOffset
                } else if member.starts_with("derived") {
                    SourceForm::DerivedCoordinate
                } else if is_generator_member(member) {
                    SourceForm::PatternMember
                } else {
                    return Err(UnknownForm(word.to_owned()));
                }
            } else {
                return Err(UnknownForm(word.to_owned()));
            }
        }
    };
    tracing::trace!(word, ?form, "parsed geometry source form");
    Ok(form)
}

/// Plan one source form. A derived coordinate binds only when the user `listed` it by hand.
pub fn plan_source_form(form: &SourceForm, listed: bool) -> PushPlan {
    let bound = |to: &'static str| PushPlan {
        binds_to: Some(to),
        push_writes: true,
        on_drift: ReadReport::Nothing,
    };
    let plan = match form {
        SourceForm::Datum => bound("frame"),
        SourceForm::DerivedCoordinate if listed => bound("dimension"),
        SourceForm::DerivedCoordinate => PushPlan {
            binds_to: None,
            push_writes: false,
            on_drift: ReadReport::Nothing,
        },
        SourceForm::PatternMember => bound("pattern dimension"),
        SourceForm::PlaceOffset => bound("mate distance dimension"),
        SourceForm::PlainPlacement => bound("mate dimension"),
        SourceForm::Placement => PushPlan {
            binds_to: Some("instance"),
            push_writes: false,
            on_drift: ReadReport::R001,
        },
        SourceForm::ArrayPlacement => PushPlan {
            binds_to: Some("native component pattern generator"),
            push_writes: false,
            on_drift: ReadReport::Nothing,
        },
        SourceForm::Rotation(op) if rotation_pushable(*op) => bound("angle mate dimension"),
        SourceForm::Rotation(_) => PushPlan {
            binds_to: None,
            push_writes: false,
            on_drift: ReadReport::DriftNote,
        },
    };
    tracing::debug!(?form, listed, ?plan, "planned geometry source form");
    plan
}

/// The report `read` gives for `plan` given whether the observed value `differs` from the design.
pub fn read_report(plan: &PushPlan, differs: bool) -> ReadReport {
    if differs {
        plan.on_drift
    } else {
        ReadReport::Nothing
    }
}

/// Prefix `-` on a written decimal (`6.35mm` -> `-6.35mm`), keeping zero unsigned.
pub fn negate_text(text: &str) -> String {
    if let Some(rest) = text.strip_prefix('-') {
        return rest.to_owned();
    }
    let numeric: String = text
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if numeric.chars().all(|c| c == '0' || c == '.') {
        text.to_owned()
    } else {
        format!("-{text}")
    }
}

/// The text written to the tool for a design `value` under `sign` (`-1` negates).
pub fn native_written(value: &str, sign: Option<i64>) -> String {
    if sign == Some(-1) {
        negate_text(value)
    } else {
        value.to_owned()
    }
}

/// The design value read back from a tool's `native` text under `sign`: the inverse of
/// [`native_written`], so a negated coordinate reads back as its magnitude.
pub fn read_back_as(native: &str, sign: Option<i64>) -> String {
    native_written(native, sign)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(word: &str, listed: bool) -> PushPlan {
        plan_source_form(&parse_source_form(word).expect("known form"), listed)
    }

    // frob:tests crate::geometry_plan::plan_source_form
    #[test]
    fn table_rows_bind_and_push() {
        assert_eq!(plan("datum", false).binds_to, Some("frame"));
        assert_eq!(
            plan("place.x", false).binds_to,
            Some("mate distance dimension")
        );
        assert_eq!(
            plan("holes.pitch_x", false).binds_to,
            Some("pattern dimension")
        );
        assert!(plan("rotate 45deg", false).push_writes);
        let ninety = plan("rotate 90deg", false);
        assert_eq!((ninety.binds_to, ninety.push_writes), (None, false));
        assert!(!plan("placement", false).push_writes);
    }

    // frob:tests crate::geometry_plan::plan_source_form
    #[test]
    fn derived_coordinate_binds_only_when_listed() {
        assert_eq!(plan("derived-coordinate", false).binds_to, None);
        assert_eq!(plan("derived-coordinate", true).binds_to, Some("dimension"));
        assert_eq!(plan("holes.derived_x", false).binds_to, None);
    }

    // frob:tests crate::geometry_plan::read_report
    #[test]
    fn drift_reports_by_form() {
        let flip = plan("flip", false);
        assert_eq!(read_report(&flip, true), ReadReport::DriftNote);
        assert_eq!(read_report(&flip, false), ReadReport::Nothing);
        assert_eq!(
            read_report(&plan("placement", false), true),
            ReadReport::R001
        );
    }

    // frob:tests crate::geometry_plan::parse_source_form
    #[test]
    fn unknown_form_is_an_error() {
        assert!(parse_source_form("holes.mystery").is_err());
        assert!(parse_source_form("rotate fast").is_err());
    }

    // frob:tests crate::geometry_plan::native_written
    #[test]
    fn sign_negates_on_write_and_restores_on_read() {
        assert_eq!(native_written("10mm", Some(-1)), "-10mm");
        assert_eq!(native_written("10mm", Some(1)), "10mm");
        assert_eq!(read_back_as("-10mm", Some(-1)), "10mm");
        assert_eq!(native_written("0mm", Some(-1)), "0mm");
    }
}
