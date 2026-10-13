//! Parsers for `use` declarations (roots, aliases, globs, nested trees) and the `pub`
//! / `pub(package)` visibility modifier (packages.md 3, 4; language.md 3).
//!
//! This is a child module of `parser` (declared there with `#[path]`) so it can extend
//! the private `Parser` without widening its visibility.

// frob:ticket 01M4SXRFXB1JYNPNKSAY6WB3QC

use super::*;

impl Parser<'_> {
    /// Parses an optional `pub` or `pub(package)`; absence is [`Vis::Private`].
    pub(super) fn vis(&mut self) -> PResult<Vis> {
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

    pub(super) fn use_decl(&mut self, vis: Vis, start: usize) -> PResult<Use> {
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
        if self.is_p(Punct::Star) {
            self.bump();
            let span = self.span_from(start);
            return Ok(UseTree {
                name: Ident {
                    name: "*".to_owned(),
                    span,
                },
                kind: UseTreeKind::Star,
                span,
            });
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean(src: &str) -> Parsed {
        let p = parse(src);
        assert!(p.diags.is_empty(), "unexpected diagnostics: {:?}", p.diags);
        p
    }

    fn names(path: &UsePath) -> Vec<&str> {
        path.segments.iter().map(|s| s.name.as_str()).collect()
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn use_path_roots_are_recorded() {
        let p = clean(
            "use package::frame::Frame\nuse self::x\nuse super::super::y\nuse acme.shop::policy::AcmeShop\nuse std::fits::iso286\n",
        );
        let u: Vec<&Use> = p.file.uses().collect();
        assert_eq!(u.len(), 5);
        assert!(matches!(u[0].path.root, UseRoot::Package));
        assert_eq!(names(&u[0].path), ["frame", "Frame"]);
        assert!(matches!(u[1].path.root, UseRoot::SelfMod));
        assert_eq!(names(&u[1].path), ["x"]);
        assert!(matches!(u[2].path.root, UseRoot::Super(2)));
        assert_eq!(names(&u[2].path), ["y"]);
        let UseRoot::Pkg(parts) = &u[3].path.root else {
            panic!("dotted dependency")
        };
        let parts: Vec<&str> = parts.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(parts, ["acme", "shop"]);
        assert_eq!(names(&u[3].path), ["policy", "AcmeShop"]);
        let UseRoot::Pkg(parts) = &u[4].path.root else {
            panic!("std")
        };
        assert_eq!(parts[0].name, "std");
        let p = clean("use super::z\n");
        assert!(matches!(
            p.file.uses().next().expect("use").path.root,
            UseRoot::Super(1)
        ));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn aliases_globs_and_nested_trees_are_recorded() {
        let p = clean("use a::b as c\nuse a::b::*\nuse a::{A, B as C, sub::{D, *}}\n");
        let u: Vec<&Use> = p.file.uses().collect();
        let UseKind::Single { alias } = &u[0].kind else {
            panic!("single")
        };
        assert_eq!(alias.as_ref().expect("alias").name, "c");
        assert!(matches!(u[1].kind, UseKind::Glob));
        assert_eq!(names(&u[1].path), ["b"]);
        let UseKind::Tree(t) = &u[2].kind else {
            panic!("tree")
        };
        assert_eq!(t.len(), 3);
        assert!(matches!(&t[0].kind, UseTreeKind::Leaf { alias: None }));
        assert!(matches!(&t[1].kind, UseTreeKind::Leaf { alias: Some(a) } if a.name == "C"));
        let UseTreeKind::Group(g) = &t[2].kind else {
            panic!("group")
        };
        assert_eq!(t[2].name.name, "sub");
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].name.name, "D");
        assert!(matches!(g[1].kind, UseTreeKind::Star));
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn visibility_is_recorded_on_items_uses_and_members() {
        let p = clean(
            "pub use a::b\npub(package) use a::c\nuse a::d\npub model M {\n  priv row = Iso4762Head[thread]\n  x = 1mm\n}\npub(package) model N {\n}\nmodel O {\n}\n",
        );
        let u: Vec<&Use> = p.file.uses().collect();
        assert_eq!(u[0].vis, Vis::Pub);
        assert_eq!(u[1].vis, Vis::PubPackage);
        assert_eq!(u[2].vis, Vis::Private);
        let vis: Vec<Vis> = p.file.items().map(|i| i.vis).collect();
        assert_eq!(vis, [Vis::Pub, Vis::PubPackage, Vis::Private]);
        let m = p.file.components().next().expect("M");
        assert!(m.members[0].private);
        assert!(!m.members[1].private);
    }

    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    #[test]
    fn no_module_or_namespace_header_is_expected() {
        clean("use a::b\nmodel M {\n}\n");
        assert!(!parse("namespace x\n").diags.is_empty());
        assert!(!parse("namespace x.y\nmodel M {\n}\n").diags.is_empty());
    }
}
