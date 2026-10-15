//! Parsers for the remaining item kinds: `process`, `table`, `enum`, `fn`, `solver`,
//! `waive`, `unit` and `kind`, plus kind expressions (language.md 3.6, 3.8, 3.10, 5a).
//!
//! This is a child module of `parser` (declared there with `#[path]`) so it can extend
//! the private `Parser` without widening its visibility.

// frob:ticket 01M4ST7A8ZG0BS11X26BB84S84

use super::*;
use crate::ast::{
    Bind, EnumDecl, FnDecl, KindDecl, KindExpr, KindOp, KindTerm, Process, ProcessMember,
    ProcessMemberKind, RawText, Rule, Solver, SolverMember, SolverMemberKind, Table, TableRow,
    UnitDecl, Waive, WaiveAlt,
};

/// One statement of a table body, before the parts are assembled into a `Table`.
enum TableEntry {
    Key(Box<ParamDecl>),
    Cols(Vec<ParamDecl>),
    Rows(Vec<TableRow>),
    Error,
}

impl Parser<'_> {
    /// True at a statement boundary: newline, `;`, `}` or end of file.
    fn at_stmt_end(&self) -> bool {
        matches!(
            self.tok(),
            Tok::Newline | Tok::P(Punct::Semi | Punct::RBrace) | Tok::Eof
        )
    }

    // ---- kind expressions ----

    /// The rest of a kind expression whose first kind name `base` is already read.
    pub(super) fn kind_expr_tail(&mut self, base: Path) -> PResult<KindExpr> {
        let mut terms = Vec::new();
        let mut name = base;
        let mut op = KindOp::Mul;
        loop {
            let start = name.span.start;
            let exp = if self.eat_p(Punct::Caret) {
                Some(self.kind_exponent()?)
            } else {
                None
            };
            terms.push(KindTerm {
                op,
                name,
                exp,
                span: self.span_from(start),
            });
            op = if self.eat_p(Punct::Star) {
                KindOp::Mul
            } else if self.eat_p(Punct::Slash) {
                KindOp::Div
            } else {
                break;
            };
            name = self.path(false, false)?;
        }
        tracing::trace!(terms = terms.len(), "parsed kind expression");
        Ok(KindExpr { terms })
    }

    /// An integer exponent, optionally negative.
    fn kind_exponent(&mut self) -> PResult<i32> {
        let neg = self.eat_p(Punct::Minus);
        if let Tok::Number(n) = self.tok().clone()
            && let Ok(v) = n.parse::<i32>()
        {
            self.bump();
            return Ok(if neg { -v } else { v });
        }
        self.err("a kind exponent must be an integer")
    }

    // ---- process ----

    /// `process NAME [citation] { { param | rule } }`.
    pub(super) fn process(&mut self) -> PResult<Process> {
        self.expect_kw("process")?;
        let name = self.expect_ident()?;
        let citation = if self.is_p(Punct::At) {
            Some(self.citation()?)
        } else {
            None
        };
        self.expect_p(Punct::LBrace)?;
        tracing::debug!(name = %name.name, "parsing process");
        let members = self.body(Self::process_member, |span| ProcessMember {
            docs: Vec::new(),
            attrs: Vec::new(),
            kind: ProcessMemberKind::Error,
            span,
        });
        Ok(Process {
            name,
            citation,
            members,
        })
    }

    fn process_member(&mut self) -> PResult<ProcessMember> {
        let docs = self.take_docs(self.start());
        let attrs = self.leading_attrs()?;
        let start = self.start();
        let kind = if self.is_kw("rule") {
            ProcessMemberKind::Rule(Box::new(self.rule()?))
        } else if matches!(self.tok(), Tok::Ident(_)) {
            ProcessMemberKind::Param(Box::new(self.param()?))
        } else {
            return self.err(format!(
                "expected a process parameter or `rule`, found {}",
                describe(&self.tok().clone())
            ));
        };
        let span = self.span_from(start);
        self.end_stmt()?;
        Ok(ProcessMember {
            docs,
            attrs,
            kind,
            span,
        })
    }

    /// `rule NAME ( pdecl ) -> type = expr`.
    fn rule(&mut self) -> PResult<Rule> {
        self.expect_kw("rule")?;
        let name = self.expect_ident()?;
        self.expect_p(Punct::LParen)?;
        self.nest += 1;
        let param = self.pdecl()?;
        self.expect_p(Punct::RParen)?;
        self.nest -= 1;
        self.expect_p(Punct::Arrow)?;
        let ret = self.ty()?;
        self.expect_p(Punct::Eq)?;
        let body = self.expr()?;
        Ok(Rule {
            name,
            param,
            ret,
            body,
        })
    }

    // ---- table ----

    /// `table NAME [citation] { key ... [cols ...] rows }`.
    pub(super) fn table(&mut self) -> PResult<Table> {
        self.expect_kw("table")?;
        let name = self.expect_ident()?;
        let citation = if self.is_p(Punct::At) {
            Some(self.citation()?)
        } else {
            None
        };
        let open = self.expect_p(Punct::LBrace)?;
        tracing::debug!(name = %name.name, "parsing table");
        let entries = self.body(Self::table_entry, |_| vec![TableEntry::Error]);
        let mut table = Table {
            name,
            citation,
            key: None,
            cols: Vec::new(),
            rows: Vec::new(),
        };
        let mut errored = false;
        for entry in entries.into_iter().flatten() {
            match entry {
                TableEntry::Key(k) if table.key.is_none() => table.key = Some(*k),
                TableEntry::Key(k) => self.diag(k.span, "a table has one `key`"),
                TableEntry::Cols(c) if table.cols.is_empty() => table.cols = c,
                TableEntry::Cols(c) => self.diag(
                    c.first().map_or(open, |p| p.span),
                    "a table has one `cols` line",
                ),
                TableEntry::Rows(r) => table.rows.extend(r),
                TableEntry::Error => errored = true,
            }
        }
        if table.key.is_none() && !errored {
            self.diag(table.name.span, "a table needs a `key` declaration");
        }
        Ok(table)
    }

    /// Records an E001 diagnostic without abandoning the item.
    fn diag(&mut self, span: Span, message: &str) {
        self.diags.push(Diagnostic {
            code: DiagCode::E001,
            span,
            message: message.to_owned(),
            fix: None,
        });
    }

    fn table_entry(&mut self) -> PResult<Vec<TableEntry>> {
        if self.eat_kw("key") {
            let k = self.pdecl()?;
            self.table_sep()?;
            return Ok(vec![TableEntry::Key(Box::new(k))]);
        }
        if self.eat_kw("cols") {
            let mut cols = vec![self.pdecl()?];
            while self.eat_p(Punct::Comma) {
                cols.push(self.pdecl()?);
            }
            self.table_sep()?;
            return Ok(vec![TableEntry::Cols(cols)]);
        }
        // Key-only rows may share a line (`3mm  4mm  5mm`); a row with `=>` ends its line.
        let mut rows = Vec::new();
        loop {
            let row = self.table_row()?;
            let keyed = row.values.is_some();
            rows.push(row);
            if keyed || self.at_stmt_end() {
                break;
            }
        }
        self.end_stmt()?;
        Ok(vec![TableEntry::Rows(rows)])
    }

    /// Ends a `key` or `cols` line; the first row may follow on the same line.
    fn table_sep(&mut self) -> PResult<()> {
        if self.at_stmt_end() || matches!(self.tok(), Tok::Number(_) | Tok::Quantity(_)) {
            if matches!(self.tok(), Tok::Newline | Tok::P(Punct::Semi)) {
                self.bump();
            }
            return Ok(());
        }
        self.end_stmt()
    }

    fn table_row(&mut self) -> PResult<TableRow> {
        let docs = self.take_docs(self.start());
        let start = self.start();
        let key = self.expr()?;
        let values = if self.eat_p(Punct::FatArrow) {
            let mut vals = vec![self.expr()?];
            while self.eat_p(Punct::Comma) {
                vals.push(self.expr()?);
            }
            Some(vals)
        } else {
            None
        };
        Ok(TableRow {
            docs,
            key,
            values,
            span: self.span_from(start),
        })
    }

    // ---- enum, fn ----

    /// `enum NAME { A, B [,] }`; newlines may separate variants.
    pub(super) fn enum_decl(&mut self) -> PResult<EnumDecl> {
        self.expect_kw("enum")?;
        let name = self.expect_ident()?;
        self.expect_p(Punct::LBrace)?;
        let saved = std::mem::replace(&mut self.nest, 1);
        let mut variants = Vec::new();
        while !self.is_p(Punct::RBrace) {
            variants.push(self.expect_ident()?);
            if !self.eat_p(Punct::Comma) {
                break;
            }
        }
        self.expect_p(Punct::RBrace)?;
        self.nest = saved;
        if variants.is_empty() {
            return Err(ParseErr {
                span: name.span,
                message: "an enum needs at least one variant".to_owned(),
            });
        }
        tracing::debug!(name = %name.name, variants = variants.len(), "parsed enum");
        Ok(EnumDecl { name, variants })
    }

    /// `fn NAME ( [pdecl {, pdecl}] ) -> type { { let } expr }`.
    pub(super) fn fn_decl(&mut self) -> PResult<FnDecl> {
        self.expect_kw("fn")?;
        let name = self.expect_ident()?;
        self.expect_p(Punct::LParen)?;
        self.nest += 1;
        let mut params = Vec::new();
        while !self.is_p(Punct::RParen) {
            params.push(self.pdecl()?);
            if !self.eat_p(Punct::Comma) {
                break;
            }
        }
        self.expect_p(Punct::RParen)?;
        self.nest -= 1;
        self.expect_p(Punct::Arrow)?;
        let ret = self.ty()?;
        let body = self.block()?;
        tracing::debug!(name = %name.name, params = params.len(), "parsed fn");
        Ok(FnDecl {
            name,
            params,
            ret,
            body,
        })
    }

    // ---- solver ----

    // frob:ticket 01M4VB630NZKXA6Q9D7253FATR
    /// `solver NAME [citation] { script STRING { in binds | out NAME: kind | deps STRING } }`.
    pub(super) fn solver(&mut self) -> PResult<Solver> {
        self.expect_kw("solver")?;
        let name = self.expect_ident()?;
        let citation = if self.is_p(Punct::At) {
            Some(self.citation()?)
        } else {
            None
        };
        self.expect_p(Punct::LBrace)?;
        tracing::debug!(name = %name.name, cited = citation.is_some(), "parsing solver");
        self.skip_blank();
        let start = self.start();
        let (script, script_span) = if matches!(self.tok(), Tok::Ident(s) if s == "script") {
            self.bump();
            let script = self.expect_str()?;
            let script_span = self.span_from(start);
            self.end_stmt()?;
            (script, script_span)
        } else {
            let hint = match &citation {
                Some(Citation {
                    name: CiteName::Literal(t),
                    ..
                }) => format!(
                    "; the draft 1.4 form `solver NAME @\"{t}\"` is gone, write `script \"{t}\"` in the body"
                ),
                _ => String::new(),
            };
            let message = format!(
                "a solver body must start with `script \"path\"`, found {}{hint}",
                describe(&self.tok().clone())
            );
            tracing::debug!(name = %name.name, "solver without a script statement");
            // Report and keep parsing the body so one error is one diagnostic.
            self.diags.push(Diagnostic {
                code: DiagCode::E001,
                span: self.span(),
                message,
                fix: None,
            });
            (String::new(), self.span())
        };
        let members = self.body(Self::solver_member, |span| SolverMember {
            kind: SolverMemberKind::Error,
            span,
        });
        Ok(Solver {
            name,
            citation,
            script,
            script_span,
            members,
        })
    }

    fn solver_member(&mut self) -> PResult<SolverMember> {
        let _ = self.take_docs(self.start());
        let start = self.start();
        let kind = if self.eat_kw("in") {
            let mut binds = vec![self.bind()?];
            while self.eat_p(Punct::Comma) {
                // A comma at the end of a line continues the bindings on the next one.
                self.skip_blank();
                binds.push(self.bind()?);
            }
            SolverMemberKind::In(binds)
        } else if self.eat_kw("out") {
            let name = self.expect_ident()?;
            // A missing kind parses so the checker can raise S007 (solvers.md 1).
            let ty = if self.eat_p(Punct::Colon) {
                Some(self.ty()?)
            } else {
                None
            };
            SolverMemberKind::Out { name, ty }
        } else if self.eat_kw("deps") {
            SolverMemberKind::Deps(self.expect_str()?)
        } else if self.is_kw("check") {
            SolverMemberKind::Check(Box::new(self.check()?))
        } else {
            return self.err(format!(
                "expected `in`, `out`, `deps` or `check`, found {}",
                describe(&self.tok().clone())
            ));
        };
        let span = self.span_from(start);
        self.end_stmt()?;
        Ok(SolverMember { kind, span })
    }

    fn bind(&mut self) -> PResult<Bind> {
        let start = self.start();
        let name = self.expect_ident()?;
        self.expect_p(Punct::Eq)?;
        let value = self.expr()?;
        Ok(Bind {
            name,
            value,
            span: self.span_from(start),
        })
    }

    // ---- waive ----

    /// `waive CODE on GLOB [for alt SLOT=ALT] { because STRING [expires DATE] }`.
    pub(super) fn waive(&mut self) -> PResult<Waive> {
        self.expect_kw("waive")?;
        let code = self.expect_ident()?;
        self.expect_kw("on")?;
        let glob = self.joined_text("a path glob")?;
        let for_alt = if self.eat_kw("for") {
            self.expect_kw("alt")?;
            let slot = self.joined_text("a slot path")?;
            self.expect_p(Punct::Eq)?;
            let alt = self.joined_text("an alternate")?;
            Some(WaiveAlt { slot, alt })
        } else {
            None
        };
        self.expect_p(Punct::LBrace)?;
        self.skip_blank();
        self.expect_kw("because")?;
        let because = self.expect_str()?;
        self.skip_blank();
        let expires = if self.eat_kw("expires") {
            // frob:ticket 01M4YR011TKARTCKMXB4P8A6ST
            // A bad date is reported here and parsing continues to the closing
            // brace, so one malformed token yields one diagnostic (no cascade).
            match self.date() {
                Ok(d) => Some(d),
                Err(e) => {
                    tracing::debug!(span = %e.span, "bad waive date, reported in place");
                    self.diags.push(Diagnostic {
                        code: DiagCode::E001,
                        span: e.span,
                        message: e.message,
                        fix: None,
                    });
                    None
                }
            }
        } else {
            None
        };
        self.skip_blank();
        self.expect_p(Punct::RBrace)?;
        tracing::debug!(code = %code.name, glob = %glob.text, "parsed waiver");
        Ok(Waive {
            code,
            glob,
            for_alt,
            because,
            expires,
        })
    }

    /// Source text for a token that may appear in a glob, slot or date.
    fn raw_piece(t: &Tok) -> Option<String> {
        match t {
            Tok::Ident(s) | Tok::Number(s) => Some(s.clone()),
            Tok::Keyword(k) => Some((*k).to_owned()),
            Tok::P(
                p @ (Punct::ColonColon
                | Punct::Dot
                | Punct::DotDot
                | Punct::Star
                | Punct::Slash
                | Punct::LBracket
                | Punct::RBracket
                | Punct::Minus),
            ) => Some(p.text().to_owned()),
            _ => None,
        }
    }

    /// Joins the run of tokens with no whitespace between them into one text.
    fn joined_text(&mut self, what: &str) -> PResult<RawText> {
        let start = self.start();
        let mut text = String::new();
        let mut end = start;
        while let Some(piece) = Self::raw_piece(self.tok())
            && (text.is_empty() || self.span().start == end)
        {
            text.push_str(&piece);
            end = self.bump().span.end;
        }
        if text.is_empty() {
            return self.err(format!("expected {what}, found {}", describe(self.tok())));
        }
        Ok(RawText {
            text,
            span: Span::new(start, end),
        })
    }

    /// A date `YYYY-MM-DD` written bare.
    fn date(&mut self) -> PResult<RawText> {
        let raw = self.joined_text("a date")?;
        let ok = {
            let parts: Vec<&str> = raw.text.split('-').collect();
            matches!(parts.as_slice(), [y, m, d]
                if y.len() == 4 && m.len() == 2 && d.len() == 2
                    && parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit())))
        };
        if !ok {
            return Err(ParseErr {
                span: raw.span,
                message: format!("expected a date such as 2027-06-30, found `{}`", raw.text),
            });
        }
        Ok(raw)
    }

    // ---- unit, kind ----

    /// `unit NAME = expr` or `unit KIND UNIT`.
    pub(super) fn unit_decl(&mut self) -> PResult<UnitDecl> {
        self.expect_kw("unit")?;
        let first = self.expect_ident()?;
        if self.eat_p(Punct::Eq) {
            let value = self.expr()?;
            tracing::debug!(name = %first.name, "parsed unit definition");
            return Ok(UnitDecl::Define { name: first, value });
        }
        let unit = self.expect_ident()?;
        tracing::debug!(kind = %first.name, unit = %unit.name, "parsed default unit");
        Ok(UnitDecl::Default { kind: first, unit })
    }

    /// `kind NAME = kindexpr`.
    pub(super) fn kind_decl(&mut self) -> PResult<KindDecl> {
        self.expect_kw("kind")?;
        let name = self.expect_ident()?;
        self.expect_p(Punct::Eq)?;
        let def = self.ty()?;
        Ok(KindDecl { name, def })
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::parser::{Parsed, parse};

    fn clean(src: &str) -> Parsed {
        let p = parse(src);
        assert!(p.diags.is_empty(), "diagnostics: {:?}", p.diags);
        p
    }

    fn only_item(src: &str) -> Item {
        let p = clean(src);
        let mut items = p.file.items();
        let item = items.next().expect("an item").clone();
        assert!(items.next().is_none(), "more than one item");
        item
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn interface_members_and_bounds() {
        let item = only_item("interface Caster {\n  diameter: Length\n  load_rating >= 400N\n}\n");
        let ItemKind::Interface(i) = item.kind else {
            panic!("{item:?}")
        };
        assert_eq!(i.members.len(), 2);
        assert!(matches!(
            i.members[0].kind,
            InterfaceMemberKind::Field {
                optional: false,
                ..
            }
        ));
        let InterfaceMemberKind::Bound(b) = &i.members[1].kind else {
            panic!()
        };
        assert!(matches!(b.kind, BoundKind::Ge { .. }));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn component_implements_interface_lists_and_citation() {
        let p = clean(
            "ots Vendor4471B: Caster @vendor {\n  diameter = 50mm\n}\nmodel M: A, B::C { }\n",
        );
        let cs: Vec<_> = p.file.components().collect();
        assert_eq!(cs[0].implements.len(), 1);
        assert_eq!(cs[0].implements[0].text(), "Caster");
        assert!(cs[0].citation.is_some());
        assert_eq!(cs[1].implements.len(), 2);
        assert_eq!(cs[1].implements[1].text(), "B::C");
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn component_parameter_defaults_are_recorded() {
        let p = clean("model Axle(wheel: Caster, span: Length = 1mm) { }\n");
        let c = p.file.components().next().expect("component");
        assert_eq!(c.params.len(), 2);
        assert!(c.params[0].default.is_none());
        assert!(c.params[1].default.is_some());
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn table_rows_and_key_only_rows() {
        let item = only_item(
            "table Iso273 @iso273 {\n  key thread: MetricThread\n  cols fine: Length, medium: Length\n  M5 => 5.3mm, 5.5mm\n  M6 => 6.4mm, 6.6mm\n}\n",
        );
        let ItemKind::Table(t) = item.kind else {
            panic!()
        };
        assert!(t.citation.is_some());
        assert_eq!(t.key.expect("key").name.name, "thread");
        assert_eq!(t.cols.len(), 2);
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0].values.as_ref().map(Vec::len), Some(2));

        let item = only_item("table Len { key length: Length  3mm  4mm  5mm }\n");
        let ItemKind::Table(t) = item.kind else {
            panic!()
        };
        assert!(t.cols.is_empty());
        assert_eq!(t.rows.len(), 3);
        assert!(t.rows.iter().all(|r| r.values.is_none()));

        let item = only_item("table Len {\n  key length: Length\n  3mm\n  4mm => 1mm\n}\n");
        let ItemKind::Table(t) = item.kind else {
            panic!()
        };
        assert!(t.rows[0].values.is_none() && t.rows[1].values.is_some());
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn table_without_key_is_an_error() {
        let p = parse("table T {\n  3mm\n}\n");
        assert_eq!(p.diags.len(), 1, "{:?}", p.diags);
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn process_params_and_rules() {
        let item = only_item(
            "process Fdm @measured:\"coupon 2026-10-01\" {\n  hole_comp = 0.2mm [+-0.05mm]\n  rule hole(d: Length) -> Length = d + hole_comp\n  rule boss(d: Length) -> Length = d - 0.1mm\n}\n",
        );
        let ItemKind::Process(p) = item.kind else {
            panic!()
        };
        assert!(p.citation.expect("citation").locator.is_some());
        assert_eq!(p.members.len(), 3);
        assert!(matches!(p.members[0].kind, ProcessMemberKind::Param(_)));
        let ProcessMemberKind::Rule(r) = &p.members[1].kind else {
            panic!()
        };
        assert_eq!(r.name.name, "hole");
        assert_eq!(r.param.name.name, "d");
        assert!(matches!(r.body.kind, ExprKind::Binary(BinOp::Add, ..)));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn solver_item_and_member() {
        let src = "solver wall {\n  script \"analysis/w.py\"\n  in load = Frame.max_load, span = Frame.span\n  out min_wall: Length\n  out q: Volume / Time\n  deps \"db.csv\"\n}\n";
        let ItemKind::Solver(s) = only_item(src).kind else {
            panic!()
        };
        assert_eq!(s.members.len(), 4);
        let SolverMemberKind::In(binds) = &s.members[0].kind else {
            panic!()
        };
        assert_eq!(binds.len(), 2);
        let SolverMemberKind::Out { ty, .. } = &s.members[2].kind else {
            panic!()
        };
        assert!(matches!(
            ty.as_ref().map(|t| &t.kind),
            Some(TypeKind::KindExpr(_))
        ));
        assert!(matches!(s.members[3].kind, SolverMemberKind::Deps(_)));

        let p = clean(&format!(
            "assembly A {{\n  {}\n}}\n",
            src.replace('\n', "\n  ")
        ));
        let c = p.file.components().next().expect("component");
        assert!(matches!(c.members[0].kind, MemberKind::Solver(_)));
        assert_eq!(s.script, "analysis/w.py");
        assert!(s.citation.is_none());
        let p = parse("solver s {\n}\n");
        assert_eq!(p.diags.len(), 1);
    }

    // frob:ticket 01M4VB630NZKXA6Q9D7253FATR
    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn solver_cited_script() {
        let src = "solver w @roark {\n  script \"a.py\"\n  out x: Length\n}\n";
        let ItemKind::Solver(s) = only_item(src).kind else {
            panic!()
        };
        assert_eq!(s.script, "a.py");
        assert!(s.citation.is_some());
    }

    // frob:ticket 01M4VB630NZKXA6Q9D7253FATR
    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn solver_without_script_or_draft_1_4_form_is_e001() {
        for src in [
            "solver s {\n  out x: Length\n}\n",
            "solver s @\"a.py\" {\n  out x: Length\n}\n",
        ] {
            let p = parse(src);
            assert_eq!(p.diags.len(), 1, "{:?}", p.diags);
            assert_eq!(p.diags[0].code, crate::lexer::DiagCode::E001);
            assert!(p.diags[0].message.contains("script"), "{:?}", p.diags);
        }
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn waive_forms() {
        let src = "waive C002 on acme.actuator::Actuator.journal {\n  because \"accepted\"\n  expires 2027-06-30\n}\n";
        let ItemKind::Waive(w) = only_item(src).kind else {
            panic!()
        };
        assert_eq!(w.code.name, "C002");
        assert_eq!(w.glob.text, "acme.actuator::Actuator.journal");
        assert_eq!(w.expires.expect("expires").text, "2027-06-30");
        assert!(w.for_alt.is_none());

        let src = "waive D001 on acme.parts::Bearing608.* for alt wheel=Colson2Series { because \"same\" }\n";
        let ItemKind::Waive(w) = only_item(src).kind else {
            panic!()
        };
        assert_eq!(w.glob.text, "acme.parts::Bearing608.*");
        let alt = w.for_alt.expect("for alt");
        assert_eq!(
            (alt.slot.text.as_str(), alt.alt.text.as_str()),
            ("wheel", "Colson2Series")
        );
        assert!(w.expires.is_none());

        let src = "waive C001 on a::**.x[*] for alt Rover/wheel = m::Colson { because \"r\" }\n";
        let ItemKind::Waive(w) = only_item(src).kind else {
            panic!()
        };
        assert_eq!(w.glob.text, "a::**.x[*]");
        assert_eq!(w.for_alt.expect("alt").slot.text, "Rover/wheel");

        let bad = parse("waive C001 on a {\n because \"r\"\n expires 2027-6-3\n}\n");
        assert_eq!(bad.diags.len(), 1, "{:?}", bad.diags);
        assert_eq!(bad.diags[0].code, crate::lexer::DiagCode::E001);
        assert!(!parse("waive C001 on a { }\n").diags.is_empty());
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn declarations_enum_fn_unit_kind_cite() {
        let p = clean(
            // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
            "cite c: document {\n  file = \"d.md\"\n}\ncite s: standard { title = \"T\"; number = \"1\" }\nenum Material { Al6061T6, Petg }\nenum E {\n  A,\n  B,\n}\nfn f(a: Length, b: Ratio = 2) -> Length {\n  let x = a * b\n  x + 1mm\n}\nunit kip = 1000lbf\nunit Stress MPa\nkind Stress = Pressure\nkind HeatFlux = Power / Area\nkind Imp = Pressure / VolumeFlow^2\nfit f1 = H7/p6 on a.b\nrequire r: a > 0mm @c\ncheck k: a > 0mm\n",
        );
        let kinds: Vec<&str> = p
            .file
            .items()
            .map(|i| match &i.kind {
                ItemKind::Cite(_) => "cite",
                ItemKind::Enum(_) => "enum",
                ItemKind::Fn(_) => "fn",
                ItemKind::Unit(_) => "unit",
                ItemKind::Kind(_) => "kind",
                ItemKind::Fit(_) => "fit",
                ItemKind::Require(_) => "require",
                ItemKind::Check(_) => "check",
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "cite", "cite", "enum", "enum", "fn", "unit", "unit", "kind", "kind", "kind",
                "fit", "require", "check"
            ]
        );
        let items: Vec<_> = p.file.items().collect();
        let ItemKind::Cite(c0) = &items[1].kind else {
            panic!()
        };
        assert_eq!((c0.ty.name.as_str(), c0.fields.len()), ("standard", 2));
        let ItemKind::Enum(e) = &items[3].kind else {
            panic!()
        };
        assert_eq!(e.variants.len(), 2);
        let ItemKind::Fn(f) = &items[4].kind else {
            panic!()
        };
        assert_eq!((f.params.len(), f.body.lets.len()), (2, 1));
        assert!(matches!(
            &items[5].kind,
            ItemKind::Unit(UnitDecl::Define { .. })
        ));
        assert!(matches!(
            &items[6].kind,
            ItemKind::Unit(UnitDecl::Default { .. })
        ));
        let ItemKind::Kind(k) = &items[9].kind else {
            panic!()
        };
        let TypeKind::KindExpr(ke) = &k.def.kind else {
            panic!()
        };
        assert_eq!(ke.terms.len(), 2);
        assert_eq!(ke.terms[1].op, KindOp::Div);
        assert_eq!(ke.terms[1].exp, Some(2));
        let ItemKind::Kind(k) = &items[7].kind else {
            panic!()
        };
        assert!(matches!(k.def.kind, TypeKind::Path(_)));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn kind_expressions_accepted_wherever_a_kind_is() {
        clean(
            "model M(q: Volume / Time) {\n  r: Length * Length = 1mm * 1mm\n  p: Length / Time = 1mm / 1s\n}\n",
        );
        let p = clean("fn g(x: Mass * Length^-1) -> Mass / Length^2 {\n  x\n}\n");
        let ItemKind::Fn(f) = &p.file.items().next().expect("item").kind else {
            panic!()
        };
        let TypeKind::KindExpr(ke) = &f.params[0].ty.kind else {
            panic!()
        };
        assert_eq!(ke.terms[1].exp, Some(-1));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn item_errors_recover() {
        let p = parse("enum E { }\nkind K = \nenum Ok { A }\n");
        assert_eq!(p.diags.len(), 2, "{:?}", p.diags);
        assert!(p.file.items().any(|i| matches!(i.kind, ItemKind::Enum(_))));
    }
}
