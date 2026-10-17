//! The canonical formatter (language.md 7): one layout for every valid file.
//!
//! Layout only: [`format_source`] re-parses its own output and refuses to return
//! text whose syntax tree differs from the input's (spans aside), so formatting can
//! never change the IR digest. Numbers, quantity literals and citations are copied
//! from the source text, so authored units and literal forms survive.
//!
//! Files with syntax errors (including items the parser skips with E001, such as
//! `process`, `table`, `enum`, `fn`, `solver`, `waive`, `unit`
//! and `kind`) cannot be formatted yet: [`FormatError::Syntax`].

// frob:ticket 01M4SPWPAJ4ZGEGGVF63ZY806Z

use std::fmt;

use crate::ast::{
    // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
    Arg,
    Attr,
    AttrArg,
    BinOp,
    Block,
    Bound,
    BoundKind,
    Check,
    Citation,
    Cite,
    CiteName,
    CiteValue,
    CmpOp,
    Component,
    ComponentClass,
    Ctor,
    Datum,
    Doc,
    Expr,
    ExprKind,
    File,
    FileEntry,
    Fit,
    FitSpec,
    Interface,
    InterfaceMember,
    InterfaceMemberKind,
    Item,
    ItemKind,
    KindOp,
    Let,
    Member,
    MemberKind,
    Param,
    ParamDecl,
    Part,
    Path,
    Pattern,
    Place,
    Process,
    ProcessMemberKind,
    Require,
    Solver,
    SolverMemberKind,
    Table,
    TolSign,
    Type,
    TypeKind,
    UnOp,
    UnitDecl,
    Use,
    UseKind,
    UsePath,
    UseRoot,
    UseTree,
    UseTreeKind,
    Vis,
};
use crate::lexer::{Comment, Span};
use crate::parser::parse;
use crate::span::SourceFile;

/// One indent level, in spaces (language.md 7).
const INDENT: usize = 2;
/// The gap before an aligned trailing attribute or comment column.
const GAP: &str = "  ";

/// Why a source could not be formatted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FormatError {
    /// The source has syntax errors; a broken file is never "formatted away".
    Syntax {
        /// How many diagnostics the parser reported.
        count: usize,
        /// The first one, as `line:col: message`.
        first: String,
    },
    /// The output did not parse back to the same tree (a formatter bug; nothing is returned).
    Unstable {
        /// What went wrong.
        detail: String,
    },
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { count, first } => {
                write!(
                    f,
                    "not formatted, {count} syntax error(s); first at {first}"
                )
            }
            Self::Unstable { detail } => write!(f, "formatter bug, output rejected: {detail}"),
        }
    }
}

impl std::error::Error for FormatError {}

/// Formats one .cdt source into its canonical layout; `Err` for a file with syntax errors.
pub fn format_source(src: &str) -> Result<String, FormatError> {
    let parsed = parse(src);
    if let Some(first) = parsed.diags.first() {
        let lc = SourceFile::new("", src).line_col(first.span.start);
        let err = FormatError::Syntax {
            count: parsed.diags.len(),
            first: format!("{}:{}: {}", lc.line, lc.col, first.message),
        };
        tracing::info!(error = %err, "fmt refused");
        return Err(err);
    }
    let mut p = Printer {
        src,
        comments: &parsed.comments,
        used: vec![false; parsed.comments.len()],
        cursor: 0,
        out: String::new(),
        last_end: None,
        group: Vec::new(),
        indent: 0,
    };
    p.file(&parsed.file);
    let out = p.out;
    let again = parse(&out);
    if !again.diags.is_empty() || shape(&again.file) != shape(&parsed.file) {
        let detail = "output re-parses to a different tree".to_owned();
        tracing::error!(%detail, "fmt rejected its own output");
        return Err(FormatError::Unstable { detail });
    }
    tracing::debug!(bytes = out.len(), "formatted");
    Ok(out)
}

/// True when `src` is already canonical (what `cadetta fmt --check` asks); changes nothing.
pub fn is_formatted(src: &str) -> Result<bool, FormatError> {
    let formatted = format_source(src)?;
    let same = formatted == src;
    tracing::debug!(same, "fmt check");
    Ok(same)
}

/// The tree's debug text with every span removed: equal shapes mean equal meaning.
fn shape(file: &File) -> String {
    let dbg = format!("{file:?}");
    let mut out = String::with_capacity(dbg.len());
    let mut rest = dbg.as_str();
    while let Some(i) = rest.find("Span { start: ") {
        out.push_str(&rest[..i]);
        match rest[i..].find('}') {
            Some(j) => rest = &rest[i + j + 1..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// One buffered single-line statement, waiting for its group to be aligned.
struct Line {
    indent: usize,
    /// Everything before the aligned `=` (the whole statement when `rest` is `None`).
    lhs: String,
    /// What follows ` = `, when the statement has an aligned `=`.
    rest: Option<String>,
    /// The separator before `rest`: ` = ` or ` => `.
    sep: &'static str,
    /// Trailing attributes, `#[a, b]`.
    attrs: String,
    /// A trailing `//` comment.
    comment: String,
}

impl Line {
    fn plain(lhs: String) -> Self {
        Self {
            indent: 0,
            lhs,
            rest: None,
            sep: " = ",
            attrs: String::new(),
            comment: String::new(),
        }
    }

    fn eq(lhs: String, rest: String) -> Self {
        Self {
            rest: Some(rest),
            ..Self::plain(lhs)
        }
    }

    fn arrow(lhs: String, rest: String) -> Self {
        Self {
            sep: " => ",
            ..Self::eq(lhs, rest)
        }
    }
}

struct Printer<'a> {
    src: &'a str,
    comments: &'a [Comment],
    used: Vec<bool>,
    /// First comment that may be unused.
    cursor: usize,
    out: String,
    /// Source offset where the last emitted thing ended; `None` at the top of a block.
    last_end: Option<usize>,
    /// Consecutive statements being aligned as one block.
    group: Vec<Line>,
    indent: usize,
}

fn width(s: &str) -> usize {
    s.chars().count()
}

fn pad(s: &str, w: usize) -> String {
    format!("{s:<w$}")
}

impl Printer<'_> {
    // ---- low level output ----

    fn pad_indent(&self, indent: usize) -> String {
        " ".repeat(indent)
    }

    fn flush_group(&mut self) {
        let lines = std::mem::take(&mut self.group);
        if lines.is_empty() {
            return;
        }
        let eq_w = lines
            .iter()
            .filter(|l| l.rest.is_some())
            .map(|l| width(&l.lhs))
            .max()
            .unwrap_or(0);
        let body = |l: &Line| match &l.rest {
            Some(r) => format!("{}{}{r}", pad(&l.lhs, eq_w), l.sep),
            None => l.lhs.clone(),
        };
        let bodies: Vec<String> = lines.iter().map(body).collect();
        let tail = |l: &Line| !l.attrs.is_empty() || !l.comment.is_empty();
        let body_w = lines
            .iter()
            .zip(&bodies)
            .filter(|(l, _)| tail(l))
            .map(|(_, b)| width(b))
            .max()
            .unwrap_or(0);
        let attr_w = lines.iter().map(|l| width(&l.attrs)).max().unwrap_or(0);
        tracing::trace!(lines = lines.len(), eq_w, body_w, attr_w, "aligned a group");
        for (l, b) in lines.iter().zip(bodies) {
            let mut s = if tail(l) { pad(&b, body_w) } else { b };
            if tail(l) {
                if attr_w > 0 {
                    s.push_str(GAP);
                    s.push_str(&pad(&l.attrs, attr_w));
                }
                if !l.comment.is_empty() {
                    s.push_str(GAP);
                    s.push_str(&l.comment);
                }
            }
            self.out.push_str(&self.pad_indent(l.indent));
            self.out.push_str(s.trim_end());
            self.out.push('\n');
        }
    }

    /// Writes one finished line, ending any open group first.
    fn raw_line(&mut self, indent: usize, text: &str) {
        self.flush_group();
        self.out.push_str(&self.pad_indent(indent));
        self.out.push_str(text.trim_end());
        self.out.push('\n');
    }

    fn touch(&mut self, end: usize) {
        self.last_end = Some(self.last_end.map_or(end, |e| e.max(end)));
    }

    /// Emits one blank line when the source had at least one between the last thing and `start`.
    fn gap(&mut self, start: usize) {
        let Some(e) = self.last_end else { return };
        if start > e && self.src[e..start].matches('\n').count() >= 2 {
            self.flush_group();
            self.out.push('\n');
        }
    }

    // ---- comments ----

    fn advance_cursor(&mut self) {
        while self.cursor < self.comments.len() && self.used[self.cursor] {
            self.cursor += 1;
        }
    }

    fn mark_doc(&mut self, doc: &Doc) {
        if let Ok(i) = self
            .comments
            .binary_search_by_key(&doc.span.start, |c| c.span.start)
        {
            self.used[i] = true;
        }
    }

    /// Emits every unused comment that starts before `upto` as an own-line comment.
    fn drain_comments(&mut self, upto: usize, indent: usize) {
        self.advance_cursor();
        let mut i = self.cursor;
        while i < self.comments.len() && self.comments[i].span.start < upto {
            if !self.used[i] {
                self.used[i] = true;
                let c = &self.comments[i];
                let (start, end, text) = (c.span.start, c.span.end, c.text.trim_end().to_owned());
                self.gap(start);
                self.raw_line(indent, &text);
                self.touch(end);
            }
            i += 1;
        }
    }

    /// Claims the unused comment on the same source line right after `after`, if any.
    fn trailing_comment(&mut self, after: usize) -> Option<String> {
        self.advance_cursor();
        let i = (self.cursor..self.comments.len())
            .find(|&i| !self.used[i] && self.comments[i].span.start >= after)?;
        let c = &self.comments[i];
        if self.src[after..c.span.start].contains('\n') {
            return None;
        }
        self.used[i] = true;
        let end = c.span.end;
        let text = c.text.trim_end().to_owned();
        self.touch(end);
        Some(text)
    }

    /// End of the source line holding `after`, trailing blanks trimmed.
    fn line_end(&self, after: usize) -> usize {
        let rest = &self.src[after..];
        let line = rest.find('\n').map_or(rest, |n| &rest[..n]);
        after + line.trim_end().len()
    }

    // ---- statement scaffolding ----

    /// Comments, blank line, docs and leading attributes that precede a statement.
    fn preamble(&mut self, first_start: usize, pre_upto: usize, docs: &[Doc], attrs: &[Attr]) {
        for d in docs {
            self.mark_doc(d);
        }
        self.drain_comments(pre_upto.min(first_start.max(pre_upto)), self.indent);
        for d in docs {
            self.gap(d.span.start);
            let text = if d.text.trim().is_empty() {
                "///".to_owned()
            } else {
                format!("/// {}", d.text.trim_end())
            };
            self.raw_line(self.indent, &text);
            self.touch(d.span.end);
        }
        for a in attrs {
            self.gap(a.span.start);
            let text = self.attr_group(std::slice::from_ref(a));
            self.raw_line(self.indent, &text);
            self.touch(self.line_end(a.span.end));
        }
        self.gap(first_start);
    }

    /// Buffers a single-line leaf statement, claiming its trailing attributes and comment.
    fn leaf(&mut self, mut line: Line, span: Span, trailing: &[Attr]) {
        if !trailing.is_empty() {
            line.attrs = self.attr_group(trailing);
        }
        let mut end = span.end;
        for a in trailing {
            end = end.max(a.span.end);
        }
        if let Some(c) = self.trailing_comment(end) {
            line.comment = c;
        }
        self.touch(self.line_end(end));
        line.indent = self.indent;
        self.group.push(line);
    }

    // ---- file ----

    fn file(&mut self, file: &File) {
        enum Top<'f> {
            Inner(&'f Attr),
            Entry(&'f FileEntry),
        }
        let mut tops: Vec<(usize, Top)> = Vec::new();
        for a in &file.inner_attrs {
            tops.push((a.span.start, Top::Inner(a)));
        }
        for e in &file.entries {
            let start = match e {
                FileEntry::Use(u) => u.span.start,
                FileEntry::Item(i) => item_first_start(i),
            };
            tops.push((start, Top::Entry(e)));
        }
        tops.sort_by_key(|(s, _)| *s);
        for (_, top) in tops {
            match top {
                Top::Inner(a) => {
                    self.preamble(a.span.start, a.span.start, &[], &[]);
                    let line = Line::plain(self.inner_attr(a));
                    self.leaf(line, a.span, &[]);
                }
                Top::Entry(FileEntry::Use(u)) => {
                    self.preamble(u.span.start, u.span.end, &[], &[]);
                    self.leaf(Line::plain(self.use_decl(u)), u.span, &[]);
                }
                Top::Entry(FileEntry::Item(i)) => self.item(i),
            }
        }
        self.drain_comments(usize::MAX, 0);
        self.flush_group();
        tracing::debug!(lines = self.out.matches('\n').count(), "printed file");
    }

    fn item(&mut self, item: &Item) {
        let first = item_first_start(item);
        let vis = vis_prefix(item.vis);
        let leaf_upto = item.span.end;
        match &item.kind {
            ItemKind::Component(c) => {
                self.preamble(first, item.span.start, &item.docs, &item.attrs);
                self.component(c, item, &vis);
            }
            ItemKind::Interface(i) => {
                self.preamble(first, item.span.start, &item.docs, &item.attrs);
                self.interface(i, item, &vis);
            }
            ItemKind::Cite(c) => {
                self.preamble(first, item.span.start, &item.docs, &item.attrs);
                self.cite_item(c, item, &vis);
            }
            ItemKind::Fit(f) => {
                self.preamble(first, leaf_upto, &item.docs, &item.attrs);
                self.leaf(self.fit_line(f, &vis), item.span, &[]);
            }
            ItemKind::Require(r) => {
                self.preamble(first, leaf_upto, &item.docs, &item.attrs);
                self.leaf(Line::plain(self.require(r, &vis)), item.span, &[]);
            }
            ItemKind::Check(c) => {
                self.preamble(first, leaf_upto, &item.docs, &item.attrs);
                self.leaf(Line::plain(self.check(c, &vis)), item.span, &[]);
            }
            ItemKind::Process(p) => {
                self.preamble(first, item.span.start, &item.docs, &item.attrs);
                self.process(p, item, &vis);
            }
            ItemKind::Table(t) => {
                self.preamble(first, item.span.start, &item.docs, &item.attrs);
                self.table(t, item, &vis);
            }
            ItemKind::Solver(sv) => {
                self.preamble(first, item.span.start, &item.docs, &item.attrs);
                self.solver(sv, item.span, &vis, "");
            }
            ItemKind::Enum(e) => {
                self.preamble(first, leaf_upto, &item.docs, &item.attrs);
                let vs: Vec<&str> = e.variants.iter().map(|v| v.name.as_str()).collect();
                let text = format!("{vis}enum {} {{ {} }}", e.name.name, vs.join(", "));
                self.leaf(Line::plain(text), item.span, &[]);
            }
            ItemKind::Fn(f) => {
                self.preamble(first, leaf_upto, &item.docs, &item.attrs);
                let ps: Vec<String> = f.params.iter().map(|p| self.pdecl(p)).collect();
                let text = format!(
                    "{vis}fn {}({}) -> {} {}",
                    f.name.name,
                    ps.join(", "),
                    self.ty(&f.ret),
                    self.block(&f.body)
                );
                self.leaf(Line::plain(text), item.span, &[]);
            }
            ItemKind::Waive(w) => {
                self.preamble(first, leaf_upto, &item.docs, &item.attrs);
                let mut text = format!("{vis}waive {} on {}", w.code.name, w.glob.text);
                if let Some(a) = &w.for_alt {
                    text.push_str(&format!(" for alt {} = {}", a.slot.text, a.alt.text));
                }
                text.push_str(&format!(" {{ because {}", quote(&w.because)));
                if let Some(d) = &w.expires {
                    text.push_str(&format!(" expires {}", d.text));
                }
                text.push_str(" }");
                self.leaf(Line::plain(text), item.span, &[]);
            }
            ItemKind::Unit(u) => {
                self.preamble(first, leaf_upto, &item.docs, &item.attrs);
                let line = match u {
                    UnitDecl::Define { name, value } => {
                        Line::eq(format!("{vis}unit {}", name.name), self.expr(value))
                    }
                    UnitDecl::Default { kind, unit } => {
                        Line::plain(format!("{vis}unit {} {}", kind.name, unit.name))
                    }
                };
                self.leaf(line, item.span, &[]);
            }
            ItemKind::Kind(k) => {
                self.preamble(first, leaf_upto, &item.docs, &item.attrs);
                let line = Line::eq(format!("{vis}kind {}", k.name.name), self.ty(&k.def));
                self.leaf(line, item.span, &[]);
            }
            ItemKind::Error => {
                // Unreachable for an error-free parse; keep the text rather than lose it.
                self.preamble(first, leaf_upto, &item.docs, &item.attrs);
                self.leaf(
                    Line::plain(item.span.slice(self.src).trim().to_owned()),
                    item.span,
                    &[],
                );
            }
        }
    }

    /// Closes a `{ ... }` body: leftover comments, the `}` line and its trailing comment.
    fn close_block(&mut self, item_span: Span, outer: usize, suffix: &str) {
        let close = item_span.start + item_span.slice(self.src).rfind('}').unwrap_or(0);
        self.drain_comments(close, outer + INDENT);
        self.flush_group();
        self.indent = outer;
        let mut text = format!("}}{suffix}");
        if let Some(c) = self.trailing_comment(close + 1) {
            text.push_str(GAP);
            text.push_str(&c);
        }
        self.raw_line(outer, &text);
        self.touch(self.line_end(close + 1));
    }

    fn component(&mut self, c: &Component, item: &Item, vis: &str) {
        let class = match c.class {
            ComponentClass::Ots => "ots",
            ComponentClass::Model => "model",
            ComponentClass::Assembly => "assembly",
        };
        let mut head = format!("{vis}{class} {}", c.name.name);
        if !c.params.is_empty() {
            let ps: Vec<String> = c.params.iter().map(|p| self.pdecl(p)).collect();
            head.push_str(&format!("({})", ps.join(", ")));
        }
        if !c.implements.is_empty() {
            head.push_str(&format!(" : {}", paths(&c.implements)));
        }
        if let Some(cit) = &c.citation {
            head.push_str(&format!(" {}", self.citation(cit)));
        }
        head.push_str(" {");
        let outer = self.indent;
        self.raw_line(outer, &head);
        self.last_end = None;
        self.indent = outer + INDENT;
        enum Body<'m> {
            Inner(&'m Attr),
            Member(&'m Member),
        }
        let mut body: Vec<(usize, Body)> = Vec::new();
        for a in &c.inner_attrs {
            body.push((a.span.start, Body::Inner(a)));
        }
        for m in &c.members {
            body.push((member_first_start(m), Body::Member(m)));
        }
        body.sort_by_key(|(s, _)| *s);
        for (_, b) in body {
            match b {
                Body::Inner(a) => {
                    self.preamble(a.span.start, a.span.start, &[], &[]);
                    let line = Line::plain(self.inner_attr(a));
                    self.leaf(line, a.span, &[]);
                }
                Body::Member(m) => self.member(m),
            }
        }
        self.close_block(item.span, outer, "");
    }

    fn member(&mut self, m: &Member) {
        let p = if m.private { "priv " } else { "" };
        if let MemberKind::Solver(sv) = &m.kind {
            self.preamble(member_first_start(m), m.span.start, &m.docs, &m.attrs);
            let suffix = if m.trailing.is_empty() {
                String::new()
            } else {
                format!("  {}", self.attr_group(&m.trailing))
            };
            self.solver(sv, m.span, p, &suffix);
            return;
        }
        self.preamble(member_first_start(m), m.span.end, &m.docs, &m.attrs);
        let line = match &m.kind {
            MemberKind::Param(x) => self.param_line(x, p),
            MemberKind::Let(l) => self.let_line(l, p),
            MemberKind::Bound(b) => Line::plain(format!("{p}{}", self.bound(b))),
            MemberKind::Part(x) => Line::plain(format!("{p}{}", self.part(x))),
            MemberKind::Fit(f) => self.fit_line(f, p),
            MemberKind::Check(c) => Line::plain(self.check(c, p)),
            MemberKind::Require(r) => Line::plain(self.require(r, p)),
            MemberKind::Solver(_) => unreachable!("handled above"),
            MemberKind::Datum(d) => Line::plain(format!("{p}{}", self.datum(d))),
            MemberKind::Place(pl) => Line::plain(format!("{p}{}", self.place(pl))),
            MemberKind::Error => Line::plain(m.span.slice(self.src).trim().to_owned()),
        };
        let mut line = line;
        if let Some(c) = &m.citation {
            let cite = self.citation(c);
            let text = line.rest.as_mut().unwrap_or(&mut line.lhs);
            text.push(' ');
            text.push_str(&cite);
        }
        self.leaf(line, m.span, &m.trailing);
    }

    /// Opens a `{` body: prints the header and indents; returns the outer indent.
    fn open_block(&mut self, head: &str) -> usize {
        let outer = self.indent;
        self.raw_line(outer, &format!("{head} {{"));
        self.last_end = None;
        self.indent = outer + INDENT;
        outer
    }

    fn cite_suffix(&self, c: &Option<Citation>) -> String {
        c.as_ref()
            .map_or_else(String::new, |c| format!(" {}", self.citation(c)))
    }

    fn process(&mut self, p: &Process, item: &Item, vis: &str) {
        let head = format!(
            "{vis}process {}{}",
            p.name.name,
            self.cite_suffix(&p.citation)
        );
        let outer = self.open_block(&head);
        for m in &p.members {
            let first = m
                .docs
                .first()
                .map(|d| d.span.start)
                .or_else(|| m.attrs.first().map(|a| a.span.start))
                .unwrap_or(m.span.start);
            self.preamble(first, m.span.end, &m.docs, &m.attrs);
            let line = match &m.kind {
                ProcessMemberKind::Param(x) => self.param_line(x, ""),
                ProcessMemberKind::Rule(r) => Line::plain(format!(
                    "rule {}({}) -> {} = {}",
                    r.name.name,
                    self.pdecl(&r.param),
                    self.ty(&r.ret),
                    self.expr(&r.body)
                )),
                ProcessMemberKind::Error => Line::plain(m.span.slice(self.src).trim().to_owned()),
            };
            self.leaf(line, m.span, &[]);
        }
        self.close_block(item.span, outer, "");
    }

    fn table(&mut self, t: &Table, item: &Item, vis: &str) {
        let head = format!(
            "{vis}table {}{}",
            t.name.name,
            self.cite_suffix(&t.citation)
        );
        let outer = self.open_block(&head);
        if let Some(k) = &t.key {
            self.preamble(k.span.start, k.span.end, &[], &[]);
            self.leaf(Line::plain(format!("key {}", self.pdecl(k))), k.span, &[]);
        }
        if let (Some(first), Some(last)) = (t.cols.first(), t.cols.last()) {
            self.preamble(first.span.start, last.span.end, &[], &[]);
            let cols: Vec<String> = t.cols.iter().map(|c| self.pdecl(c)).collect();
            let span = Span::new(first.span.start, last.span.end);
            self.leaf(Line::plain(format!("cols {}", cols.join(", "))), span, &[]);
        }
        for r in &t.rows {
            let first = r.docs.first().map_or(r.span.start, |d| d.span.start);
            self.preamble(first, r.span.end, &r.docs, &[]);
            let key = self.expr(&r.key);
            let line = match &r.values {
                Some(vs) => {
                    let vs: Vec<String> = vs.iter().map(|v| self.expr(v)).collect();
                    Line::arrow(key, vs.join(", "))
                }
                None => Line::plain(key),
            };
            self.leaf(line, r.span, &[]);
        }
        self.close_block(item.span, outer, "");
    }

    fn solver(&mut self, sv: &Solver, span: Span, prefix: &str, suffix: &str) {
        // frob:ticket 01M4VB630NZKXA6Q9D7253FATR
        let mut head = format!("{prefix}solver {}", sv.name.name);
        if let Some(c) = &sv.citation {
            head.push(' ');
            head.push_str(&self.citation(c));
        }
        let outer = self.open_block(&head);
        self.leaf(
            Line::plain(format!("script {}", quote(&sv.script))),
            sv.script_span,
            &[],
        );
        for m in &sv.members {
            self.preamble(m.span.start, m.span.end, &[], &[]);
            let text = match &m.kind {
                SolverMemberKind::In(binds) => {
                    let bs: Vec<String> = binds
                        .iter()
                        .map(|b| format!("{} = {}", b.name.name, self.expr(&b.value)))
                        .collect();
                    format!("in {}", bs.join(", "))
                }
                SolverMemberKind::Out { name, ty } => match ty {
                    Some(ty) => format!("out {}: {}", name.name, self.ty(ty)),
                    None => format!("out {}", name.name),
                },
                SolverMemberKind::Deps(f) => format!("deps {}", quote(f)),
                SolverMemberKind::Check(c) => self.check(c, ""),
                SolverMemberKind::Error => m.span.slice(self.src).trim().to_owned(),
            };
            self.leaf(Line::plain(text), m.span, &[]);
        }
        self.close_block(span, outer, suffix);
    }

    fn pdecl(&self, p: &ParamDecl) -> String {
        let q = if p.optional { "?" } else { "" };
        let mut s = format!("{}{q}: {}", p.name.name, self.ty(&p.ty));
        if let Some(d) = &p.default {
            s.push_str(&format!(" = {}", self.expr(d)));
        }
        s
    }

    fn interface(&mut self, i: &Interface, item: &Item, vis: &str) {
        let mut head = format!("{vis}interface {}", i.name.name);
        if !i.extends.is_empty() {
            head.push_str(&format!(" : {}", paths(&i.extends)));
        }
        head.push_str(" {");
        let outer = self.indent;
        self.raw_line(outer, &head);
        self.last_end = None;
        self.indent = outer + INDENT;
        for m in &i.members {
            self.interface_member(m);
        }
        self.close_block(item.span, outer, "");
    }

    fn interface_member(&mut self, m: &InterfaceMember) {
        let first = m
            .docs
            .first()
            .map(|d| d.span.start)
            .or_else(|| m.attrs.first().map(|a| a.span.start))
            .unwrap_or(m.span.start);
        self.preamble(first, m.span.end, &m.docs, &m.attrs);
        let text = match &m.kind {
            InterfaceMemberKind::Field { name, optional, ty } => {
                let q = if *optional { "?" } else { "" };
                format!("{}{q}: {}", name.name, self.ty(ty))
            }
            InterfaceMemberKind::Bound(b) => self.bound(b),
            InterfaceMemberKind::Datum(d) => self.datum(d),
            InterfaceMemberKind::Error => m.span.slice(self.src).trim().to_owned(),
        };
        self.leaf(Line::plain(text), m.span, &[]);
    }

    // ---- statements ----

    fn param_line(&self, p: &Param, prefix: &str) -> Line {
        let mut lhs = format!("{prefix}{}", p.name.name);
        if let Some(t) = &p.ty {
            lhs.push_str(&format!(": {}", self.ty(t)));
        }
        let mut rest = self.expr(&p.value);
        if let Some(c) = &p.citation {
            rest.push_str(&format!(" {}", self.citation(c)));
        }
        for w in &p.was {
            rest.push_str(&format!(" was {}", w.text()));
        }
        Line::eq(lhs, rest)
    }

    fn let_line(&self, l: &Let, prefix: &str) -> Line {
        let mut lhs = format!("{prefix}let {}", l.name.name);
        if let Some(t) = &l.ty {
            lhs.push_str(&format!(": {}", self.ty(t)));
        }
        Line::eq(lhs, self.expr(&l.value))
    }

    fn bound(&self, b: &Bound) -> String {
        match &b.kind {
            BoundKind::Le { name, value } => format!("{} <= {}", name.name, self.expr(value)),
            BoundKind::Ge { name, value } => format!("{} >= {}", name.name, self.expr(value)),
            BoundKind::In { name, lo, hi } => {
                format!("{} in {}..{}", name.name, self.expr(lo), self.expr(hi))
            }
            BoundKind::Eq { lhs, rhs } => format!("{} == {}", lhs.text(), rhs.text()),
        }
    }

    fn datum(&self, d: &Datum) -> String {
        match &d.value {
            Some(v) => format!("datum {} = {}", d.name.name, self.expr(v)),
            None => format!("datum {}", d.name.name),
        }
    }

    fn place(&self, p: &Place) -> String {
        format!("place {} at {}", self.expr(&p.target), self.expr(&p.at))
    }

    fn part(&self, p: &Part) -> String {
        let mut s = format!("part {}", p.name.name);
        if let Some(n) = &p.count {
            s.push_str(&format!("[{}]", self.expr(n)));
        }
        s.push_str(&format!(": {}", self.ty(&p.ty)));
        if let Some(args) = &p.args {
            s.push_str(&self.args(args));
        }
        if let Some(primary) = &p.primary {
            s.push_str(&format!(" = {}", self.ctor(primary)));
            if !p.alts.is_empty() {
                let alts: Vec<String> = p.alts.iter().map(|c| self.ctor(c)).collect();
                s.push_str(&format!(" alt [{}]", alts.join(", ")));
            }
        }
        s
    }

    fn ctor(&self, c: &Ctor) -> String {
        let mut s = c.path.text();
        if let Some(args) = &c.args {
            s.push_str(&self.args(args));
        }
        s
    }

    fn fit_line(&self, f: &Fit, prefix: &str) -> Line {
        let spec = match &f.spec {
            FitSpec::Class(a) => a.name.clone(),
            FitSpec::Pair(a, b) => format!("{}/{}", a.name, b.name),
            FitSpec::Paths(a, b) => format!("{} / {}", a.text(), b.text()),
        };
        let mut rest = if f.on_implied {
            spec
        } else {
            format!("{spec} on {}", f.on.text())
        };
        if let Some((lo, hi)) = &f.range {
            rest.push_str(&format!(" [{} .. {}]", self.expr(lo), self.expr(hi)));
        }
        Line::eq(format!("{prefix}fit {}", f.name.name), rest)
    }

    fn check(&self, c: &Check, prefix: &str) -> String {
        let mut s = format!("{prefix}check {}", c.name.name);
        if let Some((var, set)) = &c.each {
            s.push_str(&format!(" for {} in {}", var.name, self.expr(set)));
        }
        s.push_str(&format!(": {}", self.expr(&c.cond)));
        if let Some(m) = &c.message {
            s.push_str(&format!(" {}", quote(m)));
        }
        s
    }

    fn require(&self, r: &Require, prefix: &str) -> String {
        let mut s = format!("{prefix}require {}: {}", r.name.name, self.expr(&r.cond));
        if let Some(m) = &r.message {
            s.push_str(&format!(" {}", quote(m)));
        }
        if let Some(c) = &r.citation {
            s.push_str(&format!(" {}", self.citation(c)));
        }
        s
    }

    /// A typed `cite` as a block with one aligned `name = value` line per field.
    fn cite_item(&mut self, c: &Cite, item: &Item, vis: &str) {
        let head = format!("{vis}{}", cite_lhs(c));
        let outer = self.open_block(&head);
        for f in &c.fields {
            self.preamble(f.span.start, f.span.end, &[], &[]);
            let value = match &f.value {
                CiteValue::Text(t) => quote(t),
                CiteValue::List(l) => {
                    let items: Vec<String> = l.iter().map(|t| quote(t)).collect();
                    format!("[{}]", items.join(", "))
                }
                CiteValue::Other => f.span.slice(self.src).split_once('=').map_or_else(
                    || f.span.slice(self.src).trim().to_owned(),
                    |(_, v)| v.trim().to_owned(),
                ),
            };
            self.leaf(Line::eq(f.name.name.clone(), value), f.span, &[]);
        }
        self.close_block(item.span, outer, "");
    }

    fn use_decl(&self, u: &Use) -> String {
        let mut s = format!("{}use {}", vis_prefix(u.vis), use_path(&u.path));
        match &u.kind {
            UseKind::Single { alias } => {
                if let Some(a) = alias {
                    s.push_str(&format!(" as {}", a.name));
                }
            }
            UseKind::Glob => s.push_str("::*"),
            UseKind::Tree(trees) => s.push_str(&format!("::{}", use_group(trees))),
        }
        s
    }

    // ---- attributes, citations, types ----

    fn attr_body(&self, a: &Attr) -> String {
        let mut s = a.name.name.clone();
        if !a.args.is_empty() {
            let args: Vec<String> = a
                .args
                .iter()
                .map(|x| match x {
                    AttrArg::Expr(e) => self.expr(e),
                    AttrArg::Named { name, value } => {
                        format!("{} = {}", name.name, self.expr(value))
                    }
                })
                .collect();
            s.push_str(&format!("({})", args.join(", ")));
        }
        s
    }

    /// `#[a, b]` for outer attributes.
    fn attr_group(&self, attrs: &[Attr]) -> String {
        let body: Vec<String> = attrs.iter().map(|a| self.attr_body(a)).collect();
        format!("#[{}]", body.join(", "))
    }

    fn inner_attr(&self, a: &Attr) -> String {
        format!("#![{}]", self.attr_body(a))
    }

    fn citation(&self, c: &Citation) -> String {
        // Citations are copied from the source so locators and literals stay as authored.
        let text = c.span.slice(self.src).trim();
        if text.is_empty() {
            let name = match &c.name {
                CiteName::Named(n) => format!("@{}", n.text()),
                CiteName::Literal(l) => format!("@{}", quote(l)),
            };
            return match &c.locator {
                Some(l) => format!("{name}:{}", quote(l)),
                None => name,
            };
        }
        text.to_owned()
    }

    fn ty(&self, t: &Type) -> String {
        match &t.kind {
            TypeKind::Path(p) => p.text(),
            TypeKind::OnDatum { base, datum } => format!("{} on {}", base.text(), datum.text()),
            TypeKind::Array { elem, len } => format!("[{}; {}]", self.ty(elem), self.expr(len)),
            TypeKind::Table(k) => format!("Table<{}>", self.ty(k)),
            TypeKind::KindExpr(k) => {
                let mut s = String::new();
                for (i, t) in k.terms.iter().enumerate() {
                    if i > 0 {
                        s.push_str(if t.op == KindOp::Div { " / " } else { " * " });
                    }
                    s.push_str(&t.name.text());
                    if let Some(e) = t.exp {
                        s.push_str(&format!("^{e}"));
                    }
                }
                s
            }
        }
    }

    // ---- expressions ----

    fn args(&self, args: &[Arg]) -> String {
        let parts: Vec<String> = args
            .iter()
            .map(|a| match &a.name {
                Some(n) => format!("{} = {}", n.name, self.expr(&a.value)),
                None => self.expr(&a.value),
            })
            .zip(args)
            .map(|(text, a)| match &a.citation {
                Some(c) => format!("{text} {}", self.citation(c)),
                None => text,
            })
            .collect();
        format!("({})", parts.join(", "))
    }

    fn block(&self, b: &Block) -> String {
        let mut s = String::from("{ ");
        for l in &b.lets {
            let line = self.let_line(l, "");
            s.push_str(&format!(
                "{} = {}; ",
                line.lhs,
                line.rest.unwrap_or_default()
            ));
        }
        s.push_str(&self.expr(&b.tail));
        s.push_str(" }");
        s
    }

    fn expr(&self, e: &Expr) -> String {
        match &e.kind {
            // Literals are copied from the source: authored units and forms are preserved.
            ExprKind::Number(_) | ExprKind::Quantity(_) => e.span.slice(self.src).to_owned(),
            ExprKind::Str(s) => quote(s),
            ExprKind::Bool(b) => b.to_string(),
            ExprKind::Path(p) => p.text(),
            ExprKind::Paren(x) => format!("({})", self.expr(x)),
            ExprKind::Array(xs) => {
                let items: Vec<String> = xs.iter().map(|x| self.expr(x)).collect();
                format!("[{}]", items.join(", "))
            }
            ExprKind::Unary(UnOp::Neg, x) => format!("-{}", self.expr(x)),
            ExprKind::Unary(UnOp::Not, x) => format!("not {}", self.expr(x)),
            ExprKind::Binary(op, l, r) => {
                let (l, r) = (self.expr(l), self.expr(r));
                match op {
                    BinOp::Range => format!("{l}..{r}"),
                    _ => format!("{l} {} {r}", binop(*op)),
                }
            }
            ExprKind::Compare { first, rest } => {
                let mut s = self.expr(first);
                for (op, x) in rest {
                    s.push_str(&format!(" {} {}", cmpop(*op), self.expr(x)));
                }
                s
            }
            ExprKind::Satisfies(l, r) => format!("{} satisfies {}", self.expr(l), self.expr(r)),
            ExprKind::As(x, t) => format!("{} as {}", self.expr(x), self.ty(t)),
            ExprKind::Field(x, n) => format!("{}.{}", self.expr(x), n.name),
            ExprKind::Index(x, i) => format!("{}[{}]", self.expr(x), self.expr(i)),
            ExprKind::Call {
                callee,
                optional,
                args,
            } => {
                let q = if *optional { "?" } else { "" };
                format!("{}{q}{}", self.expr(callee), self.args(args))
            }
            ExprKind::Tolerance(x, terms) => {
                let ts: Vec<String> = terms
                    .iter()
                    .map(|t| {
                        let sign = match t.sign {
                            TolSign::Plus => "+",
                            TolSign::Minus => "-",
                            TolSign::PlusMinus => "+-",
                        };
                        format!("{sign}{}", self.expr(&t.value))
                    })
                    .collect();
                format!("{} [{}]", self.expr(x), ts.join(" "))
            }
            ExprKind::Position(x, d) => format!("{} [position {}]", self.expr(x), self.expr(d)),
            ExprKind::IsoClass(x, c) => format!("{} {}", self.expr(x), c.name),
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                let mut s = format!("if {} {}", self.expr(cond), self.block(then));
                if let Some(o) = otherwise {
                    s.push_str(&format!(" else {}", self.expr(o)));
                }
                s
            }
            ExprKind::Match { scrutinee, arms } => {
                let arms: Vec<String> = arms
                    .iter()
                    .map(|a| {
                        let pat = match &a.pat {
                            Pattern::Wildcard(_) => "_".to_owned(),
                            Pattern::Path(p) => p.text(),
                        };
                        format!("{pat} => {}", self.expr(&a.body))
                    })
                    .collect();
                if arms.is_empty() {
                    format!("match {} {{}}", self.expr(scrutinee))
                } else {
                    format!("match {} {{ {} }}", self.expr(scrutinee), arms.join(", "))
                }
            }
            ExprKind::Block(b) => self.block(b),
        }
    }
}

fn item_first_start(item: &Item) -> usize {
    item.docs
        .first()
        .map(|d| d.span.start)
        .or_else(|| item.attrs.first().map(|a| a.span.start))
        .unwrap_or(item.span.start)
}

fn member_first_start(m: &Member) -> usize {
    m.docs
        .first()
        .map(|d| d.span.start)
        .or_else(|| m.attrs.first().map(|a| a.span.start))
        .unwrap_or(m.span.start)
}

fn vis_prefix(v: Vis) -> String {
    match v {
        Vis::Private => String::new(),
        Vis::Pub => "pub ".to_owned(),
        Vis::PubPackage => "pub(package) ".to_owned(),
    }
}

fn cite_lhs(c: &Cite) -> String {
    format!("cite {}: {}", c.name.name, c.ty.name)
}

fn paths(ps: &[Path]) -> String {
    let v: Vec<String> = ps.iter().map(Path::text).collect();
    v.join(", ")
}

fn use_path(p: &UsePath) -> String {
    let mut s = match &p.root {
        UseRoot::Package => "package".to_owned(),
        UseRoot::SelfMod => "self".to_owned(),
        UseRoot::Super(n) => vec!["super"; *n].join("::"),
        UseRoot::Pkg(parts) => parts
            .iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>()
            .join("."),
    };
    for seg in &p.segments {
        s.push_str("::");
        s.push_str(&seg.name);
    }
    s
}

fn use_group(trees: &[UseTree]) -> String {
    let items: Vec<String> = trees
        .iter()
        .map(|t| match &t.kind {
            UseTreeKind::Leaf { alias: None } => t.name.name.clone(),
            UseTreeKind::Leaf { alias: Some(a) } => format!("{} as {}", t.name.name, a.name),
            UseTreeKind::Glob => format!("{}::*", t.name.name),
            UseTreeKind::Group(g) => format!("{}::{}", t.name.name, use_group(g)),
            UseTreeKind::Star => "*".to_owned(),
        })
        .collect();
    format!("{{{}}}", items.join(", "))
}

fn binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Pow => "^",
        BinOp::Range => "..",
        BinOp::In => "in",
        BinOp::And => "and",
        BinOp::Or => "or",
    }
}

fn cmpop(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "==",
        CmpOp::Ne => "!=",
        CmpOp::Lt => "<",
        CmpOp::Le => "<=",
        CmpOp::Gt => ">",
        CmpOp::Ge => ">=",
    }
}

/// A string literal with only the two escapes the language has (language.md 2).
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        if ch == '"' || ch == '\\' {
            out.push('\\');
        }
        out.push(ch);
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(src: &str) -> String {
        format_source(src).unwrap_or_else(|e| panic!("{e}"))
    }

    const SAMPLE: &str = "use std::fits::iso286\npub use self::parts::Tube\n\n// header\n\n/// A bracket.\n#[doc_only]\nmodel Bracket(h: Length = 3mm) : Mountable @\"ISO 1\" {\n#![strict]\n    thickness: Length=10mm   // both legs\n  width = 56mm [+-0.1mm]     #[important]\n  fillet_radius=thickness/2 // corner\n\n\n  let edge = (width - 4mm) / 2\n  bore = 12mm [+0.018mm -0] H8\n  thickness <= 20mm\n  part wall: Wall\n  part screws[2]: Screw(len = 8mm) = M4(x = 1) alt [M5, M6(y = 2)]\n  fit seat = H7/p6 on bore [-0.4mm .. 0.4mm]\n  check ok for p in parts: p.x >= 0mm \"say \\\"hi\\\"\"\n  require r1: width in 1mm..2mm @iso273\n  // dangling\n}\n\ninterface Mountable : Base {\n  size?: Length\n  size >= 1mm\n}\ncite iso: web {\n  url = \"https://x\"\n  title=\"T\"\n}\ncite a_long_name: book {\n  author = [\"A\", \"B\"]\n}\n";

    #[test]
    fn idempotent_on_sample() {
        // frob:tests crates/cadetta-syntax/src/format.rs::format_source kind=unit
        let once = fmt(SAMPLE);
        assert_eq!(fmt(&once), once, "second pass changed the text:\n{once}");
    }

    #[test]
    fn eq_signs_align_with_two_space_indent_and_units_survive() {
        // frob:tests crates/cadetta-syntax/src/format.rs::format_source kind=unit
        let out = fmt("model M {\n    a: Length=10mm\n  longer_name = 1-1/2in\n   c = 2'\n}\n");
        assert_eq!(
            out,
            "model M {\n  a: Length   = 10mm\n  longer_name = 1-1/2in\n  c           = 2'\n}\n"
        );
    }

    #[test]
    fn digest_input_is_unchanged_by_formatting() {
        // frob:tests crates/cadetta-syntax/src/format.rs::format_source kind=unit
        let before = parse(SAMPLE);
        let out = fmt(SAMPLE);
        let after = parse(&out);
        assert!(after.diags.is_empty(), "{:?}", after.diags);
        assert_eq!(shape(&before.file), shape(&after.file));
    }

    #[test]
    fn trailing_attributes_and_comments_align_in_columns() {
        // frob:tests crates/cadetta-syntax/src/format.rs::format_source kind=unit
        let out = fmt(
            "model M {\n  a = 1mm #[x] // one\n  longer = 22mm // two\n  b = 3mm #[yy, z]\n}\n",
        );
        assert_eq!(
            out,
            "model M {\n  a      = 1mm   #[x]      // one\n  longer = 22mm            // two\n  b      = 3mm   #[yy, z]\n}\n"
        );
    }

    #[test]
    fn is_formatted_reports_without_changing_anything() {
        // frob:tests crates/cadetta-syntax/src/format.rs::is_formatted kind=unit
        let messy = String::from("model M {\n a=1mm\n}\n");
        let copy = messy.clone();
        assert_eq!(is_formatted(&messy), Ok(false));
        assert_eq!(messy, copy);
        let good = fmt(&messy);
        assert_eq!(is_formatted(&good), Ok(true));
    }

    #[test]
    fn syntax_errors_and_skipped_items_are_refused() {
        // frob:tests crates/cadetta-syntax/src/format.rs::format_source kind=unit
        let skipped = "model M {\n  datum d\n}\n";
        assert!(matches!(
            format_source(skipped),
            Err(FormatError::Syntax { .. })
        ));
        assert!(format_source("model M {\n  a = \n}\n").is_err());
    }

    #[test]
    fn comments_stay_attached_and_blank_lines_collapse() {
        // frob:tests crates/cadetta-syntax/src/format.rs::format_source kind=unit
        let out =
            fmt("// top\n\n\n\nmodel M {\n\n  // lead\n  a = 1mm\n\n\n  b = 2mm\n  // tail\n}\n");
        assert_eq!(
            out,
            "// top\n\nmodel M {\n  // lead\n  a = 1mm\n\n  b = 2mm\n  // tail\n}\n"
        );
    }

    #[test]
    fn expression_forms_round_trip() {
        // frob:tests crates/cadetta-syntax/src/format.rs::format_source kind=unit
        let src = "model M {\n  a = if x < 1mm { let t = 2mm; t } else { 3mm }\n  b = match k { A => 1mm, _ => 2mm }\n  c = f?(1mm, n = 2mm)[0].g ^ 2\n  d = not (p and q) or r\n  e = [1mm, 2mm] as Foo\n  f = a <= b <= c\n  g = x in 1..2\n  h = -x\n}\n";
        let out = fmt(src);
        assert_eq!(fmt(&out), out);
    }

    // frob:ticket 01M50C0NEZM1R9KYFTRJCJY1Z8
    #[test]
    fn position_postfix_round_trips() {
        // frob:tests crates/cadetta-syntax/src/format.rs::format_source kind=unit
        let out =
            fmt("model M {\n  p = Point2(1mm, 2mm)   [position   0.1mm]\n  q = h[position]\n}\n");
        assert!(out.contains("Point2(1mm, 2mm) [position 0.1mm]"), "{out}");
        assert!(out.contains("h[position]"), "{out}");
        assert_eq!(fmt(&out), out);
    }

    const DECLS: &str = "// decls\nenum Grade { A,B,\n  C }\nkind Flow=Volume/Time\nkind Press = Pressure / VolumeFlow^2\nunit kip=1000lbf\nunit Stress   MPa\npub fn half(x: Length, y: Length = 2mm) -> Length { let t = x + y; t / 2 }\nwaive L001 on pkg::Bracket.*  for alt a.slot=Vendor::B {because \"fine\"   expires 2027-06-30}\nwaive L002 on **{ because \"x\" }\ntable Bolts @\"ISO 4762\" {\n  key dia: Length\n  cols  len: Length, mass: Mass = 1g\n  3mm => 10mm,1g\n  /// four\n  4mm   =>   12mm, 2g\n  5mm\n}\nprocess Print @\"cal\" {\n  hole_comp=0.2mm [+-0.05mm]\n  rule drill(d: Length) -> Length = d + 0.1mm\n}\nsolver Beam {\n  script \"beam.py\"\n  in load = 1kN, span=2m\n  // out\n  out defl: Length / Force\n  deps \"a.csv\"\n}\nmodel M {\n  #[x] solver S {\n  script \"s.py\"\n   in a = 1mm\n  } #[t]\n  priv solver T {\n  script \"t.py\"\n  }\n  q: Volume / Time = 1m\n}\n";

    #[test]
    fn new_item_kinds_are_idempotent_and_shape_preserving() {
        // frob:tests crates/cadetta-syntax/src/format.rs::format_source kind=unit
        let out = fmt(DECLS);
        assert_eq!(fmt(&out), out, "second pass changed:\n{out}");
        assert!(out.contains("enum Grade { A, B, C }\n"), "{out}");
        assert!(
            out.contains("kind Press = Pressure / VolumeFlow^2\n"),
            "{out}"
        );
        assert!(out.contains("  3mm => 10mm, 1g\n"), "{out}");
        assert!(out.contains("  /// four\n  4mm => 12mm, 2g\n"), "{out}");
        assert!(
            out.contains("waive L002 on ** { because \"x\" }\n"),
            "{out}"
        );
        assert!(
            out.contains("for alt a.slot = Vendor::B { because \"fine\" expires 2027-06-30 }"),
            "{out}"
        );
        assert!(out.contains("  }  #[t]\n"), "{out}");
        assert!(out.contains("kind Flow  = Volume / Time\n"), "{out}");
    }
}
