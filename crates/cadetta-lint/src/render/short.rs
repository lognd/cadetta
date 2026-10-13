//! The one-line renderer (lints.md 1.2): `file:line:col: severity[CODE]: message`.
//!
//! Pure: it returns a `String`; a diagnostic with no location has no file
//! prefix and renders as `severity[CODE]: message`.

use super::text::{file_pos, severity_word};
use crate::diag::Diagnostic;

/// Render one diagnostic as a single newline-terminated line.
pub fn render(d: &Diagnostic) -> String {
    tracing::trace!(code = %d.code, "render: short");
    let head = format!("{}[{}]: {}", severity_word(d.severity), d.code, d.message);
    match d.location.as_ref().map(file_pos) {
        Some((file, line, col)) => format!("{file}:{line}:{col}: {head}\n"),
        None => format!("{head}\n"),
    }
}

/// Render a run of diagnostics in the order given, one line each.
pub fn render_all(diags: &[Diagnostic]) -> String {
    tracing::debug!(count = diags.len(), "render: short run");
    diags.iter().map(render).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::{Location, Position, Severity};

    // frob:ticket 01M4SWKA1BZWA1VDN67RQA2FJR
    // frob:tests crates/cadetta-lint/src/render/short.rs::render kind=unit
    #[test]
    fn one_line_with_location() {
        let mut d = Diagnostic::new("C001", Severity::Error, "false for every value");
        d.location = Some(Location::Source {
            file: "cad/rover.cdt".into(),
            start: Position { line: 41, col: 3 },
            end: Position { line: 41, col: 25 },
        });
        assert_eq!(
            render(&d),
            "cad/rover.cdt:41:3: error[C001]: false for every value\n"
        );
        d.location = Some(Location::Lock { line: 7 });
        assert_eq!(
            render(&d),
            "cadetta.lock:7:1: error[C001]: false for every value\n"
        );
    }

    // frob:ticket 01M4SWKA1BZWA1VDN67RQA2FJR
    // frob:tests crates/cadetta-lint/src/render/short.rs::render_all kind=unit
    #[test]
    fn unlocated_and_run() {
        let d = Diagnostic::new("N001", Severity::Hint, "m");
        assert_eq!(render(&d), "hint[N001]: m\n");
        assert_eq!(render_all(&[d.clone(), d]).lines().count(), 2);
    }
}
