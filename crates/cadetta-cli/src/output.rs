//! frob:ticket 01M4SPWSXBTNXW3CPZ6FVHT5ZT
//! Output formats, the `cadetta.diag/1` record and exit codes
//! (config-cli.md 3, 5; lints.md 1.1, 1.2).
//!
//! Text is the default in every context; JSON only by `--format json` or
//! `CADETTA_FORMAT=json`, never by terminal detection.

use std::fmt::Write as _;
use std::str::FromStr;

use serde::Serialize;
use serde_json::json;

use crate::config::{FailOn, Severity};

/// Exit code when diagnostics reach `fail_on` (config-cli.md 3).
pub const EXIT_DIAGNOSTICS: u8 = 1;

/// Schema tag carried by every diagnostic record.
pub const DIAG_SCHEMA: &str = "cadetta.diag/1";

/// An output format selected by `--format` or `CADETTA_FORMAT` (config-cli.md 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// rustc-style rendering; the default everywhere.
    #[default]
    Text,
    /// One line per diagnostic.
    Short,
    /// GitHub workflow annotations.
    Github,
    /// SARIF 2.1.0 for code scanning.
    Sarif,
    /// The command's JSON schema.
    Json,
}

/// A `--format` value that is not one of the five formats.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("unknown output format `{0}` (expected text, short, github, sarif or json)")]
pub struct UnknownFormat(pub String);

impl FromStr for Format {
    type Err = UnknownFormat;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "text" => Ok(Self::Text),
            "short" => Ok(Self::Short),
            "github" => Ok(Self::Github),
            "sarif" => Ok(Self::Sarif),
            "json" => Ok(Self::Json),
            other => {
                tracing::warn!(format = other, "rejecting unknown output format");
                Err(UnknownFormat(other.to_owned()))
            }
        }
    }
}

/// A 1-based line and column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Position {
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub col: u32,
}

/// Where to look (lints.md 1.1 `location`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Location {
    /// A span in `.cdt` source.
    Source {
        /// File path relative to the project root.
        file: String,
        /// Span start.
        start: Position,
        /// Span end.
        end: Position,
    },
    /// A locator in a CAD-native file.
    Native {
        /// Adapter id.
        adapter: String,
        /// Native file.
        file: String,
        /// Adapter locator.
        locator: String,
    },
    /// A line in `cadetta.lock`.
    Lock {
        /// 1-based line.
        line: u32,
    },
}

/// A related fact or location attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Note {
    /// The note text.
    pub message: String,
    /// Where it applies, if anywhere.
    pub location: Option<Location>,
}

/// A suppression recorded on a waived diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Waived {
    /// The waiver that suppressed it.
    pub waiver: String,
    /// Its `because` reason.
    pub because: String,
}

/// One `cadetta.diag/1` record (lints.md 1.1), minus `fixes` until a fixing rule exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// Always [`DIAG_SCHEMA`].
    pub schema: &'static str,
    /// Catalog code.
    pub code: String,
    /// Effective severity.
    #[serde(serialize_with = "serialize_severity")]
    pub severity: Severity,
    /// Canonical path of the subject.
    pub path: Option<String>,
    /// `SLOT=ALTERNATE` under variant evaluation.
    pub alt: Option<String>,
    /// One sentence, no trailing period.
    pub message: String,
    /// Where to look.
    pub location: Option<Location>,
    /// Related facts.
    pub notes: Vec<Note>,
    /// Suppression, shown in JSON only.
    pub waived: Option<Waived>,
}

impl Diagnostic {
    /// A diagnostic with only a code, severity and message set.
    pub fn new(code: &str, severity: Severity, message: &str) -> Self {
        Self {
            schema: DIAG_SCHEMA,
            code: code.to_owned(),
            severity,
            path: None,
            alt: None,
            message: message.to_owned(),
            location: None,
            notes: Vec::new(),
            waived: None,
        }
    }

    /// The source file and start position, when the location has one.
    fn file_pos(&self) -> Option<(&str, Position)> {
        match self.location.as_ref()? {
            Location::Source { file, start, .. } => Some((file, *start)),
            Location::Native { file, .. } => Some((file, Position { line: 1, col: 1 })),
            Location::Lock { line } => Some((
                "cadetta.lock",
                Position {
                    line: *line,
                    col: 1,
                },
            )),
        }
    }
}

/// The lower-case name of a severity.
fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

/// Serde hook: write a [`Severity`] as its lower-case name.
fn serialize_severity<S: serde::Serializer>(
    severity: &Severity,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(severity_name(*severity))
}

/// Ordering rank of a severity (info < warning < error).
fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::Info => 0,
        Severity::Warning => 1,
        Severity::Error => 2,
    }
}

/// Sort into the deterministic (file, line, code, path) order (lints.md 1.1).
pub fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|a, b| {
        let key = |d: &Diagnostic| {
            d.file_pos()
                .map(|(f, p)| (f.to_owned(), p.line))
                .unwrap_or_default()
        };
        key(a)
            .cmp(&key(b))
            .then_with(|| a.code.cmp(&b.code))
            .then_with(|| a.path.cmp(&b.path))
    });
}

/// Exit code for a finished command: 1 when any unwaived diagnostic is at
/// or above `fail_on`, else 0 (config-cli.md 3).
pub fn exit_code(diagnostics: &[Diagnostic], fail_on: FailOn) -> u8 {
    let threshold = match fail_on {
        FailOn::Never => return crate::EXIT_OK,
        FailOn::Info => 0,
        FailOn::Warning => 1,
        FailOn::Error => 2,
    };
    let failing = diagnostics
        .iter()
        .any(|d| d.waived.is_none() && rank(d.severity) >= threshold);
    tracing::debug!(failing, ?fail_on, "computed diagnostic exit code");
    if failing {
        EXIT_DIAGNOSTICS
    } else {
        crate::EXIT_OK
    }
}

/// Render `diagnostics` in `format`; `source_line(file, line)` supplies
/// excerpt text for the text format. Waived diagnostics appear in JSON only.
pub fn render(
    format: Format,
    diagnostics: &[Diagnostic],
    source_line: &dyn Fn(&str, u32) -> Option<String>,
) -> String {
    tracing::debug!(?format, count = diagnostics.len(), "rendering diagnostics");
    if format == Format::Json {
        let report = json!({ "diagnostics": diagnostics });
        return format!("{report}\n");
    }
    let shown: Vec<&Diagnostic> = diagnostics.iter().filter(|d| d.waived.is_none()).collect();
    match format {
        Format::Text => shown.iter().map(|d| render_text(d, source_line)).collect(),
        Format::Short => shown.iter().map(|d| render_short(d)).collect(),
        Format::Github => shown.iter().map(|d| render_github(d)).collect(),
        Format::Sarif => format!("{}\n", render_sarif(&shown)),
        Format::Json => unreachable!("handled above"),
    }
}

/// `file:line:col: severity[CODE]: message`.
fn render_short(d: &Diagnostic) -> String {
    let at = d.file_pos().map_or_else(
        || "-".to_owned(),
        |(f, p)| format!("{f}:{}:{}", p.line, p.col),
    );
    format!(
        "{at}: {}[{}]: {}\n",
        severity_name(d.severity),
        d.code,
        d.message
    )
}

/// Escape a workflow-command message.
fn gh_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escape a workflow-command property value.
fn gh_prop(text: &str) -> String {
    gh_data(text).replace(':', "%3A").replace(',', "%2C")
}

/// A GitHub workflow annotation line.
fn render_github(d: &Diagnostic) -> String {
    let level = match d.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "notice",
    };
    let mut props = String::new();
    if let Some((file, pos)) = d.file_pos() {
        let _ = write!(
            props,
            "file={},line={},col={},",
            gh_prop(file),
            pos.line,
            pos.col
        );
    }
    let _ = write!(props, "title={}", gh_prop(&d.code));
    format!("::{level} {props}::{}\n", gh_data(&d.message))
}

/// A SARIF 2.1.0 log for code scanning.
fn render_sarif(shown: &[&Diagnostic]) -> serde_json::Value {
    let results: Vec<_> = shown
        .iter()
        .map(|d| {
            let level = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "note",
            };
            let locations: Vec<_> = d
                .file_pos()
                .map(|(file, pos)| {
                    json!({ "physicalLocation": {
                        "artifactLocation": { "uri": file },
                        "region": { "startLine": pos.line, "startColumn": pos.col },
                    } })
                })
                .into_iter()
                .collect();
            json!({ "ruleId": d.code, "level": level,
                    "message": { "text": d.message }, "locations": locations })
        })
        .collect();
    json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": { "name": "cadetta", "version": env!("CARGO_PKG_VERSION") } },
            "results": results,
        }],
    })
}

/// The rustc-style rendering (lints.md 1.2).
fn render_text(d: &Diagnostic, source_line: &dyn Fn(&str, u32) -> Option<String>) -> String {
    let mut out = format!("{}[{}]: {}\n", severity_name(d.severity), d.code, d.message);
    let located = d.file_pos();
    let width = located.map_or(1, |(_, p)| p.line.to_string().len());
    let pad = " ".repeat(width);
    if let Some((file, pos)) = located {
        let _ = writeln!(out, "{pad}--> {file}:{}:{}", pos.line, pos.col);
        if let Some(text) = source_line(file, pos.line) {
            let span = match &d.location {
                Some(Location::Source { start, end, .. }) if end.line == start.line => {
                    end.col.saturating_sub(start.col).max(1) as usize
                }
                _ => 1,
            };
            let _ = writeln!(out, "{pad} |");
            let _ = writeln!(out, "{} | {text}", pos.line);
            let _ = writeln!(
                out,
                "{pad} | {}{}{}",
                " ".repeat(pos.col.saturating_sub(1) as usize),
                "^".repeat(span),
                d.alt
                    .as_ref()
                    .map_or(String::new(), |a| format!(" under {a}")),
            );
        }
        let _ = writeln!(out, "{pad} |");
    }
    for note in &d.notes {
        let _ = writeln!(out, "{pad} = note: {}", note.message);
    }
    if let Some(path) = &d.path {
        let _ = writeln!(out, "{pad} = path: {path}");
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Diagnostic {
        let mut d = Diagnostic::new(
            "C001",
            Severity::Error,
            "false for every value in tolerance",
        );
        d.path = Some("acme.rover::Rover.load_ok".into());
        d.location = Some(Location::Source {
            file: "cad/rover.cdt".into(),
            start: Position { line: 41, col: 3 },
            end: Position { line: 41, col: 8 },
        });
        d.notes.push(Note {
            message: "wheel.mass = 1.2kg".into(),
            location: None,
        });
        d
    }

    fn no_source(_: &str, _: u32) -> Option<String> {
        None
    }

    // frob:tests crates/cadetta-cli/src/output.rs::exit_code
    #[test]
    fn exit_one_at_or_above_fail_on() {
        let warn = Diagnostic::new("W004", Severity::Warning, "m");
        assert_eq!(
            exit_code(std::slice::from_ref(&warn), FailOn::Warning),
            EXIT_DIAGNOSTICS
        );
        assert_eq!(exit_code(std::slice::from_ref(&warn), FailOn::Error), 0);
        assert_eq!(exit_code(std::slice::from_ref(&warn), FailOn::Never), 0);
        assert_eq!(exit_code(&[sample()], FailOn::Error), EXIT_DIAGNOSTICS);
        assert_eq!(exit_code(&[], FailOn::Info), 0);
        let mut waived = sample();
        waived.waived = Some(Waived {
            waiver: "w".into(),
            because: "b".into(),
        });
        assert_eq!(exit_code(&[waived], FailOn::Error), 0);
    }

    // frob:tests crates/cadetta-cli/src/output.rs::Format
    #[test]
    fn format_parses_all_five_and_text_is_default() {
        for (name, f) in [
            ("text", Format::Text),
            ("short", Format::Short),
            ("github", Format::Github),
            ("sarif", Format::Sarif),
            ("json", Format::Json),
        ] {
            assert_eq!(name.parse::<Format>(), Ok(f));
        }
        assert_eq!(Format::default(), Format::Text);
        assert!("yaml".parse::<Format>().is_err());
    }

    // frob:tests crates/cadetta-cli/src/output.rs::render
    #[test]
    fn text_is_rustc_style() {
        let src = |_: &str, _: u32| Some("  check wheel.mass < 1kg".to_owned());
        let text = render(Format::Text, &[sample()], &src);
        assert!(text.starts_with("error[C001]: false for every value in tolerance\n"));
        assert!(text.contains("  --> cad/rover.cdt:41:3\n"));
        assert!(text.contains("41 |   check wheel.mass < 1kg\n"));
        assert!(text.contains("   |   ^^^^^\n"));
        assert!(text.contains("   = note: wheel.mass = 1.2kg\n"));
        assert!(text.contains("   = path: acme.rover::Rover.load_ok\n"));
    }

    // frob:tests crates/cadetta-cli/src/output.rs::render
    #[test]
    fn short_github_sarif_and_json_shapes() {
        let d = [sample()];
        assert_eq!(
            render(Format::Short, &d, &no_source),
            "cad/rover.cdt:41:3: error[C001]: false for every value in tolerance\n"
        );
        assert_eq!(
            render(Format::Github, &d, &no_source),
            "::error file=cad/rover.cdt,line=41,col=3,title=C001::false for every value in tolerance\n"
        );
        let sarif: serde_json::Value =
            serde_json::from_str(&render(Format::Sarif, &d, &no_source)).unwrap();
        assert_eq!(sarif["version"], "2.1.0");
        assert_eq!(sarif["runs"][0]["results"][0]["ruleId"], "C001");
        let report: serde_json::Value =
            serde_json::from_str(&render(Format::Json, &d, &no_source)).unwrap();
        assert_eq!(report["diagnostics"][0]["schema"], DIAG_SCHEMA);
        assert_eq!(report["diagnostics"][0]["severity"], "error");
        assert_eq!(report["diagnostics"][0]["location"]["kind"], "source");
    }

    // frob:tests crates/cadetta-cli/src/output.rs::render
    #[test]
    fn waived_diagnostics_appear_in_json_only() {
        let mut d = sample();
        d.waived = Some(Waived {
            waiver: "w".into(),
            because: "b".into(),
        });
        assert_eq!(render(Format::Short, &[d.clone()], &no_source), "");
        assert!(render(Format::Json, &[d], &no_source).contains("\"because\":\"b\""));
    }

    // frob:tests crates/cadetta-cli/src/output.rs::sort_diagnostics
    #[test]
    fn diagnostics_sort_by_file_line_code() {
        let mut a = sample();
        a.code = "Z1".into();
        let mut b = sample();
        b.code = "A1".into();
        let mut v = vec![a, b];
        sort_diagnostics(&mut v);
        assert_eq!(v[0].code, "A1");
    }
}
