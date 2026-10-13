//! The hand-written lexer (language.md 2, quantities.md 2): tokens, comments and diagnostics.
//!
//! Quantity literals are single tokens and stay exact: numbers are kept as
//! source text (`"1-1/2"`, `"1e-3"`) for sema to turn into rationals, so no
//! float is ever built here.

// frob:ticket 01M4SPWNYX632H3Q6WEGN0SM0R

use std::fmt;

/// A half-open byte range of one source file.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Default)]
pub struct Span {
    /// First byte.
    pub start: usize,
    /// One past the last byte.
    pub end: usize,
}

impl Span {
    /// A span from `start` to `end` (clamped so `end >= start`).
    pub const fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end: if end < start { start } else { end },
        }
    }

    /// The text of `src` this span covers (empty when out of range).
    pub fn slice(self, src: &str) -> &str {
        src.get(self.start..self.end).unwrap_or("")
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A suggested edit: replace `span` with `replacement`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fix {
    /// The bytes to replace.
    pub span: Span,
    /// The replacement text (empty deletes).
    pub replacement: String,
}

/// What kind of lexical problem a [`Diagnostic`] reports.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiagCode {
    /// E001 syntax-error, lexical flavour (bad character, malformed literal).
    E001,
    /// Q007 quantity-literal-split: whitespace between number and unit.
    Q007,
}

impl DiagCode {
    /// The catalog code, as in `docs/spec/lints.md`.
    pub const fn code(self) -> &'static str {
        match self {
            Self::E001 => "E001",
            Self::Q007 => "Q007",
        }
    }
}

/// A problem found while lexing.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Diagnostic {
    /// The catalog code.
    pub code: DiagCode,
    /// Where, in byte offsets.
    pub span: Span,
    /// One sentence, no trailing period.
    pub message: String,
    /// A suggested edit, when there is one.
    pub fix: Option<Fix>,
}

/// Punctuation tokens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Punct {
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `;`
    Semi,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `::`
    ColonColon,
    /// `.`
    Dot,
    /// `..`
    DotDot,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `^`
    Caret,
    /// `?`
    Question,
    /// `%` standing alone (the percent unit in `#[display(%)]`)
    Percent,
    /// `@` (citation)
    At,
    /// `->`
    Arrow,
    /// `=>`
    FatArrow,
    /// `#[` (attribute)
    HashBracket,
    /// `#![` (inner attribute)
    HashBangBracket,
}

impl Punct {
    /// The spelling of the token.
    pub const fn text(self) -> &'static str {
        match self {
            Self::LBrace => "{",
            Self::RBrace => "}",
            Self::LParen => "(",
            Self::RParen => ")",
            Self::LBracket => "[",
            Self::RBracket => "]",
            Self::Semi => ";",
            Self::Comma => ",",
            Self::Colon => ":",
            Self::ColonColon => "::",
            Self::Dot => ".",
            Self::DotDot => "..",
            Self::Eq => "=",
            Self::EqEq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Caret => "^",
            Self::Question => "?",
            Self::Percent => "%",
            Self::At => "@",
            Self::Arrow => "->",
            Self::FatArrow => "=>",
            Self::HashBracket => "#[",
            Self::HashBangBracket => "#![",
        }
    }
}

/// Reserved keywords (language.md 2), in every position.
pub const KEYWORDS: &[&str] = &[
    "pub",
    "priv",
    "package",
    "super",
    "use",
    "as",
    "cite",
    "interface",
    "ots",
    "model",
    "assembly",
    "process",
    "table",
    "enum",
    "fn",
    "let",
    "if",
    "else",
    "match",
    "part",
    "alt",
    "fit",
    "on",
    "solver",
    "in",
    "out",
    "deps",
    "require",
    "check",
    "for",
    "satisfies",
    "waive",
    "because",
    "expires",
    "was",
    "unit",
    "kind",
    "self",
    "true",
    "false",
    "datum",
    "place",
    "rule",
    "key",
    "cols",
    "origin",
    "and",
    "or",
    "not",
];

/// The keyword spelled `word`, if it is reserved.
pub fn keyword(word: &str) -> Option<&'static str> {
    KEYWORDS.iter().copied().find(|k| *k == word)
}

/// One number-unit segment of a quantity literal.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Segment {
    /// The number, as written (`12`, `12.5`, `1e-3`, `3/4`, `1-1/2`).
    pub number: String,
    /// The unit lexeme; the quote aliases are normalised (`"` is `in`, `'` is `ft`).
    pub unit: String,
}

/// A token kind.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Tok {
    /// An identifier.
    Ident(String),
    /// A reserved keyword.
    Keyword(&'static str),
    /// A bare number, as written.
    Number(String),
    /// A quantity literal: one segment, or several for a compound literal.
    Quantity(Vec<Segment>),
    /// A string with escapes resolved.
    Str(String),
    /// Punctuation.
    P(Punct),
    /// A line break (consecutive ones collapse); the parser decides if it ends a statement.
    Newline,
    /// End of input.
    Eof,
    /// A lexical error already reported; skipped by the parser.
    Bad,
}

/// A token with its span.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Token {
    /// The kind.
    pub tok: Tok,
    /// Where it is.
    pub span: Span,
}

/// The two comment forms (language.md 2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommentKind {
    /// `// text`
    Line,
    /// `/// text`, attached to the next item.
    Doc,
}

/// A comment, kept verbatim.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Comment {
    /// Which form.
    pub kind: CommentKind,
    /// The full lexeme including the slashes, without the newline.
    pub text: String,
    /// Where it is.
    pub span: Span,
}

impl Comment {
    /// The prose of a doc comment: after `///`, one leading space removed.
    pub fn doc_text(&self) -> &str {
        let t = self.text.strip_prefix("///").unwrap_or(&self.text);
        t.strip_prefix(' ').unwrap_or(t)
    }
}

/// The output of [`lex`].
#[derive(Clone, Debug, Default)]
pub struct Lexed {
    /// Tokens, ending with [`Tok::Eof`].
    pub tokens: Vec<Token>,
    /// Comments in source order (never in the token stream).
    pub comments: Vec<Comment>,
    /// Lexical diagnostics.
    pub diags: Vec<Diagnostic>,
}

/// Dimensions that allow compound literals.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CompoundDim {
    Length,
    Time,
    Angle,
    Mass,
}

/// Units usable in compound literals with a size in a common base per
/// dimension (nm, ms, arcsec, ug); larger means a bigger unit.
const COMPOUND_UNITS: &[(&str, CompoundDim, u64)] = &[
    ("km", CompoundDim::Length, 1_000_000_000_000),
    ("m", CompoundDim::Length, 1_000_000_000),
    ("yd", CompoundDim::Length, 914_400_000),
    ("ft", CompoundDim::Length, 304_800_000),
    ("cm", CompoundDim::Length, 10_000_000),
    ("in", CompoundDim::Length, 25_400_000),
    ("mm", CompoundDim::Length, 1_000_000),
    ("mil", CompoundDim::Length, 25_400),
    ("thou", CompoundDim::Length, 25_400),
    ("um", CompoundDim::Length, 1_000),
    ("nm", CompoundDim::Length, 1),
    ("wk", CompoundDim::Time, 604_800_000),
    ("d", CompoundDim::Time, 86_400_000),
    ("h", CompoundDim::Time, 3_600_000),
    ("min", CompoundDim::Time, 60_000),
    ("s", CompoundDim::Time, 1_000),
    ("ms", CompoundDim::Time, 1),
    ("rev", CompoundDim::Angle, 1_296_000),
    ("deg", CompoundDim::Angle, 3_600),
    ("arcmin", CompoundDim::Angle, 60),
    ("arcsec", CompoundDim::Angle, 1),
    ("kg", CompoundDim::Mass, 1_000_000_000),
    ("lb", CompoundDim::Mass, 453_592_370),
    ("oz", CompoundDim::Mass, 28_349_523),
    ("g", CompoundDim::Mass, 1_000_000),
    ("mg", CompoundDim::Mass, 1_000),
];

/// Built-in unit atoms (quantities.md 3) that Q007 recognises after whitespace.
const BUILTIN_UNIT_ATOMS: &[&str] = &[
    "m", "mm", "cm", "um", "nm", "km", "in", "ft", "yd", "mil", "thou", "rad", "deg", "arcmin",
    "arcsec", "rev", "L", "mL", "kg", "g", "mg", "lb", "oz", "N", "kN", "lbf", "J", "kJ", "Wh",
    "W", "kW", "hp", "Pa", "kPa", "MPa", "GPa", "psi", "ksi", "bar", "mmH2O", "inH2O", "s", "ms",
    "min", "h", "d", "wk", "Hz", "kHz", "cfm", "gn", "rpm", "A", "mA", "V", "mV", "ohm", "kohm",
    "K", "degC", "degF", "dK", "ddegC", "ddegF", "USD", "EUR", "GBP", "CNY", "JPY",
];

fn compound_info(unit: &str) -> Option<(CompoundDim, u64)> {
    COMPOUND_UNITS
        .iter()
        .find(|(u, _, _)| *u == unit)
        .map(|(_, d, s)| (*d, *s))
}

struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    out: Lexed,
}

/// Lexes `src` into tokens, comments and diagnostics; never fails, errors become `Bad` tokens.
pub fn lex(src: &str) -> Lexed {
    let mut lx = Lexer {
        src,
        bytes: src.as_bytes(),
        pos: 0,
        out: Lexed::default(),
    };
    lx.run();
    tracing::debug!(
        tokens = lx.out.tokens.len(),
        comments = lx.out.comments.len(),
        diags = lx.out.diags.len(),
        "lexed"
    );
    lx.out
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_cont(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The outcome of reading one unit lexeme.
struct UnitRead {
    unit: String,
    /// A single atom or quote alias with no operator or exponent.
    simple: bool,
}

impl Lexer<'_> {
    fn peek(&self, off: usize) -> Option<u8> {
        self.bytes.get(self.pos + off).copied()
    }

    fn push(&mut self, tok: Tok, start: usize) {
        self.out.tokens.push(Token {
            tok,
            span: Span::new(start, self.pos),
        });
    }

    fn diag(&mut self, code: DiagCode, span: Span, msg: impl Into<String>, fix: Option<Fix>) {
        let message = msg.into();
        tracing::debug!(code = code.code(), %span, %message, "lex diagnostic");
        self.out.diags.push(Diagnostic {
            code,
            span,
            message,
            fix,
        });
    }

    /// Reports a lexical error over `start..pos` and emits a `Bad` token.
    fn error(&mut self, start: usize, msg: impl Into<String>) {
        let span = Span::new(start, self.pos);
        self.diag(DiagCode::E001, span, msg, None);
        self.out.tokens.push(Token {
            tok: Tok::Bad,
            span,
        });
    }

    /// Skips the rest of a malformed literal (identifier characters and a closing quote).
    fn recover(&mut self) {
        while self.peek(0).is_some_and(is_ident_cont) {
            self.pos += 1;
        }
        if matches!(self.peek(0), Some(b'"' | b'\'')) {
            self.pos += 1;
        }
    }

    /// True when the next line starts with a binary operator (`+ * / ^`), which continues the
    /// statement instead of starting one (language.md 2); `//` starts a comment, not a `/`.
    fn continues_on_next_line(&self) -> bool {
        let mut i = 0;
        while matches!(self.peek(i), Some(b' ' | b'\t')) {
            i += 1;
        }
        match self.peek(i) {
            Some(b'+' | b'*' | b'^') => true,
            Some(b'/') => self.peek(i + 1) != Some(b'/'),
            _ => false,
        }
    }

    fn run(&mut self) {
        while let Some(b) = self.peek(0) {
            match b {
                b' ' | b'\t' | b'\r' => self.pos += 1,
                b'\n' => {
                    let start = self.pos;
                    self.pos += 1;
                    if self.continues_on_next_line() {
                        continue;
                    }
                    if !matches!(
                        self.out.tokens.last(),
                        Some(Token {
                            tok: Tok::Newline,
                            ..
                        })
                    ) {
                        self.push(Tok::Newline, start);
                    }
                }
                b'/' if self.peek(1) == Some(b'/') => self.line_comment(),
                b'"' => self.string(),
                b'.' if self.peek(1).is_some_and(|c| c.is_ascii_digit()) => self.leading_dot(),
                b if b.is_ascii_digit() => self.number(),
                b if is_ident_start(b) => self.ident(),
                _ => self.punct(),
            }
        }
        let end = self.src.len();
        self.out.tokens.push(Token {
            tok: Tok::Eof,
            span: Span::new(end, end),
        });
    }

    fn line_comment(&mut self) {
        let start = self.pos;
        while self.peek(0).is_some_and(|b| b != b'\n') {
            self.pos += 1;
        }
        let text = self.src[start..self.pos].trim_end_matches('\r').to_owned();
        let kind = if text.starts_with("///") && !text.starts_with("////") {
            CommentKind::Doc
        } else {
            CommentKind::Line
        };
        self.out.comments.push(Comment {
            kind,
            text,
            span: Span::new(start, self.pos),
        });
    }

    fn ident(&mut self) {
        let start = self.pos;
        while self.peek(0).is_some_and(is_ident_cont) {
            self.pos += 1;
        }
        let text = &self.src[start..self.pos];
        let tok = match keyword(text) {
            Some(k) => Tok::Keyword(k),
            None => Tok::Ident(text.to_owned()),
        };
        self.push(tok, start);
    }

    fn string(&mut self) {
        let start = self.pos;
        self.pos += 1;
        let mut value = String::new();
        loop {
            match self.peek(0) {
                None | Some(b'\n') => {
                    self.error(start, "unterminated string");
                    return;
                }
                Some(b'"') => {
                    self.pos += 1;
                    self.push(Tok::Str(value), start);
                    return;
                }
                Some(b'\\') => {
                    match self.peek(1) {
                        Some(b'"') => value.push('"'),
                        Some(b'\\') => value.push('\\'),
                        _ => {
                            self.pos += 1;
                            self.error(start, "unknown escape in string (only \\\" and \\\\)");
                            return;
                        }
                    }
                    self.pos += 2;
                }
                Some(_) => {
                    // Copy one whole UTF-8 character.
                    let ch = self.src[self.pos..].chars().next().unwrap_or('\u{fffd}');
                    value.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn digits(&mut self) -> usize {
        let from = self.pos;
        while self.peek(0).is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
        }
        self.pos - from
    }

    fn unit_follows(&self) -> bool {
        self.peek(0)
            .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'%' | b'"' | b'\''))
    }

    /// `.5mm`: a leading dot is a lex error.
    fn leading_dot(&mut self) {
        let start = self.pos;
        self.pos += 1;
        self.digits();
        self.recover();
        self.error(start, "a number may not start with `.`; write `0.5`");
    }

    /// Scans a number (integer, decimal, exponent, fraction, mixed) at `pos`.
    ///
    /// A fraction or mixed number only counts when a unit follows it
    /// immediately; otherwise the scan stops after the integer part.
    /// Returns false (after reporting) on a trailing dot.
    fn scan_number(&mut self) -> bool {
        let int_start = self.pos;
        self.digits();
        let int_end = self.pos;
        // Mixed number `1-1/2u` and fraction `3/4u`.
        if self.peek(0) == Some(b'-') && self.peek(1).is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
            self.digits();
            if self.peek(0) == Some(b'/') && self.peek(1).is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
                self.digits();
                if self.unit_follows() {
                    return true;
                }
            }
            self.pos = int_end;
        } else if self.peek(0) == Some(b'/') && self.peek(1).is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
            self.digits();
            if self.unit_follows() {
                return true;
            }
            self.pos = int_end;
        }
        if self.peek(0) == Some(b'.') {
            match self.peek(1) {
                Some(b) if b.is_ascii_digit() => {
                    self.pos += 1;
                    self.digits();
                }
                Some(b'.') => {}
                _ => {
                    self.pos += 1;
                    self.error(int_start, "a number may not end with `.`; write `5.0`");
                    return false;
                }
            }
        }
        if matches!(self.peek(0), Some(b'e' | b'E')) {
            let sign = usize::from(matches!(self.peek(1), Some(b'+' | b'-')));
            if self.peek(1 + sign).is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1 + sign;
                self.digits();
            }
        }
        true
    }

    fn number(&mut self) {
        let start = self.pos;
        if !self.scan_number() {
            return;
        }
        let number = self.src[start..self.pos].to_owned();
        if self.unit_follows() {
            self.quantity(start, number);
            return;
        }
        // Q007: whitespace between the number and a built-in unit.
        let ws_start = self.pos;
        let mut j = self.pos;
        while matches!(self.bytes.get(j), Some(b' ' | b'\t')) {
            j += 1;
        }
        if j > ws_start && self.split_unit_at(j) {
            let ws = Span::new(ws_start, j);
            self.pos = j;
            self.quantity_split(start, number, ws);
            return;
        }
        self.push(Tok::Number(number), start);
    }

    /// Whether a built-in unit atom starts at byte `j` and is a whole word.
    fn split_unit_at(&self, j: usize) -> bool {
        let mut k = j;
        while self.bytes.get(k).is_some_and(u8::is_ascii_alphanumeric) {
            k += 1;
        }
        let word = &self.src[j..k];
        BUILTIN_UNIT_ATOMS.contains(&word)
            && self.bytes.get(k) != Some(&b'(')
            && !self.bytes.get(k).copied().is_some_and(|b| b == b'_')
    }

    fn quantity_split(&mut self, start: usize, number: String, ws: Span) {
        let Some(read) = self.unit() else {
            self.error(start, "malformed quantity literal");
            return;
        };
        let span = Span::new(start, self.pos);
        self.diag(
            DiagCode::Q007,
            span,
            "quantity literal split by whitespace; write the number and unit together",
            Some(Fix {
                span: ws,
                replacement: String::new(),
            }),
        );
        self.push(
            Tok::Quantity(vec![Segment {
                number,
                unit: read.unit,
            }]),
            start,
        );
    }

    /// Reads the unit(s) after `number`, including compound continuations.
    fn quantity(&mut self, start: usize, first_number: String) {
        let mut segs: Vec<Segment> = Vec::new();
        let mut number = first_number;
        loop {
            let Some(read) = self.unit() else {
                self.recover();
                self.error(start, "malformed quantity literal");
                return;
            };
            segs.push(Segment {
                number,
                unit: read.unit,
            });
            if read.simple && self.peek(0).is_some_and(|b| b.is_ascii_digit()) {
                let ns = self.pos;
                self.digits();
                if self.peek(0) == Some(b'.') && self.peek(1).is_some_and(|b| b.is_ascii_digit()) {
                    self.pos += 1;
                    self.digits();
                }
                number = self.src[ns..self.pos].to_owned();
                if !self.unit_follows() {
                    self.recover();
                    self.error(start, "compound literal segment has no unit");
                    return;
                }
            } else {
                break;
            }
        }
        if let Some(msg) = check_compound(&segs) {
            self.recover();
            self.error(start, msg);
            return;
        }
        self.push(Tok::Quantity(segs), start);
    }

    /// Reads one unit lexeme at `pos`; `None` on a malformed one (`5"abc"`).
    fn unit(&mut self) -> Option<UnitRead> {
        match self.peek(0)? {
            b'%' => {
                self.pos += 1;
                Some(UnitRead {
                    unit: "%".to_owned(),
                    simple: false,
                })
            }
            q @ (b'"' | b'\'') => {
                self.pos += 1;
                if self.peek(0).is_some_and(is_ident_start) {
                    return None;
                }
                Some(UnitRead {
                    unit: if q == b'"' { "in" } else { "ft" }.to_owned(),
                    simple: true,
                })
            }
            b if b.is_ascii_alphabetic() => {
                let start = self.pos;
                let mut simple = true;
                self.atom();
                simple &= !self.exponent();
                while matches!(self.peek(0), Some(b'*' | b'/'))
                    && self.peek(1).is_some_and(|b| b.is_ascii_alphabetic())
                {
                    simple = false;
                    self.pos += 1;
                    self.atom();
                    self.exponent();
                }
                Some(UnitRead {
                    unit: self.src[start..self.pos].to_owned(),
                    simple,
                })
            }
            _ => None,
        }
    }

    /// Scans letters; digits only inside `H2O` (`mmH2O`, `inH2O`).
    fn atom(&mut self) {
        while self.peek(0).is_some_and(|b| b.is_ascii_alphabetic()) {
            self.pos += 1;
        }
        if self.src[..self.pos].ends_with('H') && self.src[self.pos..].starts_with("2O") {
            self.pos += 2;
        }
    }

    /// Scans `^n` or `^-n`; true when an exponent was consumed.
    fn exponent(&mut self) -> bool {
        if self.peek(0) != Some(b'^') {
            return false;
        }
        let neg = usize::from(self.peek(1) == Some(b'-'));
        if self.peek(1 + neg).is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1 + neg;
            self.digits();
            true
        } else {
            false
        }
    }

    fn punct(&mut self) {
        let start = self.pos;
        let two = (self.peek(0), self.peek(1));
        let (p, len) = match two {
            (Some(b':'), Some(b':')) => (Punct::ColonColon, 2),
            (Some(b'.'), Some(b'.')) => (Punct::DotDot, 2),
            (Some(b'='), Some(b'=')) => (Punct::EqEq, 2),
            (Some(b'='), Some(b'>')) => (Punct::FatArrow, 2),
            (Some(b'!'), Some(b'=')) => (Punct::Ne, 2),
            (Some(b'<'), Some(b'=')) => (Punct::Le, 2),
            (Some(b'>'), Some(b'=')) => (Punct::Ge, 2),
            (Some(b'-'), Some(b'>')) => (Punct::Arrow, 2),
            (Some(b'#'), Some(b'[')) => (Punct::HashBracket, 2),
            (Some(b'#'), Some(b'!')) if self.peek(2) == Some(b'[') => (Punct::HashBangBracket, 3),
            (Some(b'{'), _) => (Punct::LBrace, 1),
            (Some(b'}'), _) => (Punct::RBrace, 1),
            (Some(b'('), _) => (Punct::LParen, 1),
            (Some(b')'), _) => (Punct::RParen, 1),
            (Some(b'['), _) => (Punct::LBracket, 1),
            (Some(b']'), _) => (Punct::RBracket, 1),
            (Some(b';'), _) => (Punct::Semi, 1),
            (Some(b','), _) => (Punct::Comma, 1),
            (Some(b':'), _) => (Punct::Colon, 1),
            (Some(b'.'), _) => (Punct::Dot, 1),
            (Some(b'='), _) => (Punct::Eq, 1),
            (Some(b'<'), _) => (Punct::Lt, 1),
            (Some(b'>'), _) => (Punct::Gt, 1),
            (Some(b'+'), _) => (Punct::Plus, 1),
            (Some(b'-'), _) => (Punct::Minus, 1),
            (Some(b'*'), _) => (Punct::Star, 1),
            (Some(b'/'), _) => (Punct::Slash, 1),
            (Some(b'^'), _) => (Punct::Caret, 1),
            (Some(b'?'), _) => (Punct::Question, 1),
            (Some(b'%'), _) => (Punct::Percent, 1),
            (Some(b'@'), _) => (Punct::At, 1),
            _ => {
                let ch = self.src[self.pos..].chars().next().unwrap_or('\u{fffd}');
                self.pos += ch.len_utf8();
                self.error(start, format!("unexpected character `{ch}`"));
                return;
            }
        };
        self.pos += len;
        self.push(Tok::P(p), start);
    }
}

/// Validates a compound literal; returns the error message, if any.
fn check_compound(segs: &[Segment]) -> Option<String> {
    if segs.len() < 2 {
        return None;
    }
    let mut prev: Option<(CompoundDim, u64)> = None;
    for (i, seg) in segs.iter().enumerate() {
        let Some((dim, size)) = compound_info(&seg.unit) else {
            return Some(format!(
                "`{}` cannot be part of a compound literal",
                seg.unit
            ));
        };
        if i + 1 < segs.len() && !seg.number.bytes().all(|b| b.is_ascii_digit()) {
            return Some("only the last segment of a compound literal may be a decimal".to_owned());
        }
        if let Some((pdim, psize)) = prev {
            if pdim != dim {
                return Some("compound literal mixes dimensions".to_owned());
            }
            if size >= psize {
                return Some(
                    "compound literal segments must be in strictly descending unit size".to_owned(),
                );
            }
        }
        prev = Some((dim, size));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Tok> {
        lex(src)
            .tokens
            .into_iter()
            .map(|t| t.tok)
            .filter(|t| !matches!(t, Tok::Eof))
            .collect()
    }

    fn single_quantity(src: &str) -> Vec<(String, String)> {
        let l = lex(src);
        assert!(l.diags.is_empty(), "{src}: {:?}", l.diags);
        match kinds(src).as_slice() {
            [Tok::Quantity(segs)] => segs
                .iter()
                .map(|s| (s.number.clone(), s.unit.clone()))
                .collect(),
            other => panic!("{src}: expected one quantity, got {other:?}"),
        }
    }

    fn has_lex_error(src: &str) -> bool {
        let l = lex(src);
        l.diags.iter().any(|d| d.code == DiagCode::E001)
            && l.tokens.iter().any(|t| t.tok == Tok::Bad)
    }

    // frob:tests crates/cadetta-syntax/src/lexer.rs::lex kind=unit
    #[test]
    fn split_literal_is_q007_with_fixit() {
        let src = "5 mm";
        let l = lex(src);
        assert_eq!(l.diags.len(), 1);
        let d = &l.diags[0];
        assert_eq!(d.code, DiagCode::Q007);
        let fix = d.fix.as_ref().expect("fix-it");
        assert_eq!(fix.span.slice(src), " ");
        assert_eq!(fix.replacement, "");
        // Recovery still yields the joined literal.
        assert!(matches!(&l.tokens[0].tok, Tok::Quantity(s) if s[0].unit == "mm"));
        // `5 in` is the same mistake; `x in 5 .. 9` is not.
        assert_eq!(lex("5 in").diags[0].code, DiagCode::Q007);
        assert!(lex("a in 5 .. 9").diags.is_empty());
        assert!(lex("5 + x").diags.is_empty());
    }

    // frob:tests crates/cadetta-syntax/src/lexer.rs::lex kind=unit
    #[test]
    fn leading_and_trailing_dot_are_lex_errors() {
        assert!(has_lex_error(".5mm"));
        assert!(has_lex_error("5.mm"));
        assert!(has_lex_error("5."));
        // A range after an integer is not a trailing dot.
        assert_eq!(
            kinds("0..5"),
            vec![
                Tok::Number("0".into()),
                Tok::P(Punct::DotDot),
                Tok::Number("5".into())
            ]
        );
    }

    // frob:tests crates/cadetta-syntax/src/lexer.rs::lex kind=unit
    #[test]
    fn quote_alias_followed_by_letters_is_a_lex_error() {
        assert!(has_lex_error("5\"abc\""));
        assert!(has_lex_error("2'abc"));
        assert_eq!(single_quantity("5\""), vec![("5".into(), "in".into())]);
        assert_eq!(single_quantity("2'"), vec![("2".into(), "ft".into())]);
    }

    // frob:tests crates/cadetta-syntax/src/lexer.rs::lex kind=unit
    #[test]
    fn line_and_doc_comments_are_distinguished() {
        let l = lex("// c\n/// d\n//// e\nx");
        let kinds: Vec<_> = l.comments.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            vec![CommentKind::Line, CommentKind::Doc, CommentKind::Line]
        );
        assert_eq!(l.comments[1].doc_text(), "d");
        assert!(l.diags.is_empty());
        assert!(l.tokens.iter().all(|t| !matches!(t.tok, Tok::Bad)));
    }

    // frob:tests crates/cadetta-syntax/src/lexer.rs::lex kind=unit
    #[test]
    fn compound_literals_are_one_token() {
        assert_eq!(
            single_quantity("2'6\""),
            vec![("2".into(), "ft".into()), ("6".into(), "in".into())]
        );
        assert_eq!(
            single_quantity("2ft6in"),
            vec![("2".into(), "ft".into()), ("6".into(), "in".into())]
        );
        assert_eq!(
            single_quantity("1h30min"),
            vec![("1".into(), "h".into()), ("30".into(), "min".into())]
        );
        // Only the last segment may be a decimal.
        assert_eq!(single_quantity("1h30.5min").len(), 2);
        assert!(has_lex_error("1.5h30min"));
        // Unordered, equal, cross-dimension and unknown units are errors.
        assert!(has_lex_error("6in2ft"));
        assert!(has_lex_error("1h1h"));
        assert!(has_lex_error("1ft5min"));
        assert!(has_lex_error("5N2m"));
    }

    // frob:tests crates/cadetta-syntax/src/lexer.rs::lex kind=unit
    #[test]
    fn fractions_need_no_whitespace_and_a_unit() {
        assert_eq!(single_quantity("3/4in"), vec![("3/4".into(), "in".into())]);
        assert_eq!(
            single_quantity("1-1/2in"),
            vec![("1-1/2".into(), "in".into())]
        );
        assert_eq!(
            single_quantity("1/16\""),
            vec![("1/16".into(), "in".into())]
        );
        assert_eq!(
            kinds("3 / 4in"),
            vec![
                Tok::Number("3".into()),
                Tok::P(Punct::Slash),
                Tok::Quantity(vec![Segment {
                    number: "4".into(),
                    unit: "in".into()
                }]),
            ]
        );
        // Without a unit a fraction is division and a mixed number a subtraction.
        assert_eq!(kinds("3/4").len(), 3);
        assert_eq!(kinds("1-1/2").len(), 5);
    }

    // frob:tests crates/cadetta-syntax/src/lexer.rs::lex kind=unit
    #[test]
    fn exponent_compound_unit_and_percent_are_single_quantities() {
        assert_eq!(
            single_quantity("1e-3mm"),
            vec![("1e-3".into(), "mm".into())]
        );
        assert_eq!(
            single_quantity("9.81m/s^2"),
            vec![("9.81".into(), "m/s^2".into())]
        );
        assert_eq!(single_quantity("5%"), vec![("5".into(), "%".into())]);
        assert_eq!(
            single_quantity("3.2N*m"),
            vec![("3.2".into(), "N*m".into())]
        );
        assert_eq!(
            single_quantity("8mm/rev"),
            vec![("8".into(), "mm/rev".into())]
        );
        assert_eq!(
            single_quantity("2mmH2O"),
            vec![("2".into(), "mmH2O".into())]
        );
        assert_eq!(single_quantity("1500rpm").len(), 1);
        // A bare exponent number stays a bare number.
        assert_eq!(kinds("1e3"), vec![Tok::Number("1e3".into())]);
    }

    // frob:tests crates/cadetta-syntax/src/lexer.rs::keyword kind=unit
    #[test]
    fn reserved_words_lex_as_keywords() {
        for kw in KEYWORDS {
            assert_eq!(kinds(kw), vec![Tok::Keyword(kw)], "{kw}");
        }
        assert_eq!(kinds("pub"), vec![Tok::Keyword("pub")]);
        assert_eq!(kinds("pubx"), vec![Tok::Ident("pubx".into())]);
        assert_eq!(KEYWORDS.len(), 49);
    }

    // frob:tests crates/cadetta-syntax/src/lexer.rs::lex kind=unit
    #[test]
    fn strings_attributes_and_punctuation() {
        assert_eq!(kinds("\"a\\\"b\\\\\""), vec![Tok::Str("a\"b\\".into())]);
        assert!(has_lex_error("\"open"));
        assert_eq!(
            kinds("#![x] @std::iso273"),
            vec![
                Tok::P(Punct::HashBangBracket),
                Tok::Ident("x".into()),
                Tok::P(Punct::RBracket),
                Tok::P(Punct::At),
                Tok::Ident("std".into()),
                Tok::P(Punct::ColonColon),
                Tok::Ident("iso273".into()),
            ]
        );
        assert_eq!(kinds("a\n\n\nb").len(), 3);
        assert!(has_lex_error("$"));
    }
}
