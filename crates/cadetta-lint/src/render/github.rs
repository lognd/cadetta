//! The GitHub workflow-annotation renderer (lints.md 1.2).
//!
//! Pure: it returns a `String`. Severity maps error -> `error`, warning ->
//! `warning`, info and hint -> `notice`. Escaping follows the workflow
//! command rules so a message cannot inject further commands.

use crate::diag::{Diagnostic, Location, Severity};

/// The annotation level for a severity.
fn level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info | Severity::Hint => "notice",
    }
}

/// Escape a command message (the text after `::`).
fn escape_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escape a command property value.
fn escape_prop(s: &str) -> String {
    escape_data(s).replace(':', "%3A").replace(',', "%2C")
}

/// Render one diagnostic as one `::level file=..,line=..,title=CODE::message` line.
pub fn render(d: &Diagnostic) -> String {
    tracing::trace!(code = %d.code, "render: github");
    let mut props: Vec<String> = Vec::new();
    match &d.location {
        Some(Location::Source { file, start, end }) => {
            props.push(format!("file={}", escape_prop(file)));
            props.push(format!("line={}", start.line));
            props.push(format!("col={}", start.col));
            props.push(format!("endLine={}", end.line));
            props.push(format!("endColumn={}", end.col));
        }
        Some(Location::Native { file, .. }) => {
            props.push(format!("file={}", escape_prop(file)));
            props.push("line=1".to_owned());
        }
        Some(Location::Lock { line }) => {
            props.push("file=cadetta.lock".to_owned());
            props.push(format!("line={line}"));
        }
        None => {}
    }
    props.push(format!("title={}", escape_prop(&d.code)));
    format!(
        "::{} {}::{}\n",
        level(d.severity),
        props.join(","),
        escape_data(&d.message)
    )
}

/// Render a run of diagnostics in the order given, one annotation each.
pub fn render_all(diags: &[Diagnostic]) -> String {
    tracing::debug!(count = diags.len(), "render: github run");
    diags.iter().map(render).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Position;

    // frob:ticket 01M4SWKA1BZWA1VDN67RQA2FJR
    // frob:tests crates/cadetta-lint/src/render/github.rs::render kind=unit
    #[test]
    fn level_mapping_and_location() {
        let mut d = Diagnostic::new("C001", Severity::Error, "bad");
        d.location = Some(Location::Source {
            file: "cad/rover.cdt".into(),
            start: Position { line: 41, col: 3 },
            end: Position { line: 41, col: 25 },
        });
        assert_eq!(
            render(&d),
            "::error file=cad/rover.cdt,line=41,col=3,endLine=41,endColumn=25,title=C001::bad\n"
        );
        for (sev, want) in [
            (Severity::Warning, "warning"),
            (Severity::Info, "notice"),
            (Severity::Hint, "notice"),
        ] {
            d.severity = sev;
            assert!(render(&d).starts_with(&format!("::{want} ")));
        }
    }

    // frob:ticket 01M4SWKA1BZWA1VDN67RQA2FJR
    // frob:tests crates/cadetta-lint/src/render/github.rs::render_all kind=unit
    #[test]
    fn escaping_and_unlocated() {
        let d = Diagnostic::new("N001", Severity::Hint, "100%\nnext::cmd");
        assert_eq!(render(&d), "::notice title=N001::100%25%0Anext::cmd\n");
        assert_eq!(render_all(&[d.clone(), d]).lines().count(), 2);
        let mut l = Diagnostic::new("O001", Severity::Warning, "stale");
        l.location = Some(Location::Lock { line: 7 });
        assert!(render(&l).contains("file=cadetta.lock,line=7"));
    }
}
