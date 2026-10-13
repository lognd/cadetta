//! The hand-written recursive-descent parser (language.md 3 and 5).
//!
//! Total: [`parse`] never fails. A syntax error becomes an E001 diagnostic plus an
//! `Error` node, and parsing resumes at the next statement boundary, so one mistake
//! does not hide the rest of the file. Statements end at a newline or `;`; a newline
//! inside `( )` or `[ ]` does not end one (language.md 2).

// frob:ticket 01M4SPWP1JK8XH8EKXK71P7EFH

use crate::ast::{
    Arg, Attr, AttrArg, BinOp, Block, Bound, BoundKind, Check, Citation, Cite, CiteName,
    CiteTarget, CmpOp, Component, ComponentClass, Ctor, Doc, Expr, ExprKind, File, FileEntry, Fit,
    FitSpec, Ident, Interface, InterfaceMember, InterfaceMemberKind, Item, ItemKind, Let, MatchArm,
    Member, MemberKind, Param, ParamDecl, Part, Path, PathSep, Pattern, Require, TolSign, TolTerm,
    Type, TypeKind, UnOp, Use, UseKind, UsePath, UseRoot, UseTree, UseTreeKind, Vis,
};
use crate::lexer::{Comment, CommentKind, DiagCode, Diagnostic, Punct, Span, Tok, Token, lex};

/// The deepest expression or type nesting accepted (a bound on recursion, not a language rule).
pub const MAX_DEPTH: usize = 128;

/// The result of [`parse`]: the tree plus every lexical and syntax diagnostic.
#[derive(Clone, Debug, Default)]
pub struct Parsed {
    /// The syntax tree (always present; broken parts are `Error` nodes).
    pub file: File,
    /// Comments in source order, for the formatter.
    pub comments: Vec<Comment>,
    /// Lexical diagnostics first, then syntax errors in source order.
    pub diags: Vec<Diagnostic>,
}

/// A syntax error being propagated to the nearest recovery point.
#[derive(Clone, PartialEq, Eq, Debug)]
struct ParseErr {
    span: Span,
    message: String,
}

type PResult<T> = Result<T, ParseErr>;

/// Parses one .cdt file; total (never fails).
pub fn parse(src: &str) -> Parsed {
    let lexed = lex(src);
    let mut toks: Vec<Token> = lexed
        .tokens
        .into_iter()
        .filter(|t| t.tok != Tok::Bad)
        .collect();
    if !matches!(toks.last(), Some(Token { tok: Tok::Eof, .. })) {
        toks.push(Token {
            tok: Tok::Eof,
            span: Span::new(src.len(), src.len()),
        });
    }
    let mut p = Parser {
        toks,
        pos: 0,
        nest: 0,
        depth: 0,
        last_end: 0,
        comments: &lexed.comments,
        doc_idx: 0,
        diags: Vec::new(),
    };
    let mut file = p.file();
    file.span = Span::new(0, src.len());
    let mut diags = lexed.diags;
    diags.append(&mut p.diags);
    tracing::info!(
        entries = file.entries.len(),
        diags = diags.len(),
        "parsed file"
    );
    Parsed {
        file,
        comments: lexed.comments.clone(),
        diags,
    }
}

/// True for the shape of an ISO 286 class name: one or two letters then one or two digits.
fn is_class_name(s: &str) -> bool {
    let letters = s.bytes().take_while(u8::is_ascii_alphabetic).count();
    let digits = s.len() - letters;
    (1..=2).contains(&letters)
        && (1..=2).contains(&digits)
        && s.bytes().skip(letters).all(|b| b.is_ascii_digit())
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Ident(n) => format!("`{n}`"),
        Tok::Keyword(k) => format!("keyword `{k}`"),
        Tok::Number(_) | Tok::Quantity(_) => "a number".to_owned(),
        Tok::Str(_) => "a string".to_owned(),
        Tok::P(p) => format!("`{}`", p.text()),
        Tok::Newline => "end of line".to_owned(),
        Tok::Eof => "end of file".to_owned(),
        Tok::Bad => "an invalid token".to_owned(),
    }
}

struct Parser<'a> {
    toks: Vec<Token>,
    pos: usize,
    /// Greater than zero inside `( )` or `[ ]`, where newlines are insignificant.
    nest: usize,
    depth: usize,
    last_end: usize,
    comments: &'a [Comment],
    doc_idx: usize,
    diags: Vec<Diagnostic>,
}

impl Parser<'_> {
    // ---- token access ----

    /// Index of the current token, skipping newlines when they are insignificant.
    fn idx(&self) -> usize {
        let mut i = self.pos;
        if self.nest > 0 {
            while matches!(self.toks[i].tok, Tok::Newline) {
                i += 1;
            }
        }
        i
    }

    fn tok(&self) -> &Tok {
        &self.toks[self.idx()].tok
    }

    fn nth_tok(&self, n: usize) -> &Tok {
        let last = self.toks.len() - 1;
        let mut i = self.idx();
        for _ in 0..n {
            if i >= last {
                break;
            }
            i += 1;
            if self.nest > 0 {
                while matches!(self.toks[i].tok, Tok::Newline) {
                    i += 1;
                }
            }
        }
        &self.toks[i].tok
    }

    fn span(&self) -> Span {
        self.toks[self.idx()].span
    }

    fn start(&self) -> usize {
        self.span().start
    }

    fn bump(&mut self) -> Token {
        let i = self.idx();
        let t = self.toks[i].clone();
        if !matches!(t.tok, Tok::Eof) {
            self.pos = i + 1;
        }
        self.last_end = t.span.end;
        t
    }

    fn is_p(&self, p: Punct) -> bool {
        matches!(self.tok(), Tok::P(q) if *q == p)
    }

    fn nth_is_p(&self, n: usize, p: Punct) -> bool {
        matches!(self.nth_tok(n), Tok::P(q) if *q == p)
    }

    fn is_kw(&self, k: &str) -> bool {
        matches!(self.tok(), Tok::Keyword(x) if *x == k)
    }

    fn nth_is_kw(&self, n: usize, k: &str) -> bool {
        matches!(self.nth_tok(n), Tok::Keyword(x) if *x == k)
    }

    fn eat_p(&mut self, p: Punct) -> bool {
        let hit = self.is_p(p);
        if hit {
            self.bump();
        }
        hit
    }

    fn eat_kw(&mut self, k: &str) -> bool {
        let hit = self.is_kw(k);
        if hit {
            self.bump();
        }
        hit
    }

    fn err<T>(&self, message: impl Into<String>) -> PResult<T> {
        Err(ParseErr {
            span: self.span(),
            message: message.into(),
        })
    }

    fn expect_p(&mut self, p: Punct) -> PResult<Span> {
        if self.is_p(p) {
            return Ok(self.bump().span);
        }
        self.err(format!(
            "expected `{}`, found {}",
            p.text(),
            describe(self.tok())
        ))
    }

    fn expect_kw(&mut self, k: &str) -> PResult<Span> {
        if self.is_kw(k) {
            return Ok(self.bump().span);
        }
        self.err(format!("expected `{k}`, found {}", describe(self.tok())))
    }

    fn expect_ident(&mut self) -> PResult<Ident> {
        match self.tok().clone() {
            Tok::Ident(name) => {
                let span = self.bump().span;
                Ok(Ident { name, span })
            }
            Tok::Keyword(k) => self.err(format!(
                "`{k}` is a reserved keyword and cannot be used as a name"
            )),
            other => self.err(format!("expected a name, found {}", describe(&other))),
        }
    }

    fn expect_str(&mut self) -> PResult<String> {
        if let Tok::Str(s) = self.tok().clone() {
            self.bump();
            return Ok(s);
        }
        self.err(format!("expected a string, found {}", describe(self.tok())))
    }

    fn span_from(&self, start: usize) -> Span {
        Span::new(start, self.last_end)
    }

    fn enter(&mut self) -> PResult<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.err("nesting is too deep");
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    // ---- statement structure ----

    /// Consumes newlines and `;` between statements.
    fn skip_blank(&mut self) {
        while matches!(self.toks[self.pos].tok, Tok::Newline | Tok::P(Punct::Semi)) {
            self.bump();
        }
    }

    /// A statement ends at a newline, `;`, or just before `}` or end of file.
    fn end_stmt(&mut self) -> PResult<()> {
        match self.tok() {
            Tok::Newline | Tok::P(Punct::Semi) => {
                self.bump();
                Ok(())
            }
            Tok::P(Punct::RBrace) | Tok::Eof => Ok(()),
            other => {
                let msg = format!(
                    "expected a newline or `;` after the statement, found {}",
                    describe(other)
                );
                self.err(msg)
            }
        }
    }

    /// Records `e` and resynchronises at the next statement boundary.
    fn recover(&mut self, e: &ParseErr, start_pos: usize) {
        tracing::debug!(span = %e.span, message = %e.message, "syntax error, recovering");
        self.diags.push(Diagnostic {
            code: DiagCode::E001,
            span: e.span,
            message: e.message.clone(),
            fix: None,
        });
        self.nest = 0;
        self.depth = 0;
        let mut braces = 0usize;
        loop {
            match self.toks[self.pos].tok {
                Tok::Eof => break,
                Tok::Newline | Tok::P(Punct::Semi) if braces == 0 => {
                    self.bump();
                    break;
                }
                Tok::P(Punct::RBrace) if braces == 0 => break,
                Tok::P(Punct::LBrace) => braces += 1,
                Tok::P(Punct::RBrace) => braces -= 1,
                _ => {}
            }
            self.bump();
        }
        // Guarantee progress so a loop over statements always terminates.
        if self.pos == start_pos
            && !matches!(self.toks[self.pos].tok, Tok::P(Punct::RBrace) | Tok::Eof)
        {
            self.bump();
        }
    }

    /// Doc comments that start before byte `upto` and were not yet claimed.
    fn take_docs(&mut self, upto: usize) -> Vec<Doc> {
        let mut docs = Vec::new();
        while let Some(c) = self.comments.get(self.doc_idx) {
            if c.span.start >= upto {
                break;
            }
            if c.kind == CommentKind::Doc {
                docs.push(Doc {
                    text: c.doc_text().to_owned(),
                    span: c.span,
                });
            }
            self.doc_idx += 1;
        }
        docs
    }

    // ---- file and items ----

    fn file(&mut self) -> File {
        let mut file = File::default();
        loop {
            self.skip_blank();
            if matches!(self.tok(), Tok::Eof) {
                break;
            }
            let start_pos = self.pos;
            if self.is_p(Punct::RBrace) {
                let span = self.span();
                let e = ParseErr {
                    span,
                    message: "unexpected `}` with no open block".to_owned(),
                };
                self.recover(&e, start_pos);
                self.bump_if_stuck(start_pos);
                continue;
            }
            match self.entry(&mut file) {
                Ok(Some(entry)) => file.entries.push(entry),
                Ok(None) => {}
                Err(e) => {
                    let start = e.span.start;
                    self.recover(&e, start_pos);
                    file.entries.push(FileEntry::Item(Box::new(Item {
                        docs: Vec::new(),
                        attrs: Vec::new(),
                        vis: Vis::Private,
                        kind: ItemKind::Error,
                        span: Span::new(start, self.last_end),
                    })));
                }
            }
        }
        file
    }

    fn bump_if_stuck(&mut self, start_pos: usize) {
        if self.pos == start_pos && !matches!(self.tok(), Tok::Eof) {
            self.bump();
        }
    }

    /// One `use` or item; `Ok(None)` for a file-level inner attribute (stored in `file`).
    fn entry(&mut self, file: &mut File) -> PResult<Option<FileEntry>> {
        let docs = self.take_docs(self.start());
        let mut attrs = Vec::new();
        loop {
            if self.is_p(Punct::HashBangBracket) {
                file.inner_attrs.extend(self.attr_group()?);
                self.end_stmt()?;
                self.skip_blank();
                if attrs.is_empty() && docs.is_empty() && !matches!(self.tok(), Tok::Eof) {
                    continue;
                }
            }
            if self.is_p(Punct::HashBracket) {
                attrs.extend(self.attr_group()?);
                while matches!(self.toks[self.pos].tok, Tok::Newline) {
                    self.bump();
                }
                continue;
            }
            break;
        }
        if matches!(self.tok(), Tok::Eof) {
            return Ok(None);
        }
        let start = self.start();
        let vis = self.vis()?;
        if self.is_kw("use") {
            let u = self.use_decl(vis, start)?;
            self.end_stmt()?;
            return Ok(Some(FileEntry::Use(u)));
        }
        let kind = match self.tok().clone() {
            Tok::Keyword("cite") => ItemKind::Cite(self.cite()?),
            Tok::Keyword("interface") => ItemKind::Interface(self.interface()?),
            Tok::Keyword("ots") => ItemKind::Component(self.component(ComponentClass::Ots)?),
            Tok::Keyword("model") => ItemKind::Component(self.component(ComponentClass::Model)?),
            Tok::Keyword("assembly") => {
                ItemKind::Component(self.component(ComponentClass::Assembly)?)
            }
            Tok::Keyword("fit") => ItemKind::Fit(self.fit()?),
            Tok::Keyword("require") => ItemKind::Require(self.require()?),
            Tok::Keyword("check") => ItemKind::Check(self.check()?),
            Tok::Keyword(
                k @ ("process" | "table" | "enum" | "fn" | "solver" | "waive" | "unit" | "kind"),
            ) => {
                return self.err(format!("`{k}` items are not parsed yet"));
            }
            other => {
                return self.err(format!("expected an item, found {}", describe(&other)));
            }
        };
        self.end_stmt()?;
        Ok(Some(FileEntry::Item(Box::new(Item {
            docs,
            attrs,
            vis,
            kind,
            span: self.span_from(start),
        }))))
    }

    fn vis(&mut self) -> PResult<Vis> {
        if !self.eat_kw("pub") {
            return Ok(Vis::Private);
        }
        if self.is_p(Punct::LParen) && self.nth_is_kw(1, "package") {
            self.bump();
            self.bump();
            self.expect_p(Punct::RParen)?;
            return Ok(Vis::PubPackage);
        }
        Ok(Vis::Pub)
    }

    fn cite(&mut self) -> PResult<Cite> {
        self.expect_kw("cite")?;
        let name = self.expect_ident()?;
        self.expect_p(Punct::Eq)?;
        let target = if let Tok::Str(s) = self.tok().clone() {
            self.bump();
            CiteTarget::Str(s)
        } else {
            CiteTarget::Path(self.path(false, false)?)
        };
        Ok(Cite { name, target })
    }

    // ---- use ----

    fn use_decl(&mut self, vis: Vis, start: usize) -> PResult<Use> {
        self.expect_kw("use")?;
        let path_start = self.start();
        let root = if self.eat_kw("package") {
            UseRoot::Package
        } else if self.eat_kw("self") {
            UseRoot::SelfMod
        } else if self.eat_kw("super") {
            let mut n = 1;
            while self.is_p(Punct::ColonColon) && self.nth_is_kw(1, "super") {
                self.bump();
                self.bump();
                n += 1;
            }
            UseRoot::Super(n)
        } else {
            let mut parts = vec![self.expect_ident()?];
            while self.is_p(Punct::Dot) {
                self.bump();
                parts.push(self.expect_ident()?);
            }
            UseRoot::Pkg(parts)
        };
        let mut segments = Vec::new();
        let mut kind = None;
        while self.is_p(Punct::ColonColon) {
            self.bump();
            match self.tok() {
                Tok::P(Punct::Star) => {
                    self.bump();
                    kind = Some(UseKind::Glob);
                    break;
                }
                Tok::P(Punct::LBrace) => {
                    kind = Some(UseKind::Tree(self.use_group()?));
                    break;
                }
                _ => segments.push(self.expect_ident()?),
            }
        }
        let path = UsePath {
            root,
            segments,
            span: self.span_from(path_start),
        };
        let kind = match kind {
            Some(k) => k,
            None => {
                let alias = if self.eat_kw("as") {
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                UseKind::Single { alias }
            }
        };
        Ok(Use {
            vis,
            path,
            kind,
            span: self.span_from(start),
        })
    }

    fn use_group(&mut self) -> PResult<Vec<UseTree>> {
        self.expect_p(Punct::LBrace)?;
        self.nest += 1;
        let mut trees = Vec::new();
        while !self.is_p(Punct::RBrace) {
            trees.push(self.use_tree()?);
            if !self.eat_p(Punct::Comma) {
                break;
            }
        }
        self.expect_p(Punct::RBrace)?;
        self.nest -= 1;
        Ok(trees)
    }

    fn use_tree(&mut self) -> PResult<UseTree> {
        let start = self.start();
        let name = self.expect_ident()?;
        let kind = if self.eat_p(Punct::ColonColon) {
            if self.eat_p(Punct::Star) {
                UseTreeKind::Glob
            } else if self.is_p(Punct::LBrace) {
                UseTreeKind::Group(self.use_group()?)
            } else {
                return self.err("expected `*` or `{` after `::` in a use tree");
            }
        } else if self.eat_kw("as") {
            UseTreeKind::Leaf {
                alias: Some(self.expect_ident()?),
            }
        } else {
            UseTreeKind::Leaf { alias: None }
        };
        Ok(UseTree {
            name,
            kind,
            span: self.span_from(start),
        })
    }

    // ---- attributes and citations ----

    /// `#[a, b(c)]` or `#![...]`.
    fn attr_group(&mut self) -> PResult<Vec<Attr>> {
        let inner = self.is_p(Punct::HashBangBracket);
        self.bump();
        self.nest += 1;
        let mut attrs = Vec::new();
        loop {
            attrs.push(self.attr(inner)?);
            if !self.eat_p(Punct::Comma) || self.is_p(Punct::RBracket) {
                break;
            }
        }
        self.expect_p(Punct::RBracket)?;
        self.nest -= 1;
        Ok(attrs)
    }

    fn attr_name(&mut self) -> PResult<Ident> {
        if let Tok::Keyword(k) = self.tok().clone() {
            let span = self.bump().span;
            return Ok(Ident {
                name: k.to_owned(),
                span,
            });
        }
        self.expect_ident()
    }

    fn attr(&mut self, inner: bool) -> PResult<Attr> {
        let start = self.start();
        let name = self.attr_name()?;
        let mut args = Vec::new();
        if self.eat_p(Punct::LParen) {
            self.nest += 1;
            while !self.is_p(Punct::RParen) {
                let named = matches!(self.tok(), Tok::Ident(_) | Tok::Keyword(_))
                    && self.nth_is_p(1, Punct::Eq);
                if named {
                    let n = self.attr_name()?;
                    self.bump();
                    let value = self.expr()?;
                    args.push(AttrArg::Named { name: n, value });
                } else {
                    args.push(AttrArg::Expr(self.expr()?));
                }
                if !self.eat_p(Punct::Comma) {
                    break;
                }
            }
            self.expect_p(Punct::RParen)?;
            self.nest -= 1;
        }
        Ok(Attr {
            name,
            args,
            inner,
            span: self.span_from(start),
        })
    }

    fn citation(&mut self) -> PResult<Citation> {
        let start = self.start();
        self.expect_p(Punct::At)?;
        let name = if let Tok::Str(s) = self.tok().clone() {
            self.bump();
            CiteName::Literal(s)
        } else {
            CiteName::Named(self.expect_ident()?)
        };
        let mut locator = None;
        if self.is_p(Punct::Colon) && matches!(self.nth_tok(1), Tok::Str(_)) {
            self.bump();
            locator = Some(self.expect_str()?);
        }
        Ok(Citation {
            name,
            locator,
            span: self.span_from(start),
        })
    }

    // ---- paths and types ----

    fn path(&mut self, allow_dot: bool, allow_kw_root: bool) -> PResult<Path> {
        let start = self.start();
        let first = match self.tok().clone() {
            Tok::Keyword(k @ ("self" | "origin" | "package" | "super")) if allow_kw_root => {
                let span = self.bump().span;
                Ident {
                    name: k.to_owned(),
                    span,
                }
            }
            _ => self.expect_ident()?,
        };
        let mut segments = vec![first];
        let mut seps = Vec::new();
        loop {
            let sep = if self.is_p(Punct::ColonColon) {
                PathSep::ColonColon
            } else if allow_dot && self.is_p(Punct::Dot) {
                PathSep::Dot
            } else {
                break;
            };
            if !matches!(self.nth_tok(1), Tok::Ident(_)) {
                break;
            }
            self.bump();
            seps.push(sep);
            segments.push(self.expect_ident()?);
        }
        Ok(Path {
            segments,
            seps,
            span: self.span_from(start),
        })
    }

    fn ty(&mut self) -> PResult<Type> {
        self.enter()?;
        let start = self.start();
        let kind = if self.eat_p(Punct::LBracket) {
            self.nest += 1;
            let elem = Box::new(self.ty()?);
            self.expect_p(Punct::Semi)?;
            let len = Box::new(self.expr()?);
            self.expect_p(Punct::RBracket)?;
            self.nest -= 1;
            TypeKind::Array { elem, len }
        } else {
            let base = self.path(false, false)?;
            if self.eat_kw("on") {
                let datum = self.path(true, true)?;
                TypeKind::OnDatum { base, datum }
            } else if base.text() == "Table" && self.eat_p(Punct::Lt) {
                let inner = self.ty()?;
                self.expect_p(Punct::Gt)?;
                TypeKind::Table(Box::new(inner))
            } else {
                TypeKind::Path(base)
            }
        };
        self.leave();
        Ok(Type {
            kind,
            span: self.span_from(start),
        })
    }

    // ---- components ----

    fn iface_list(&mut self) -> PResult<Vec<Path>> {
        let mut list = vec![self.path(false, false)?];
        while self.eat_p(Punct::Comma) {
            list.push(self.path(false, false)?);
        }
        Ok(list)
    }

    fn interface(&mut self) -> PResult<Interface> {
        self.expect_kw("interface")?;
        let name = self.expect_ident()?;
        let extends = if self.eat_p(Punct::Colon) {
            self.iface_list()?
        } else {
            Vec::new()
        };
        self.expect_p(Punct::LBrace)?;
        tracing::debug!(name = %name.name, "parsing interface");
        let members = self.body(Self::interface_member, |span| InterfaceMember {
            docs: Vec::new(),
            attrs: Vec::new(),
            kind: InterfaceMemberKind::Error,
            span,
        });
        Ok(Interface {
            name,
            extends,
            members,
        })
    }

    fn interface_member(&mut self) -> PResult<InterfaceMember> {
        let docs = self.take_docs(self.start());
        let attrs = self.leading_attrs()?;
        let start = self.start();
        let kind = match (self.tok().clone(), self.nth_tok(1).clone()) {
            (Tok::Ident(_), Tok::P(Punct::Colon | Punct::Question)) => {
                let name = self.expect_ident()?;
                let optional = self.eat_p(Punct::Question);
                self.expect_p(Punct::Colon)?;
                let ty = self.ty()?;
                InterfaceMemberKind::Field { name, optional, ty }
            }
            (Tok::Ident(_), _) => InterfaceMemberKind::Bound(Box::new(self.bound()?)),
            (Tok::Keyword("datum"), _) => return self.err("`datum` members are not parsed yet"),
            (other, _) => {
                return self.err(format!(
                    "expected an interface member, found {}",
                    describe(&other)
                ));
            }
        };
        let span = self.span_from(start);
        self.end_stmt()?;
        Ok(InterfaceMember {
            docs,
            attrs,
            kind,
            span,
        })
    }

    fn pdecl(&mut self) -> PResult<ParamDecl> {
        let start = self.start();
        let name = self.expect_ident()?;
        self.expect_p(Punct::Colon)?;
        let ty = self.ty()?;
        let default = if self.eat_p(Punct::Eq) {
            Some(self.expr()?)
        } else {
            None
        };
        Ok(ParamDecl {
            name,
            ty,
            default,
            span: self.span_from(start),
        })
    }

    fn component(&mut self, class: ComponentClass) -> PResult<Component> {
        self.bump();
        let name = self.expect_ident()?;
        let mut params = Vec::new();
        if self.eat_p(Punct::LParen) {
            self.nest += 1;
            while !self.is_p(Punct::RParen) {
                params.push(self.pdecl()?);
                if !self.eat_p(Punct::Comma) {
                    break;
                }
            }
            self.expect_p(Punct::RParen)?;
            self.nest -= 1;
        }
        let implements = if self.eat_p(Punct::Colon) {
            self.iface_list()?
        } else {
            Vec::new()
        };
        let citation = if self.is_p(Punct::At) {
            Some(self.citation()?)
        } else {
            None
        };
        self.expect_p(Punct::LBrace)?;
        tracing::debug!(name = %name.name, ?class, "parsing component");
        self.skip_blank();
        let mut inner_attrs = Vec::new();
        while self.is_p(Punct::HashBangBracket) {
            inner_attrs.extend(self.attr_group()?);
            self.end_stmt()?;
            self.skip_blank();
        }
        let members = self.body(Self::member, |span| Member {
            docs: Vec::new(),
            attrs: Vec::new(),
            private: false,
            kind: MemberKind::Error,
            trailing: Vec::new(),
            span,
        });
        Ok(Component {
            class,
            name,
            params,
            implements,
            citation,
            inner_attrs,
            members,
        })
    }

    /// Reads statements until the closing `}` (already past the opening one), with recovery.
    fn body<T>(&mut self, one: fn(&mut Self) -> PResult<T>, error: fn(Span) -> T) -> Vec<T> {
        let mut out = Vec::new();
        loop {
            self.skip_blank();
            match self.tok() {
                Tok::P(Punct::RBrace) => {
                    self.bump();
                    break;
                }
                Tok::Eof => {
                    let span = self.span();
                    tracing::debug!(%span, "unclosed block");
                    self.diags.push(Diagnostic {
                        code: DiagCode::E001,
                        span,
                        message: "expected `}` before the end of the file".to_owned(),
                        fix: None,
                    });
                    break;
                }
                _ => {}
            }
            let start_pos = self.pos;
            let start = self.start();
            match one(self) {
                Ok(v) => out.push(v),
                Err(e) => {
                    self.recover(&e, start_pos);
                    out.push(error(Span::new(start, self.last_end.max(start))));
                }
            }
        }
        out
    }

    fn leading_attrs(&mut self) -> PResult<Vec<Attr>> {
        let mut attrs = Vec::new();
        while self.is_p(Punct::HashBracket) {
            attrs.extend(self.attr_group()?);
            while matches!(self.toks[self.pos].tok, Tok::Newline) {
                self.bump();
            }
        }
        Ok(attrs)
    }

    fn member(&mut self) -> PResult<Member> {
        let docs = self.take_docs(self.start());
        let attrs = self.leading_attrs()?;
        let start = self.start();
        let private = self.eat_kw("priv");
        let kind = match (self.tok().clone(), self.nth_tok(1).clone()) {
            (Tok::Keyword("let"), _) => MemberKind::Let(self.let_stmt()?),
            (Tok::Keyword("part"), _) => MemberKind::Part(self.part()?),
            (Tok::Keyword("fit"), _) => MemberKind::Fit(self.fit()?),
            (Tok::Keyword("check"), _) => MemberKind::Check(self.check()?),
            (Tok::Keyword("require"), _) => MemberKind::Require(self.require()?),
            (Tok::Keyword(k @ ("datum" | "place" | "solver")), _) => {
                return self.err(format!("`{k}` members are not parsed yet"));
            }
            (Tok::Ident(_), Tok::P(Punct::Colon | Punct::Eq)) => MemberKind::Param(self.param()?),
            (
                Tok::Ident(_),
                Tok::P(Punct::Le | Punct::Ge | Punct::Dot | Punct::ColonColon | Punct::EqEq)
                | Tok::Keyword("in"),
            ) => MemberKind::Bound(Box::new(self.bound()?)),
            (Tok::Ident(_), other) => {
                return self.err(format!(
                    "expected `=`, `:` or a bound after the name, found {}",
                    describe(&other)
                ));
            }
            (other, _) => {
                return self.err(format!("expected a member, found {}", describe(&other)));
            }
        };
        let span = self.span_from(start);
        let mut trailing = Vec::new();
        while self.is_p(Punct::HashBracket) {
            trailing.extend(self.attr_group()?);
        }
        self.end_stmt()?;
        Ok(Member {
            docs,
            attrs,
            private,
            kind,
            trailing,
            span,
        })
    }

    fn param(&mut self) -> PResult<Param> {
        let name = self.expect_ident()?;
        let ty = if self.eat_p(Punct::Colon) {
            Some(self.ty()?)
        } else {
            None
        };
        self.expect_p(Punct::Eq)?;
        let value = self.expr()?;
        let citation = if self.is_p(Punct::At) {
            Some(self.citation()?)
        } else {
            None
        };
        let mut was = Vec::new();
        while self.eat_kw("was") {
            was.push(self.path(true, false)?);
        }
        Ok(Param {
            name,
            ty,
            value,
            citation,
            was,
        })
    }

    fn let_stmt(&mut self) -> PResult<Let> {
        let start = self.start();
        self.expect_kw("let")?;
        let name = self.expect_ident()?;
        let ty = if self.eat_p(Punct::Colon) {
            Some(self.ty()?)
        } else {
            None
        };
        self.expect_p(Punct::Eq)?;
        let value = self.expr()?;
        Ok(Let {
            name,
            ty,
            value,
            span: self.span_from(start),
        })
    }

    fn bound(&mut self) -> PResult<Bound> {
        let start = self.start();
        let kind = match self.nth_tok(1).clone() {
            Tok::P(op @ (Punct::Le | Punct::Ge)) => {
                let name = self.expect_ident()?;
                self.bump();
                let value = self.expr()?;
                if op == Punct::Le {
                    BoundKind::Le { name, value }
                } else {
                    BoundKind::Ge { name, value }
                }
            }
            Tok::Keyword("in") => {
                let name = self.expect_ident()?;
                self.bump();
                let lo = self.add()?;
                if !self.is_p(Punct::DotDot) {
                    return self.err("a bound with `in` needs a range `lo..hi`");
                }
                self.bump();
                let hi = self.add()?;
                BoundKind::In { name, lo, hi }
            }
            _ => {
                let lhs = self.path(true, false)?;
                self.expect_p(Punct::EqEq)?;
                let rhs = self.path(true, false)?;
                BoundKind::Eq { lhs, rhs }
            }
        };
        Ok(Bound {
            kind,
            span: self.span_from(start),
        })
    }

    fn part(&mut self) -> PResult<Part> {
        self.expect_kw("part")?;
        let name = self.expect_ident()?;
        let count = if self.eat_p(Punct::LBracket) {
            self.nest += 1;
            let n = self.expr()?;
            self.expect_p(Punct::RBracket)?;
            self.nest -= 1;
            Some(n)
        } else {
            None
        };
        self.expect_p(Punct::Colon)?;
        let ty = self.ty()?;
        let args = if matches!(ty.kind, TypeKind::Path(_)) && self.is_p(Punct::LParen) {
            Some(self.args()?)
        } else {
            None
        };
        let mut primary = None;
        let mut alts = Vec::new();
        if self.eat_p(Punct::Eq) {
            primary = Some(self.ctor()?);
            if self.eat_kw("alt") {
                self.expect_p(Punct::LBracket)?;
                self.nest += 1;
                while !self.is_p(Punct::RBracket) {
                    alts.push(self.ctor()?);
                    if !self.eat_p(Punct::Comma) {
                        break;
                    }
                }
                self.expect_p(Punct::RBracket)?;
                self.nest -= 1;
            }
        }
        Ok(Part {
            name,
            count,
            ty,
            args,
            primary,
            alts,
        })
    }

    fn ctor(&mut self) -> PResult<Ctor> {
        let start = self.start();
        let path = self.path(false, false)?;
        let args = if self.is_p(Punct::LParen) {
            Some(self.args()?)
        } else {
            None
        };
        Ok(Ctor {
            path,
            args,
            span: self.span_from(start),
        })
    }

    /// `( arg, name = arg, ... )`
    fn args(&mut self) -> PResult<Vec<Arg>> {
        self.expect_p(Punct::LParen)?;
        self.nest += 1;
        let mut args = Vec::new();
        while !self.is_p(Punct::RParen) {
            let start = self.start();
            let name = if matches!(self.tok(), Tok::Ident(_)) && self.nth_is_p(1, Punct::Eq) {
                let n = self.expect_ident()?;
                self.bump();
                Some(n)
            } else {
                None
            };
            let value = self.expr()?;
            args.push(Arg {
                name,
                value,
                span: self.span_from(start),
            });
            if !self.eat_p(Punct::Comma) {
                break;
            }
        }
        self.expect_p(Punct::RParen)?;
        self.nest -= 1;
        Ok(args)
    }

    fn fit(&mut self) -> PResult<Fit> {
        self.expect_kw("fit")?;
        let name = self.expect_ident()?;
        self.expect_p(Punct::Eq)?;
        let first = self.path(true, false)?;
        let single_class = |p: &Path| p.segments.len() == 1 && is_class_name(&p.segments[0].name);
        let spec = if self.eat_p(Punct::Slash) {
            let second = self.path(true, false)?;
            if single_class(&first) && single_class(&second) {
                let [a] = <[Ident; 1]>::try_from(first.segments).expect("single class");
                let [b] = <[Ident; 1]>::try_from(second.segments).expect("single class");
                FitSpec::Pair(a, b)
            } else {
                FitSpec::Paths(first, second)
            }
        } else if single_class(&first) {
            let [a] = <[Ident; 1]>::try_from(first.segments).expect("single class");
            FitSpec::Class(a)
        } else {
            return Err(ParseErr {
                span: first.span,
                message: "expected an ISO class such as `p6` or a `path / path` pair".to_owned(),
            });
        };
        self.expect_kw("on")?;
        let on = self.path(true, true)?;
        let range = if self.is_p(Punct::LBracket) {
            self.bump();
            self.nest += 1;
            let lo = self.add()?;
            self.expect_p(Punct::DotDot)?;
            let hi = self.add()?;
            self.expect_p(Punct::RBracket)?;
            self.nest -= 1;
            Some((lo, hi))
        } else {
            None
        };
        Ok(Fit {
            name,
            spec,
            on,
            range,
        })
    }

    fn check(&mut self) -> PResult<Check> {
        self.expect_kw("check")?;
        let name = self.expect_ident()?;
        let each = if self.eat_kw("for") {
            let var = self.expect_ident()?;
            self.expect_kw("in")?;
            Some((var, self.expr()?))
        } else {
            None
        };
        self.expect_p(Punct::Colon)?;
        let cond = self.expr()?;
        let message = if matches!(self.tok(), Tok::Str(_)) {
            Some(self.expect_str()?)
        } else {
            None
        };
        Ok(Check {
            name,
            each,
            cond,
            message,
        })
    }

    fn require(&mut self) -> PResult<Require> {
        self.expect_kw("require")?;
        let name = self.expect_ident()?;
        self.expect_p(Punct::Colon)?;
        let cond = self.expr()?;
        let message = if matches!(self.tok(), Tok::Str(_)) {
            Some(self.expect_str()?)
        } else {
            None
        };
        let citation = if self.is_p(Punct::At) {
            Some(self.citation()?)
        } else {
            None
        };
        Ok(Require {
            name,
            cond,
            message,
            citation,
        })
    }

    // ---- expressions (language.md 5), lowest precedence first ----

    fn mk(&self, kind: ExprKind, start: usize) -> Expr {
        Expr {
            kind,
            span: self.span_from(start),
        }
    }

    fn expr(&mut self) -> PResult<Expr> {
        self.enter()?;
        let r = self.or();
        self.leave();
        r
    }

    fn or(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut l = self.and()?;
        while self.eat_kw("or") {
            let r = self.and()?;
            l = self.mk(ExprKind::Binary(BinOp::Or, Box::new(l), Box::new(r)), start);
        }
        Ok(l)
    }

    fn and(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut l = self.not()?;
        while self.eat_kw("and") {
            let r = self.not()?;
            l = self.mk(
                ExprKind::Binary(BinOp::And, Box::new(l), Box::new(r)),
                start,
            );
        }
        Ok(l)
    }

    fn not(&mut self) -> PResult<Expr> {
        let start = self.start();
        if self.eat_kw("not") {
            self.enter()?;
            let e = self.not()?;
            self.leave();
            return Ok(self.mk(ExprKind::Unary(UnOp::Not, Box::new(e)), start));
        }
        self.satisfies()
    }

    fn satisfies(&mut self) -> PResult<Expr> {
        let start = self.start();
        let l = self.compare()?;
        if self.eat_kw("satisfies") {
            let r = self.compare()?;
            return Ok(self.mk(ExprKind::Satisfies(Box::new(l), Box::new(r)), start));
        }
        Ok(l)
    }

    fn compare(&mut self) -> PResult<Expr> {
        let start = self.start();
        let first = self.as_expr()?;
        let mut rest = Vec::new();
        loop {
            let op = match self.tok() {
                Tok::P(Punct::EqEq) => CmpOp::Eq,
                Tok::P(Punct::Ne) => CmpOp::Ne,
                Tok::P(Punct::Lt) => CmpOp::Lt,
                Tok::P(Punct::Le) => CmpOp::Le,
                Tok::P(Punct::Gt) => CmpOp::Gt,
                Tok::P(Punct::Ge) => CmpOp::Ge,
                _ => break,
            };
            self.bump();
            rest.push((op, self.as_expr()?));
        }
        if rest.is_empty() {
            return Ok(first);
        }
        Ok(self.mk(
            ExprKind::Compare {
                first: Box::new(first),
                rest,
            },
            start,
        ))
    }

    fn as_expr(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut l = self.in_expr()?;
        while self.eat_kw("as") {
            let ty = self.ty()?;
            l = self.mk(ExprKind::As(Box::new(l), ty), start);
        }
        Ok(l)
    }

    fn in_expr(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut l = self.range()?;
        while self.eat_kw("in") {
            let r = self.range()?;
            l = self.mk(ExprKind::Binary(BinOp::In, Box::new(l), Box::new(r)), start);
        }
        Ok(l)
    }

    fn range(&mut self) -> PResult<Expr> {
        let start = self.start();
        let l = self.add()?;
        if self.eat_p(Punct::DotDot) {
            let r = self.add()?;
            return Ok(self.mk(
                ExprKind::Binary(BinOp::Range, Box::new(l), Box::new(r)),
                start,
            ));
        }
        Ok(l)
    }

    fn add(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut l = self.mul()?;
        loop {
            let op = match self.tok() {
                Tok::P(Punct::Plus) => BinOp::Add,
                Tok::P(Punct::Minus) => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let r = self.mul()?;
            l = self.mk(ExprKind::Binary(op, Box::new(l), Box::new(r)), start);
        }
        Ok(l)
    }

    fn mul(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut l = self.pow()?;
        loop {
            let op = match self.tok() {
                Tok::P(Punct::Star) => BinOp::Mul,
                Tok::P(Punct::Slash) => BinOp::Div,
                _ => break,
            };
            self.bump();
            let r = self.pow()?;
            l = self.mk(ExprKind::Binary(op, Box::new(l), Box::new(r)), start);
        }
        Ok(l)
    }

    fn pow(&mut self) -> PResult<Expr> {
        let start = self.start();
        let l = self.unary()?;
        if self.eat_p(Punct::Caret) {
            self.enter()?;
            let r = self.pow()?;
            self.leave();
            return Ok(self.mk(
                ExprKind::Binary(BinOp::Pow, Box::new(l), Box::new(r)),
                start,
            ));
        }
        Ok(l)
    }

    fn unary(&mut self) -> PResult<Expr> {
        let start = self.start();
        if self.eat_p(Punct::Minus) {
            self.enter()?;
            let e = self.unary()?;
            self.leave();
            return Ok(self.mk(ExprKind::Unary(UnOp::Neg, Box::new(e)), start));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut e = self.primary()?;
        loop {
            match self.tok().clone() {
                Tok::P(Punct::Dot) => {
                    self.bump();
                    let name = self.expect_ident()?;
                    e = self.mk(ExprKind::Field(Box::new(e), name), start);
                }
                Tok::P(Punct::LBracket) => {
                    if matches!(self.nth_tok(1), Tok::P(Punct::Plus | Punct::Minus)) {
                        let terms = self.tolerance()?;
                        e = self.mk(ExprKind::Tolerance(Box::new(e), terms), start);
                    } else {
                        self.bump();
                        self.nest += 1;
                        let i = self.expr()?;
                        self.expect_p(Punct::RBracket)?;
                        self.nest -= 1;
                        e = self.mk(ExprKind::Index(Box::new(e), Box::new(i)), start);
                    }
                }
                Tok::P(Punct::LParen) => {
                    let args = self.args()?;
                    e = self.mk(
                        ExprKind::Call {
                            callee: Box::new(e),
                            optional: false,
                            args,
                        },
                        start,
                    );
                }
                Tok::P(Punct::Question) if self.nth_is_p(1, Punct::LParen) => {
                    self.bump();
                    let args = self.args()?;
                    e = self.mk(
                        ExprKind::Call {
                            callee: Box::new(e),
                            optional: true,
                            args,
                        },
                        start,
                    );
                }
                Tok::Ident(name) if is_class_name(&name) => {
                    let class = self.expect_ident()?;
                    e = self.mk(ExprKind::IsoClass(Box::new(e), class), start);
                }
                _ => break,
            }
        }
        Ok(e)
    }

    /// `[+0.018mm -0]` or `[+-0.1mm]`; each term is a signed product-level operand.
    fn tolerance(&mut self) -> PResult<Vec<TolTerm>> {
        self.expect_p(Punct::LBracket)?;
        self.nest += 1;
        let mut terms = Vec::new();
        while !self.is_p(Punct::RBracket) {
            let sign = match self.tok() {
                Tok::P(Punct::Plus) => {
                    let plus_end = self.span().end;
                    self.bump();
                    if self.is_p(Punct::Minus) && self.span().start == plus_end {
                        self.bump();
                        TolSign::PlusMinus
                    } else {
                        TolSign::Plus
                    }
                }
                Tok::P(Punct::Minus) => {
                    self.bump();
                    TolSign::Minus
                }
                other => {
                    let msg = format!("expected `+`, `-` or `+-`, found {}", describe(other));
                    return self.err(msg);
                }
            };
            let value = self.mul()?;
            terms.push(TolTerm { sign, value });
        }
        self.expect_p(Punct::RBracket)?;
        self.nest -= 1;
        Ok(terms)
    }

    fn primary(&mut self) -> PResult<Expr> {
        let start = self.start();
        match self.tok().clone() {
            Tok::Number(n) => {
                self.bump();
                Ok(self.mk(ExprKind::Number(n), start))
            }
            Tok::Quantity(q) => {
                self.bump();
                Ok(self.mk(ExprKind::Quantity(q), start))
            }
            Tok::Str(s) => {
                self.bump();
                Ok(self.mk(ExprKind::Str(s), start))
            }
            Tok::Keyword(k @ ("true" | "false")) => {
                self.bump();
                Ok(self.mk(ExprKind::Bool(k == "true"), start))
            }
            Tok::Keyword("self" | "origin") | Tok::Ident(_) => {
                let p = self.path(false, true)?;
                Ok(self.mk(ExprKind::Path(p), start))
            }
            Tok::P(Punct::LParen) => {
                self.bump();
                self.nest += 1;
                let e = self.expr()?;
                self.expect_p(Punct::RParen)?;
                self.nest -= 1;
                Ok(self.mk(ExprKind::Paren(Box::new(e)), start))
            }
            Tok::P(Punct::LBracket) => {
                self.bump();
                self.nest += 1;
                let mut items = Vec::new();
                while !self.is_p(Punct::RBracket) {
                    items.push(self.expr()?);
                    if !self.eat_p(Punct::Comma) {
                        break;
                    }
                }
                self.expect_p(Punct::RBracket)?;
                self.nest -= 1;
                Ok(self.mk(ExprKind::Array(items), start))
            }
            Tok::P(Punct::LBrace) => {
                let b = self.block()?;
                Ok(self.mk(ExprKind::Block(b), start))
            }
            Tok::Keyword("if") => self.if_expr(),
            Tok::Keyword("match") => self.match_expr(),
            Tok::Keyword(k) => self.err(format!(
                "`{k}` is a reserved keyword and cannot be used as a name"
            )),
            other => self.err(format!(
                "expected an expression, found {}",
                describe(&other)
            )),
        }
    }

    /// `{ let a = ...; expr }`; newlines and `;` separate its statements.
    fn block(&mut self) -> PResult<Block> {
        let start = self.start();
        self.expect_p(Punct::LBrace)?;
        let saved = std::mem::replace(&mut self.nest, 0);
        let mut lets = Vec::new();
        let tail = loop {
            self.skip_blank();
            if self.is_kw("let") {
                lets.push(self.let_stmt()?);
                continue;
            }
            if self.is_p(Punct::RBrace) {
                return self.err("a block needs a result expression before `}`");
            }
            break self.expr()?;
        };
        self.skip_blank();
        self.expect_p(Punct::RBrace)?;
        self.nest = saved;
        Ok(Block {
            lets,
            tail: Box::new(tail),
            span: self.span_from(start),
        })
    }

    fn if_expr(&mut self) -> PResult<Expr> {
        let start = self.start();
        self.expect_kw("if")?;
        self.enter()?;
        let cond = Box::new(self.expr()?);
        let then = self.block()?;
        // `else` may start the next line.
        let mut j = self.pos;
        while matches!(self.toks[j].tok, Tok::Newline) {
            j += 1;
        }
        let otherwise = if matches!(self.toks[j].tok, Tok::Keyword("else")) {
            self.pos = j;
            self.bump();
            if self.is_kw("if") {
                Some(Box::new(self.if_expr()?))
            } else {
                let bstart = self.start();
                let b = self.block()?;
                Some(Box::new(self.mk(ExprKind::Block(b), bstart)))
            }
        } else {
            None
        };
        self.leave();
        Ok(self.mk(
            ExprKind::If {
                cond,
                then,
                otherwise,
            },
            start,
        ))
    }

    fn match_expr(&mut self) -> PResult<Expr> {
        let start = self.start();
        self.expect_kw("match")?;
        let scrutinee = Box::new(self.expr()?);
        self.expect_p(Punct::LBrace)?;
        let saved = std::mem::replace(&mut self.nest, 1);
        let mut arms = Vec::new();
        while !self.is_p(Punct::RBrace) {
            let astart = self.start();
            let pat = if matches!(self.tok(), Tok::Ident(n) if n == "_") {
                Pattern::Wildcard(self.bump().span)
            } else {
                Pattern::Path(self.path(false, false)?)
            };
            self.expect_p(Punct::FatArrow)?;
            let body = self.expr()?;
            arms.push(MatchArm {
                pat,
                body,
                span: self.span_from(astart),
            });
            if !self.eat_p(Punct::Comma) {
                break;
            }
        }
        self.expect_p(Punct::RBrace)?;
        self.nest = saved;
        Ok(self.mk(ExprKind::Match { scrutinee, arms }, start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec_example() -> String {
        let md = include_str!("../../../docs/spec/language.md");
        let body = md.split("```").nth(1).expect("first code block");
        body.trim_start_matches('\n').to_owned()
    }

    fn clean(src: &str) -> Parsed {
        let p = parse(src);
        assert!(p.diags.is_empty(), "unexpected diagnostics: {:?}", p.diags);
        p
    }

    fn components(p: &Parsed, class: ComponentClass) -> Vec<&Component> {
        p.file.components().filter(|c| c.class == class).collect()
    }

    fn only_member(src: &str) -> Member {
        let p = clean(&format!("model M {{\n{src}\n}}\n"));
        let c = p.file.components().next().expect("component");
        assert_eq!(c.members.len(), 1);
        c.members[0].clone()
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn spec_example_parses_with_expected_shape() {
        let p = clean(&spec_example());
        let interfaces = p
            .file
            .items()
            .filter(|i| matches!(i.kind, ItemKind::Interface(_)))
            .count();
        assert_eq!(interfaces, 1);
        assert_eq!(components(&p, ComponentClass::Ots).len(), 2);
        assert_eq!(components(&p, ComponentClass::Model).len(), 3);
        assert_eq!(components(&p, ComponentClass::Assembly).len(), 1);
        let fits = components(&p, ComponentClass::Model)
            .iter()
            .flat_map(|c| &c.members)
            .filter(|m| matches!(m.kind, MemberKind::Fit(_)))
            .count();
        assert_eq!(fits, 1);
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn all_member_productions_accepted() {
        let src = "use std::x\ncite c = \"d.md\"\nmodel A(n: Length = 1mm) {\n  p: Length = 2mm @c\n  q = 3\n  let t = p * 2\n  p <= 5mm\n  p >= 1mm\n  p in 1mm .. 9mm\n  a.b == c.d\n  part w: Wheel\n  fit f = H7/g6 on a.b\n  check ok: p > 0mm \"positive\"\n  require r: p < 9mm @c\n}\n";
        let p = clean(src);
        let c = p.file.components().next().expect("component");
        let kinds: Vec<&str> = c
            .members
            .iter()
            .map(|m| match &m.kind {
                MemberKind::Param(_) => "param",
                MemberKind::Let(_) => "let",
                MemberKind::Bound(_) => "bound",
                MemberKind::Part(_) => "part",
                MemberKind::Fit(_) => "fit",
                MemberKind::Check(_) => "check",
                MemberKind::Require(_) => "require",
                MemberKind::Error => "error",
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "param", "param", "let", "bound", "bound", "bound", "bound", "part", "fit",
                "check", "require"
            ]
        );
        assert_eq!(p.file.uses().count(), 1);
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn statements_end_at_newline_or_semicolon() {
        let c = |src: &str| {
            let p = clean(src);
            p.file.components().next().expect("component").members.len()
        };
        assert_eq!(c("model M {\n a = 1\n b = 2\n}\n"), 2);
        assert_eq!(c("model M { a = 1; b = 2; c = 3 }\n"), 3);
        // Newlines inside ( ) and [ ] do not end a statement.
        assert_eq!(c("model M {\n a = f(1,\n 2,\n 3)\n b = [1,\n 2]\n}\n"), 2);
        assert_eq!(c("model M {\n a = 12mm [+0.1mm\n -0]\n}\n"), 1);
        // Two statements on one line without a separator are an error.
        assert!(!parse("model M {\n a = 1 b = 2\n}\n").diags.is_empty());
        // A newline outside brackets ends the statement, so the bare operator is an error.
        assert!(!parse("model M {\n a = 1 +\n 2\n}\n").diags.is_empty());
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn reserved_keyword_as_identifier_is_error() {
        for src in [
            "model M {\n let = 1\n}\n",
            "model M {\n a = match\n}\n",
            "model in {\n}\n",
            "model M {\n part table: T\n}\n",
        ] {
            let p = parse(src);
            assert!(
                p.diags.iter().any(|d| d.code == DiagCode::E001),
                "expected E001 for {src:?}, got {:?}",
                p.diags
            );
        }
        let p = parse("model M {\n part fit: T\n}\n");
        assert!(
            p.diags
                .iter()
                .any(|d| d.message.contains("reserved keyword"))
        );
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn part_has_one_primary_and_alternates() {
        let m = only_member("part wheel[4]: Caster = Vendor4471B alt [Colson2Series]");
        let MemberKind::Part(part) = m.kind else {
            panic!("not a part")
        };
        assert!(part.count.is_some());
        assert_eq!(part.primary.expect("primary").path.text(), "Vendor4471B");
        assert_eq!(part.alts.len(), 1);
        assert_eq!(part.alts[0].path.text(), "Colson2Series");

        let m = only_member("part w: C = A alt [B, D(x = 1)]");
        let MemberKind::Part(part) = m.kind else {
            panic!("not a part")
        };
        assert_eq!(part.alts.len(), 2);
        assert!(part.alts[1].args.is_some());
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn file_is_uses_and_items_with_no_header() {
        let p = clean("use a::b\nmodel M {\n}\nuse c::d\n");
        assert_eq!(p.file.entries.len(), 3);
        assert!(matches!(p.file.entries[0], FileEntry::Use(_)));
        assert!(matches!(p.file.entries[1], FileEntry::Item(_)));
        // There is no namespace header or `mod` block.
        assert!(!parse("mod x {\n}\n").diags.is_empty());
        assert!(!parse("namespace x\n").diags.is_empty());
        // Empty and comment-only files are fine.
        assert!(clean("// nothing\n").file.entries.is_empty());
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn use_trees_aliases_globs_and_roots() {
        let p = clean(
            "use std::fasteners::{MetricThread, ScrewJoint}\nuse std::fits::iso286 as fits\nuse std::x::*\nuse package::a::b\nuse self::c\nuse super::super::d\npub use acme.rover::E\nuse std::g::{H as I, J::{K, L as M}}\n",
        );
        let uses: Vec<&Use> = p.file.uses().collect();
        assert_eq!(uses.len(), 8);
        let UseKind::Tree(t) = &uses[0].kind else {
            panic!("tree")
        };
        let names: Vec<&str> = t.iter().map(|x| x.name.name.as_str()).collect();
        assert_eq!(names, ["MetricThread", "ScrewJoint"]);
        assert_eq!(uses[0].path.segments.len(), 1);
        let UseKind::Single { alias } = &uses[1].kind else {
            panic!("single")
        };
        assert_eq!(alias.as_ref().expect("alias").name, "fits");
        assert!(matches!(uses[2].kind, UseKind::Glob));
        assert!(matches!(uses[3].path.root, UseRoot::Package));
        assert!(matches!(uses[4].path.root, UseRoot::SelfMod));
        assert!(matches!(uses[5].path.root, UseRoot::Super(2)));
        assert!(matches!(&uses[6].path.root, UseRoot::Pkg(p) if p.len() == 2));
        assert_eq!(uses[6].vis, Vis::Pub);
        let UseKind::Tree(t) = &uses[7].kind else {
            panic!("tree")
        };
        assert!(matches!(&t[0].kind, UseTreeKind::Leaf { alias: Some(a) } if a.name == "I"));
        assert!(matches!(&t[1].kind, UseTreeKind::Group(g) if g.len() == 2));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn errors_recover_and_parsing_continues() {
        let p = parse("model A {\n a = = 1\n b = 2\n}\nmodel B {\n}\n");
        assert_eq!(p.diags.len(), 1);
        assert_eq!(p.file.components().count(), 2);
        let a = p.file.components().next().expect("A");
        assert!(matches!(a.members[0].kind, MemberKind::Error));
        assert!(matches!(a.members[1].kind, MemberKind::Param(_)));
        // Garbage never hangs or panics.
        for src in [
            "}",
            "{{{{",
            "model",
            "model M {",
            "use",
            "a b c",
            "\"unterminated",
            "(((((",
        ] {
            let _ = parse(src);
        }
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn expressions_tolerances_and_classes() {
        let m = only_member("a = 12mm [+0.018mm -0]");
        let MemberKind::Param(p) = m.kind else {
            panic!()
        };
        assert!(matches!(&p.value.kind, ExprKind::Tolerance(_, t) if t.len() == 2));
        let m = only_member("a = 12mm H8");
        let MemberKind::Param(p) = m.kind else {
            panic!()
        };
        assert!(matches!(p.value.kind, ExprKind::IsoClass(..)));
        let m = only_member("a = xs[0]");
        let MemberKind::Param(p) = m.kind else {
            panic!()
        };
        assert!(matches!(p.value.kind, ExprKind::Index(..)));
        let m = only_member("a = 1 + 2 * 3 ^ 2");
        let MemberKind::Param(p) = m.kind else {
            panic!()
        };
        assert!(matches!(p.value.kind, ExprKind::Binary(BinOp::Add, ..)));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn doc_comments_attach_to_items() {
        let p = clean("/// Hello\n/// world\nmodel M {\n}\n");
        let item = p.file.items().next().expect("item");
        assert_eq!(item.docs.len(), 2);
    }
}
