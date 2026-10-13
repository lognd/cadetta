//! The rustc-style text renderer (lints.md 1.2, config-cli.md 5).
//!
//! Pure: it returns a `String`; the CLI output layer owns all writing.
//! Output is ASCII and never depends on whether a terminal is attached.

use std::fmt::Write as _;

use crate::diag::{Diagnostic, Location, Severity};

/// The lower-case severity word used in the header line.
fn severity_word(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Hint => "hint",
    }
}

/// The file and 1-based line/column a location points at, if it has one.
fn file_pos(loc: &Location) -> (&str, u32, u32) {
    match loc {
        Location::Source { file, start, .. } => (file, start.line, start.col),
        Location::Native { file, .. } => (file, 1, 1),
        Location::Lock { line } => ("cadetta.lock", *line, 1),
    }
}

/// Render one diagnostic as rustc-style text, ending in a blank line.
///
/// `source_line(file, line)` supplies the text of a 1-based source line; when
/// it returns `None` the excerpt is omitted and only `-->` is shown.
pub fn render(d: &Diagnostic, source_line: &dyn Fn(&str, u32) -> Option<String>) -> String {
    tracing::trace!(code = %d.code, "render: text");
    let mut out = format!("{}[{}]: {}\n", severity_word(d.severity), d.code, d.message);
    let located = d.location.as_ref().map(file_pos);
    let width = located.map_or(1, |(_, line, _)| line.to_string().len());
    let pad = " ".repeat(width);
    if let Some((file, line, col)) = located {
        let _ = writeln!(out, "{pad}--> {file}:{line}:{col}");
        match source_line(file, line) {
            Some(text) => {
                let text = text.trim_end_matches(['\r', '\n']);
                let line_len = text.chars().count() as u32;
                let span = match &d.location {
                    Some(Location::Source { start, end, .. }) if end.line == start.line => {
                        end.col.saturating_sub(start.col).max(1)
                    }
                    // Multi-line span: underline to the end of the first line.
                    Some(Location::Source { start, .. }) => {
                        (line_len + 1).saturating_sub(start.col).max(1)
                    }
                    _ => 1,
                };
                let label = d
                    .alt
                    .as_ref()
                    .map_or(String::new(), |a| format!(" under {a}"));
                let _ = writeln!(out, "{pad} |");
                let _ = writeln!(out, "{line} | {text}");
                let _ = writeln!(
                    out,
                    "{pad} | {}{}{label}",
                    " ".repeat(col.saturating_sub(1) as usize),
                    "^".repeat(span as usize),
                );
                let _ = writeln!(out, "{pad} |");
            }
            None => tracing::debug!(file, line, "render: no source line, excerpt omitted"),
        }
    }
    for note in &d.notes {
        match note.location.as_ref().map(file_pos) {
            Some((file, line, col)) => {
                let _ = writeln!(out, "{pad} = note: {} ({file}:{line}:{col})", note.message);
            }
            None => {
                let _ = writeln!(out, "{pad} = note: {}", note.message);
            }
        }
    }
    for fix in &d.fixes {
        let _ = writeln!(out, "{pad} = help: {}", fix.title);
    }
    if let Some(path) = &d.path {
        let _ = writeln!(out, "{pad} = path: {path}");
    }
    out.push('\n');
    out
}

/// Render a run of diagnostics in the order given, concatenated.
pub fn render_all(
    diags: &[Diagnostic],
    source_line: &dyn Fn(&str, u32) -> Option<String>,
) -> String {
    tracing::debug!(count = diags.len(), "render: text run");
    diags.iter().map(|d| render(d, source_line)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::{Edit, Fix, Note, Position, Range};

    fn lines(_: &str, line: u32) -> Option<String> {
        (line == 41).then(|| "  check wheel.mass < 1kg".to_owned())
    }

    fn sample() -> Diagnostic {
        let mut d = Diagnostic::new(
            "C001",
            Severity::Error,
            "false for every value in tolerance",
        );
        d.path = Some("acme.rover::Rover.load_ok".into());
        d.alt = Some("wheel=Colson2Series".into());
        d.location = Some(Location::Source {
            file: "cad/rover.cdt".into(),
            start: Position { line: 41, col: 3 },
            end: Position { line: 41, col: 25 },
        });
        d.notes.push(Note {
            message: "wheel.mass = 1.2kg".into(),
            location: None,
        });
        d.fixes.push(Fix {
            title: "use the instance".into(),
            edits: vec![Edit {
                file: "cad/rover.cdt".into(),
                range: Range::default(),
                text: "x".into(),
            }],
        });
        d
    }

    // frob:ticket 01M4SWK9Y67QRHQ6K0FMZMDBM4
    // frob:tests crates/cadetta-lint/src/render/text.rs::render kind=unit
    #[test]
    fn matches_spec_example() {
        let want = "\
error[C001]: false for every value in tolerance
  --> cad/rover.cdt:41:3
   |
41 |   check wheel.mass < 1kg
   |   ^^^^^^^^^^^^^^^^^^^^^^ under wheel=Colson2Series
   |
   = note: wheel.mass = 1.2kg
   = help: use the instance
   = path: acme.rover::Rover.load_ok

";
        assert_eq!(render(&sample(), &lines), want);
    }

    // frob:ticket 01M4SWK9Y67QRHQ6K0FMZMDBM4
    // frob:tests crates/cadetta-lint/src/render/text.rs::render kind=unit
    #[test]
    fn default_is_plain_ascii_text() {
        // No terminal detection exists: output is plain ASCII, no escapes.
        let out = render(&sample(), &lines);
        assert!(out.is_ascii());
        assert!(!out.contains('\x1b'));
        let bare = render(&Diagnostic::new("N001", Severity::Hint, "m"), &lines);
        assert_eq!(bare, "hint[N001]: m\n\n");
        let mut no_src = sample();
        no_src.notes.clear();
        let out = render(&no_src, &|_: &str, _: u32| None);
        assert!(out.contains("--> cad/rover.cdt:41:3"));
        assert!(!out.contains("^"));
    }

    // frob:ticket 01M4SWK9Y67QRHQ6K0FMZMDBM4
    // frob:tests crates/cadetta-lint/src/render/text.rs::render_all kind=unit
    #[test]
    fn lock_location_and_run() {
        let mut d = Diagnostic::new("O001", Severity::Warning, "stale");
        d.location = Some(Location::Lock { line: 7 });
        let out = render_all(&[d.clone(), d], &|_: &str, _: u32| None);
        assert_eq!(out.matches("--> cadetta.lock:7:1").count(), 2);
    }
}
