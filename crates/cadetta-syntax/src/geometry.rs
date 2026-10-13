//! Parsers for `datum` and `place` members (geometry.md 2 and 4).
//!
//! A child module of `parser` (declared there with `#[path]`) so it can extend the private
//! `Parser`. Positions in expressions (`Point2 on top`, `p in D`) are parsed by the type and
//! expression grammar; this module owns only the statement forms and the reserved-name rule.

// frob:ticket 01M4SWFX0C2BXQGR8WQ2VN1G88

use super::*;
use crate::ast::{Datum, Place, PlaceRange};

impl Parser<'_> {
    /// Fails with the geometry.md 2 message when the next token is the reserved `origin`.
    pub(super) fn reject_origin_name(&mut self) -> PResult<()> {
        if matches!(self.tok(), Tok::Keyword("origin")) {
            tracing::debug!("rejecting a member named origin");
            return self.err(
                "`origin` is reserved: every component already has the datum `origin` (geometry.md 2)",
            );
        }
        Ok(())
    }

    /// Parses `datum NAME [= expr]`; the value is required unless `interface` is true.
    pub(super) fn datum(&mut self, interface: bool) -> PResult<Datum> {
        let start = self.start();
        self.expect_kw("datum")?;
        self.reject_origin_name()?;
        let name = self.expect_ident()?;
        let value = if self.eat_p(Punct::Eq) {
            if interface {
                return self.err("an interface datum has no value; each implementer defines it");
            }
            Some(self.expr()?)
        } else if interface {
            None
        } else {
            return self.err("expected `=` and the datum's defining expression");
        };
        tracing::debug!(name = %name.name, interface, "parsed datum");
        Ok(Datum {
            name,
            value,
            span: self.span_from(start),
        })
    }

    /// Parses `place TARGET on expr [for IDENT in lo..hi]`.
    pub(super) fn place(&mut self) -> PResult<Place> {
        let start = self.start();
        self.expect_kw("place")?;
        let target = self.postfix()?;
        self.expect_kw("on")?;
        let on = self.expr()?;
        let each = if self.eat_kw("for") {
            let var = self.expect_ident()?;
            self.expect_kw("in")?;
            let lo = self.add()?;
            if !self.is_p(Punct::DotDot) {
                return self.err("a placement `for` needs a range `lo..hi`");
            }
            self.bump();
            let hi = self.add()?;
            Some(PlaceRange { var, lo, hi })
        } else {
            None
        };
        tracing::debug!(ranged = each.is_some(), "parsed place");
        Ok(Place {
            target,
            on,
            each,
            span: self.span_from(start),
        })
    }
}
