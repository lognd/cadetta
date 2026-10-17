//! frob:ticket 01M4VK7W0KGNQZ0JBXX0CXQ6EZ
//! Editor queries of `cadetta lsp`: hover as `why`, code lens as `impact` and go to
//! definition along canonical paths (config-cli.md 3.8). Like diagnostics, they are answered
//! from the package as last checked on disk and add no rule of their own.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::{
    CodeLens, Command, Hover, HoverContents, Location as LspLocation, MarkupContent, MarkupKind,
    Position, Range as LspRange, Url,
};
use serde_json::json;

use cadetta_lint::diag::Location;

use super::impact::downstream;
use super::lsp::{Encoding, to_lsp_position};
use super::why::{render as render_why, trace};
use crate::output::Format;
use crate::pipeline::Analysis;
use crate::project::Project;

/// The client command a code lens names; arguments are `[path]`.
pub const IMPACT_COMMAND: &str = "cadetta.impact";

/// One member (checks carry no span in the IR) of a checked package, with its precomputed answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Canonical path.
    pub path: String,
    /// Canonical paths it reads.
    pub deps: Vec<String>,
    /// Start of the declaration, 1-based line and character column.
    pub start: (u32, u32),
    /// End of the declaration (exclusive), 1-based line and character column.
    pub end: (u32, u32),
    /// The `cadetta why` text for the path.
    pub why: String,
    /// How many nodes lie downstream (the `cadetta impact` count).
    pub dependents: usize,
}

/// The entries of one package by absolute file.
pub type FileEntries = HashMap<PathBuf, Vec<Entry>>;

/// Build the query index of an analysed project; empty when analysis halted.
pub fn build_index(project: &Project, analysis: &Analysis) -> FileEntries {
    let mut files = FileEntries::new();
    if analysis.halted {
        tracing::info!("lsp: analysis halted, no query index");
        return files;
    }
    for package in &analysis.packages {
        let ir = &package.ir;
        let members = ir
            .components
            .iter()
            .flat_map(|c| &c.members)
            .map(|m| (&m.path, &m.deps, &m.span));
        for (path, deps, span) in members {
            let Some(span) = span else { continue };
            let Location::Source {
                file, start, end, ..
            } = analysis
                .sources
                .locate_ref(&span.file, span.start, span.end)
            else {
                continue;
            };
            let path = path.to_string();
            let mut why = Vec::new();
            if let Ok(node) = trace(ir, &path) {
                render_why(&node, Format::Text, &mut why);
            }
            let dependents = downstream(ir, &path).map_or(0, |i| i.dependents.len());
            files
                .entry(project.root.join(file))
                .or_default()
                .push(Entry {
                    deps: deps.iter().map(ToString::to_string).collect(),
                    path,
                    start: (start.line, start.col),
                    end: (end.line, end.col),
                    why: String::from_utf8_lossy(&why).into_owned(),
                    dependents,
                });
        }
    }
    tracing::info!(root = %project.root.display(), entries = files.values().map(Vec::len).sum::<usize>(), "lsp: query index built");
    files
}

/// The impact text of `path` for a client command, from the index entry.
fn impact_title(e: &Entry) -> String {
    match e.dependents {
        0 => "impact: no dependents".to_owned(),
        1 => "impact: 1 dependent".to_owned(),
        n => format!("impact: {n} dependents"),
    }
}

/// The LSP range of an entry in `text`.
fn range_of(e: &Entry, text: &str, enc: Encoding) -> LspRange {
    LspRange::new(
        to_lsp_position(text, e.start.0, e.start.1, enc),
        to_lsp_position(text, e.end.0, e.end.1, enc),
    )
}

/// Compare positions as `(line, character)`.
fn key(p: Position) -> (u32, u32) {
    (p.line, p.character)
}

/// The innermost entry whose declaration contains `pos`.
fn enclosing<'a>(
    entries: &'a [Entry],
    text: &str,
    pos: Position,
    enc: Encoding,
) -> Option<&'a Entry> {
    entries
        .iter()
        .filter(|e| {
            let r = range_of(e, text, enc);
            key(r.start) <= key(pos) && key(pos) < key(r.end)
        })
        .min_by_key(|e| {
            let r = range_of(e, text, enc);
            (r.end.line - r.start.line, r.end.character)
        })
}

/// Hover: the `why` chain of the declaration under `pos`.
pub fn hover(entries: &[Entry], text: &str, pos: Position, enc: Encoding) -> Option<Hover> {
    let e = enclosing(entries, text, pos, enc)?;
    tracing::debug!(path = %e.path, "lsp: hover");
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("why `{}`\n\n```\n{}```", e.path, e.why),
        }),
        range: Some(range_of(e, text, enc)),
    })
}

/// Code lenses: one `impact` lens per declaration of the file.
pub fn lenses(entries: &[Entry], text: &str, enc: Encoding) -> Vec<CodeLens> {
    entries
        .iter()
        .map(|e| {
            let at = range_of(e, text, enc).start;
            CodeLens {
                range: LspRange::new(at, at),
                command: Some(Command {
                    title: impact_title(e),
                    command: IMPACT_COMMAND.to_owned(),
                    arguments: Some(vec![json!(e.path)]),
                }),
                data: None,
            }
        })
        .collect()
}

/// The canonical-path token (letters, digits, `_`, `:` and `.`) around `pos`.
fn token_at(text: &str, pos: Position, enc: Encoding) -> Option<String> {
    let line = text.split('\n').nth(pos.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let mut at = 0usize;
    let mut units = 0u32;
    while at < chars.len() && units < pos.character {
        units += enc.units(chars[at]);
        at += 1;
    }
    let part = |c: &char| c.is_alphanumeric() || matches!(c, '_' | ':' | '.');
    let mut lo = at;
    while lo > 0 && part(&chars[lo - 1]) {
        lo -= 1;
    }
    let mut hi = at;
    while hi < chars.len() && part(&chars[hi]) {
        hi += 1;
    }
    let token: String = chars[lo..hi].iter().collect();
    let token = token.trim_matches(['.', ':']).to_owned();
    (!token.is_empty()).then_some(token)
}

/// The last segment of a canonical path (after `::` or `.`).
fn last_segment(path: &str) -> &str {
    path.rsplit([':', '.']).next().unwrap_or(path)
}

/// Go to definition: the declaration of the path under `pos`, either a full canonical path
/// or a name the enclosing declaration reads. `all` is every indexed entry with its file.
pub fn definition(
    all: &[(&Path, &Entry)],
    here: &[Entry],
    text: &str,
    pos: Position,
    enc: Encoding,
    read: impl Fn(&Path) -> String,
) -> Option<LspLocation> {
    let token = token_at(text, pos, enc)?;
    let target = all.iter().find(|(_, e)| e.path == token).or_else(|| {
        let from = enclosing(here, text, pos, enc)?;
        let word = last_segment(&token);
        let dep = from
            .deps
            .iter()
            .find(|d| *d == &token || last_segment(d) == word)?;
        all.iter().find(|(_, e)| &e.path == dep)
    });
    let Some((file, entry)) = target else {
        tracing::debug!(%token, "lsp: definition not found");
        return None;
    };
    let uri = Url::from_file_path(file).ok()?;
    tracing::debug!(%token, path = %entry.path, "lsp: definition");
    Some(LspLocation::new(uri, range_of(entry, &read(file), enc)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests crates/cadetta-cli/src/cmd/lsp_query.rs::token_at
    #[test]
    fn token_spans_path_characters() {
        let text = "  let w = a::B.c + d\n";
        let at = |c| token_at(text, Position::new(0, c), Encoding::Utf16);
        assert_eq!(at(10).as_deref(), Some("a::B.c"));
        assert_eq!(at(19).as_deref(), Some("d"));
        assert_eq!(at(14), Some("a::B.c".to_owned()));
        assert_eq!(token_at("  \n", Position::new(0, 1), Encoding::Utf16), None);
    }

    // frob:tests crates/cadetta-cli/src/cmd/lsp_query.rs::lenses
    #[test]
    fn lens_title_counts_dependents() {
        let e = Entry {
            path: "p::M.a".into(),
            deps: Vec::new(),
            start: (1, 1),
            end: (1, 5),
            why: String::new(),
            dependents: 0,
        };
        let l = lenses(std::slice::from_ref(&e), "abcd\n", Encoding::Utf16);
        assert_eq!(
            l[0].command.as_ref().unwrap().title,
            "impact: no dependents"
        );
        let e = Entry { dependents: 3, ..e };
        assert_eq!(impact_title(&e), "impact: 3 dependents");
    }
}
