//! Source locations (language.md 3): byte spans resolved to `file:line:col-line:col`.
//!
//! Every AST node carries a byte [`Span`]; a [`SourceFile`] turns it into a
//! human-facing [`Location`]. Lines and columns are 1-based; a column counts
//! Unicode scalar values, so a multi-byte character is one column.

// frob:ticket 01M4SPWP7B9ANMSNWB9SYD2T0Q

use std::fmt;

pub use crate::lexer::Span;

use crate::ast;

/// A 1-based line and column.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct LineCol {
    /// 1-based line.
    pub line: usize,
    /// 1-based column, in characters.
    pub col: usize,
}

/// A resolved span: `file:line:col-line:col`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Location {
    /// The file name as given to [`SourceFile::new`].
    pub file: String,
    /// Where the span starts.
    pub start: LineCol,
    /// Where the span ends (exclusive).
    pub end: LineCol,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}-{}:{}",
            self.file, self.start.line, self.start.col, self.end.line, self.end.col
        )
    }
}

/// A source text with a line index, for resolving byte offsets.
#[derive(Clone, Debug)]
pub struct SourceFile {
    name: String,
    src: String,
    /// Byte offset of the start of each line.
    starts: Vec<usize>,
}

impl SourceFile {
    /// Index `src`, reported under `name`.
    pub fn new(name: impl Into<String>, src: impl Into<String>) -> Self {
        let (name, src) = (name.into(), src.into());
        let mut starts = vec![0];
        starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
        tracing::trace!(file = %name, lines = starts.len(), "indexed source file");
        Self { name, src, starts }
    }

    /// The indexed text.
    pub fn text(&self) -> &str {
        &self.src
    }

    /// The line and column of byte `offset` (clamped to the end of the text).
    pub fn line_col(&self, offset: usize) -> LineCol {
        let mut offset = offset.min(self.src.len());
        while !self.src.is_char_boundary(offset) {
            offset -= 1;
        }
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let col = self.src[self.starts[line]..offset].chars().count() + 1;
        LineCol {
            line: line + 1,
            col,
        }
    }

    /// Resolve `span` to a [`Location`].
    pub fn locate(&self, span: Span) -> Location {
        Location {
            file: self.name.clone(),
            start: self.line_col(span.start),
            end: self.line_col(span.end),
        }
    }
}

/// A syntax-tree node that knows where it came from.
pub trait Spanned {
    /// The node's byte span.
    fn span(&self) -> Span;
}

impl Spanned for Span {
    fn span(&self) -> Span {
        *self
    }
}

macro_rules! spanned {
    ($($t:ty),* $(,)?) => {
        $(impl Spanned for $t {
            fn span(&self) -> Span {
                self.span
            }
        })*
    };
}

spanned!(
    ast::Ident,
    ast::Path,
    ast::Doc,
    ast::Attr,
    ast::Citation,
    ast::File,
    ast::Use,
    ast::UsePath,
    ast::UseTree,
    ast::Item,
    ast::InterfaceMember,
    ast::ParamDecl,
    ast::Member,
    ast::Let,
    ast::Bound,
    ast::Ctor,
    ast::Arg,
    ast::Type,
    ast::Expr,
    ast::MatchArm,
    ast::Block,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::MemberKind;
    use crate::parser::parse;

    // frob:tests crates/cadetta-syntax/src/span.rs::SourceFile.locate kind=unit
    #[test]
    fn locations_are_one_based_and_display() {
        let f = SourceFile::new("a.cdt", "ab\ncd\n");
        assert_eq!(f.line_col(0), LineCol { line: 1, col: 1 });
        assert_eq!(f.line_col(4), LineCol { line: 2, col: 2 });
        assert_eq!(f.locate(Span::new(1, 4)).to_string(), "a.cdt:1:2-2:2");
        assert_eq!(f.line_col(999), LineCol { line: 3, col: 1 });
    }

    // frob:tests crates/cadetta-syntax/src/span.rs::SourceFile.line_col kind=unit
    #[test]
    fn columns_count_characters() {
        let f = SourceFile::new("u.cdt", "\u{3bc}m x");
        assert_eq!(f.line_col(2), LineCol { line: 1, col: 2 });
        // A mid-character offset snaps back to the character start.
        assert_eq!(f.line_col(1), LineCol { line: 1, col: 1 });
    }

    // frob:tests crates/cadetta-syntax/src/span.rs::Spanned kind=unit
    #[test]
    fn every_node_has_a_location() {
        let src = "model A {\n  a = 12mm [+0.1mm -0]\n}\n";
        let sf = SourceFile::new("m.cdt", src);
        let p = parse(src);
        assert!(p.diags.is_empty());
        let item = p.file.items().next().expect("item");
        let il = sf.locate(item.span());
        assert_eq!(il.start, LineCol { line: 1, col: 1 });
        assert!(il.end.line >= 3);
        let comp = p.file.components().next().expect("component");
        let m = &comp.members[0];
        assert_eq!(sf.locate(m.span()).start, LineCol { line: 2, col: 3 });
        let MemberKind::Param(d) = &m.kind else {
            panic!("param")
        };
        let loc = sf.locate(d.value.span());
        assert_eq!(loc.start, LineCol { line: 2, col: 7 });
        assert_eq!(loc.end.line, 2);
        assert!(d.name.span().end <= d.value.span().start);
    }

    // frob:tests crates/cadetta-syntax/src/span.rs::SourceFile.locate kind=unit
    #[test]
    fn two_errors_reported_with_spans() {
        let src = "model A {\n a = = 1\n b = 2\n}\nmodel B {\n c = = 3\n}\n";
        let sf = SourceFile::new("e.cdt", src);
        let p = parse(src);
        assert_eq!(p.diags.len(), 2);
        let lines: Vec<usize> = p
            .diags
            .iter()
            .map(|d| sf.locate(d.span).start.line)
            .collect();
        assert_eq!(lines, vec![2, 6]);
    }
}
