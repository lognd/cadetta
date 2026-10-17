//! Conformance tests for the expression grammar (language.md 5, 5a).
//!
//! The grammar itself lives in `parser.rs` (precedence climbing from `or`
//! down to `postfix`); this module pins its behavior, one test per rule.

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::parser::parse;

    /// Parse `src` as a model body and return its single member.
    fn member(src: &str) -> Member {
        let p = parse(&format!("model M {{\n{src}\n}}\n"));
        assert!(p.diags.is_empty(), "diagnostics for {src:?}: {:?}", p.diags);
        let c = p.file.components().next().expect("component");
        assert_eq!(c.members.len(), 1);
        c.members[0].clone()
    }

    /// Parse `a = <src>` and return the value expression.
    fn value(src: &str) -> Expr {
        match member(&format!("a = {src}")).kind {
            MemberKind::Param(p) => p.value,
            other => panic!("not a param: {other:?}"),
        }
    }

    fn bin(e: &Expr) -> (BinOp, &Expr, &Expr) {
        match &e.kind {
            ExprKind::Binary(op, l, r) => (*op, l, r),
            other => panic!("not binary: {other:?}"),
        }
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn multiplication_binds_tighter_than_addition() {
        let e = value("2 * Wheel.hub_width + Frame.span");
        let (op, l, r) = bin(&e);
        assert_eq!(op, BinOp::Add);
        assert!(matches!(bin(l).0, BinOp::Mul));
        assert!(matches!(&r.kind, ExprKind::Field(..)));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn precedence_ladder_postfix_to_comparison() {
        // unary - below postfix, above ^
        let e = value("-a.b");
        let ExprKind::Unary(UnOp::Neg, inner) = &e.kind else {
            panic!("{e:?}")
        };
        assert!(matches!(inner.kind, ExprKind::Field(..)));
        // ^ above * ; * above + ; + above ..
        let e = value("1 + 2 * 3 ^ 4");
        let (op, _, r) = bin(&e);
        assert_eq!(op, BinOp::Add);
        let (op, _, r) = bin(r);
        assert_eq!(op, BinOp::Mul);
        assert_eq!(bin(r).0, BinOp::Pow);
        let e = value("1 + 2 .. 3 * 4");
        let (op, l, r) = bin(&e);
        assert_eq!(op, BinOp::Range);
        assert_eq!(bin(l).0, BinOp::Add);
        assert_eq!(bin(r).0, BinOp::Mul);
        // .. above as
        let e = value("1 .. 2 as Length");
        let ExprKind::As(inner, _) = &e.kind else {
            panic!("{e:?}")
        };
        assert_eq!(bin(inner).0, BinOp::Range);
        // as above comparisons
        let e = value("a as Length < b");
        let ExprKind::Compare { first, .. } = &e.kind else {
            panic!("{e:?}")
        };
        assert!(matches!(first.kind, ExprKind::As(..)));
        // comparison above and above or
        let e = value("a < b and c or d");
        let (op, l, r) = bin(&e);
        assert_eq!(op, BinOp::Or);
        assert_eq!(bin(l).0, BinOp::And);
        assert!(matches!(r.kind, ExprKind::Path(_)));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn tolerance_and_iso_class_attach_to_operand() {
        for src in ["10mm [+-0.05mm]", "8mm [+0 -0.008mm]"] {
            assert!(matches!(value(src).kind, ExprKind::Tolerance(..)), "{src}");
        }
        let ExprKind::Tolerance(_, t) = value("8mm [+0 -0.008mm]").kind else {
            panic!()
        };
        assert_eq!(t.len(), 2);
        for src in ["20mm H7", "x j5"] {
            assert!(matches!(value(src).kind, ExprKind::IsoClass(..)), "{src}");
        }
        // The postfix binds the operand, not the whole sum.
        let e = value("1mm + 20mm H7");
        let (op, _, r) = bin(&e);
        assert_eq!(op, BinOp::Add);
        assert!(matches!(r.kind, ExprKind::IsoClass(..)));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn bracket_is_index_unless_it_starts_with_sign() {
        assert!(matches!(value("a[0]").kind, ExprKind::Index(..)));
        assert!(matches!(value("a [+-0.1mm]").kind, ExprKind::Tolerance(..)));
        assert!(matches!(
            value("a [-0.1mm +0]").kind,
            ExprKind::Tolerance(..)
        ));
    }

    // frob:ticket 01M50C0NEZM1R9KYFTRJCJY1Z8
    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn position_postfix_parses_but_position_index_stays_index() {
        assert!(matches!(
            value("Point2(1mm, 2mm) [position 0.1mm]").kind,
            ExprKind::Position(..)
        ));
        assert!(matches!(
            value("p [position d]").kind,
            ExprKind::Position(..)
        ));
        assert!(matches!(value("holes[position]").kind, ExprKind::Index(..)));
        assert!(matches!(
            value("holes[position + 1]").kind,
            ExprKind::Index(..)
        ));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn chained_comparison_and_satisfies() {
        let e = value("4mm <= x <= 6mm");
        let ExprKind::Compare { rest, .. } = &e.kind else {
            panic!("{e:?}")
        };
        assert_eq!(rest.len(), 2);
        assert!(matches!(rest[0].0, CmpOp::Le) && matches!(rest[1].0, CmpOp::Le));
        assert!(matches!(
            value("x satisfies P").kind,
            ExprKind::Satisfies(..)
        ));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn if_match_and_block_are_expressions() {
        assert!(matches!(
            value("if a { 1mm } else { 2mm }").kind,
            ExprKind::If { .. }
        ));
        let e = value("match k { Kind::A => 1mm, _ => 2mm }");
        let ExprKind::Match { arms, .. } = &e.kind else {
            panic!("{e:?}")
        };
        assert_eq!(arms.len(), 2);
        let e = value("{ let a = 1mm; a }");
        let ExprKind::Block(b) = &e.kind else {
            panic!("{e:?}")
        };
        assert_eq!(b.lets.len(), 1);
        // As an operand too.
        assert!(matches!(
            bin(&value("1mm + { let a = 1mm; a }")).2.kind,
            ExprKind::Block(_)
        ));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn member_and_scope_attributes_attach() {
        let m = member("a = 1mm #[emit(mid)]");
        assert_eq!(m.trailing.len(), 1);
        assert_eq!(m.trailing[0].name.name, "emit");
        assert!(!m.trailing[0].inner);
        let p = parse("model M {\n#![emit(nominal), display(mm)]\na = 1mm\n}\n");
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        let c = p.file.components().next().expect("component");
        assert_eq!(c.inner_attrs.len(), 2);
        assert!(c.inner_attrs.iter().all(|a| a.inner));
        assert_eq!(c.inner_attrs[1].name.name, "display");
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn rename_and_citation_with_locator_recorded() {
        let m = member("a = 1mm @vendor:\"p.2, table 1\" was Globals.plate_thk");
        let MemberKind::Param(p) = m.kind else {
            panic!()
        };
        let c = p.citation.expect("citation");
        // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
        assert!(matches!(&c.name, CiteName::Named(i) if i.text() == "vendor"));
        assert_eq!(c.locator.as_deref(), Some("p.2, table 1"));
        assert_eq!(p.was.len(), 1);
        assert_eq!(p.was[0].segments.len(), 2);
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn calls_and_lookups_parse() {
        let ExprKind::Call { optional, args, .. } = value("sum?(self, unit_cost)").kind else {
            panic!()
        };
        assert!(optional);
        assert_eq!(args.len(), 2);
        let e = value("Iso273[thread].medium");
        let ExprKind::Field(inner, f) = &e.kind else {
            panic!("{e:?}")
        };
        assert_eq!(f.name, "medium");
        assert!(matches!(inner.kind, ExprKind::Index(..)));
        let e = value("T.at_least(x).key");
        let ExprKind::Field(inner, f) = &e.kind else {
            panic!("{e:?}")
        };
        assert_eq!(f.name, "key");
        assert!(matches!(
            inner.kind,
            ExprKind::Call {
                optional: false,
                ..
            }
        ));
    }
}
