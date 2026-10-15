//! Conformance tests for `use` resolution: order, globs, re-exports, prelude (packages.md 4).
//!
//! The behaviour lives in `resolve.rs` (`Resolver::scope_lookup` and friends);
//! this module pins it to the acceptance criteria of the use-resolution story
//! and records the decisions written into packages.md 4: a `pub use` is an
//! alias whose canonical path is the definition's, a glob imports only
//! non-private re-exports and `pub` items, and `std` contributes nothing to
//! the prelude beyond the names the caller registers with `add_prelude`.

// frob:ticket 01M4SXRV606ZNADV511Z42YR0Q

#[cfg(test)]
mod tests {
    use crate::resolve::{Ctx, EnumDecl, ModuleSrc, PackageSrc, Resolved, Resolver};
    use cadetta_syntax::ast::{Expr, File, MemberKind, Path};
    use cadetta_syntax::parser::parse;

    fn file(src: &str) -> File {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        p.file
    }

    fn expr_of(src: &str) -> Expr {
        let f = file(&format!("model T {{\n  v = {src}\n}}"));
        match &f.components().next().unwrap().members[0].kind {
            MemberKind::Param(p) => p.value.clone(),
            _ => unreachable!(),
        }
    }

    fn path_of(src: &str) -> Path {
        match &file(&format!("model T {{\n  v = 1 was {src}\n}}"))
            .components()
            .next()
            .unwrap()
            .members[0]
            .kind
        {
            MemberKind::Param(p) => p.was[0].clone(),
            _ => unreachable!(),
        }
    }

    fn ctx(pkg: &str, module: &[&str], comp: &str) -> Ctx {
        Ctx {
            package: pkg.to_owned(),
            module: module.iter().map(|s| (*s).to_owned()).collect(),
            component: Some(comp.to_owned()),
            locals: Vec::new(),
        }
    }

    fn pkg<'a>(name: &str, deps: &[&str], mods: Vec<(&[&str], &'a File)>) -> PackageSrc<'a> {
        let no_enums: &'a [EnumDecl] = &[];
        PackageSrc {
            defaults: Default::default(),
            name: name.to_owned(),
            deps: deps.iter().map(|s| (*s).to_owned()).collect(),
            modules: mods
                .into_iter()
                .map(|(p, file)| ModuleSrc {
                    path: p.iter().map(|s| (*s).to_owned()).collect(),
                    file,
                    enums: no_enums,
                })
                .collect(),
        }
    }

    fn item_path(r: &Resolved) -> String {
        match r {
            Resolved::Member { path, .. } | Resolved::Item { path, .. } => path.to_string(),
            other => format!("{other:?}"),
        }
    }

    // frob:tests crates/cadetta-sema/src/imports.rs::order_own_then_module_then_use_then_glob_then_prelude kind=unit
    #[test]
    fn order_own_then_module_then_use_then_glob_then_prelude() {
        // `v` is a member of User, a module item, an explicit use and a glob name.
        let shop = file("pub model v { w = 1 }\npub model g { w = 2 }\n");
        let main = file(
            "use acme.shop::v\nuse acme.shop::*\nmodel v { w = 3 }\nmodel User { v = 1\n  x = v }\n",
        );
        let (mut r, errs) = Resolver::new(vec![
            pkg("acme.rover", &["acme.shop"], vec![(&[], &main)]),
            pkg("acme.shop", &[], vec![(&[], &shop)]),
        ]);
        assert!(errs.is_empty());
        r.add_prelude(["mm".to_owned(), "g".to_owned()]);
        let c = ctx("acme.rover", &[], "User");
        // 1. own member beats everything.
        let got = r.resolve_ref(&c, &expr_of("v"), None).unwrap();
        assert_eq!(item_path(&got), "acme.rover::User.v");
        // 2. a module item beats the explicit use of the same name.
        let c2 = ctx("acme.rover", &[], "Other");
        let got = r.resolve_ref(&c2, &expr_of("v.w"), None).unwrap();
        assert_eq!(item_path(&got), "acme.rover::v.w");
        // 4 beats 5: a glob name beats the prelude name `g`.
        let got = r.resolve_ref(&c2, &expr_of("g.w"), None).unwrap();
        assert_eq!(item_path(&got), "acme.shop::g.w");
        // 5. the prelude is last.
        let got = r.resolve_ref(&c2, &expr_of("mm"), None).unwrap();
        assert_eq!(got, Resolved::Prelude("mm".to_owned()));
    }

    // frob:tests crates/cadetta-sema/src/imports.rs::explicit_use_beats_glob kind=unit
    #[test]
    fn explicit_use_beats_glob() {
        let a = file("pub model Same { v = 1 }\n");
        let b = file("pub model Same { v = 2 }\n");
        let user = file("use package::a::Same\nuse package::b::*\nmodel User { x = 1 }\n");
        let (r, _) = Resolver::new(vec![pkg(
            "p",
            &[],
            vec![(&[], &user), (&["a"], &a), (&["b"], &b)],
        )]);
        let got = r
            .resolve_ref(&ctx("p", &[], "User"), &expr_of("Same.v"), None)
            .unwrap();
        assert_eq!(item_path(&got), "p::a::Same.v");
    }

    // frob:tests crates/cadetta-sema/src/imports.rs::glob_clash_is_e004_only_at_use kind=unit
    #[test]
    fn glob_clash_is_e004_only_at_use() {
        let a = file("pub model Same { v = 1 }\npub model OnlyA { v = 1 }\n");
        let b = file("pub model Same { v = 2 }\n");
        let user = file("use package::a::*\nuse package::b::*\nmodel User { x = 1 }\n");
        let (r, errs) = Resolver::new(vec![pkg(
            "p",
            &[],
            vec![(&[], &user), (&["a"], &a), (&["b"], &b)],
        )]);
        assert!(errs.is_empty(), "the clash alone is not an error");
        let c = ctx("p", &[], "User");
        // An unambiguous glob name still resolves.
        assert!(r.resolve_ref(&c, &expr_of("OnlyA.v"), None).is_ok());
        // The ambiguous name fires E004 at the use.
        let e = r.resolve_ref(&c, &expr_of("Same.v"), None).unwrap_err();
        assert_eq!(e.code, "E004");
        assert_eq!(e.related.len(), 1);
        // An unknown name is E002.
        let e = r.resolve_ref(&c, &expr_of("Nope.v"), None).unwrap_err();
        assert_eq!(e.code, "E002");
    }

    // frob:tests crates/cadetta-sema/src/imports.rs::pub_use_reexport_keeps_definition_path kind=unit
    #[test]
    fn pub_use_reexport_keeps_definition_path() {
        let lib = file("pub use self::fasteners::{ScrewJoint, MetricThread}\n");
        let fasteners =
            file("pub model ScrewJoint { pitch = 1mm }\npub model MetricThread { d = 5mm }\n");
        let user = file("model User { x = 1 }\n");
        let (r, errs) = Resolver::new(vec![
            pkg("acme.rover", &["acme.shop"], vec![(&[], &user)]),
            pkg(
                "acme.shop",
                &[],
                vec![(&[], &lib), (&["fasteners"], &fasteners)],
            ),
        ]);
        assert!(errs.is_empty());
        let c = ctx("acme.rover", &[], "User");
        let got = r
            .resolve_path(&c, &path_of("acme.shop::ScrewJoint.pitch"), None)
            .unwrap();
        assert_eq!(item_path(&got), "acme.shop::fasteners::ScrewJoint.pitch");
        let got = r
            .resolve_path(&c, &path_of("acme.shop::MetricThread"), None)
            .unwrap();
        assert_eq!(item_path(&got), "acme.shop::fasteners::MetricThread");
    }

    // frob:tests crates/cadetta-sema/src/imports.rs::private_use_is_not_reexported kind=unit
    #[test]
    fn private_use_is_not_reexported() {
        let lib = file("use self::fasteners::ScrewJoint\n");
        let fasteners = file("pub model ScrewJoint { pitch = 1mm }\n");
        let user = file("model User { x = 1 }\n");
        let (r, _) = Resolver::new(vec![
            pkg("acme.rover", &["acme.shop"], vec![(&[], &user)]),
            pkg(
                "acme.shop",
                &[],
                vec![(&[], &lib), (&["fasteners"], &fasteners)],
            ),
        ]);
        let e = r
            .resolve_path(
                &ctx("acme.rover", &[], "User"),
                &path_of("acme.shop::ScrewJoint"),
                None,
            )
            .unwrap_err();
        assert_eq!(e.code, "E002");
    }

    // frob:tests crates/cadetta-sema/src/imports.rs::qualified_path_needs_no_use kind=unit
    #[test]
    fn qualified_path_needs_no_use() {
        let shop = file("pub model Policy { limit = 1mm }\n");
        let deep = file("model Inner { y = 1mm }\nmodel Reader { r = 1 }\n");
        let user = file("model User { x = 1 }\n");
        let (r, _) = Resolver::new(vec![
            pkg(
                "acme.rover",
                &["acme.shop"],
                vec![(&[], &user), (&["a", "b"], &deep)],
            ),
            pkg("acme.shop", &[], vec![(&[], &shop)]),
        ]);
        // Legal from any module, here the root and a nested one.
        for c in [
            ctx("acme.rover", &[], "User"),
            ctx("acme.rover", &["a", "b"], "Reader"),
        ] {
            let got = r
                .resolve_path(&c, &path_of("acme.shop::Policy.limit"), None)
                .unwrap();
            assert_eq!(item_path(&got), "acme.shop::Policy.limit");
            let got = r
                .resolve_path(&c, &path_of("acme.rover::a::b::Inner.y"), None)
                .unwrap();
            assert_eq!(item_path(&got), "acme.rover::a::b::Inner.y");
        }
    }

    // frob:tests crates/cadetta-sema/src/imports.rs::prelude_has_builtins_and_registered_names_only kind=unit
    #[test]
    fn prelude_has_builtins_and_registered_names_only() {
        let std_f = file("pub model ScrewJoint { pitch = 1mm }\n");
        let user = file("model User { x = 1 }\n");
        let (mut r, _) = Resolver::new(vec![
            pkg("acme.rover", &[], vec![(&[], &user)]),
            pkg("std", &[], vec![(&[], &std_f)]),
        ]);
        r.add_prelude(["mm".to_owned(), "Length".to_owned()]);
        let c = ctx("acme.rover", &[], "User");
        for name in ["mm", "Length", "Bool", "Table", "origin"] {
            let got = r.resolve_ref(&c, &expr_of(name), None).unwrap();
            assert_eq!(got, Resolved::Prelude(name.to_owned()), "{name}");
        }
        // Nothing from std is in scope without a use.
        let e = r
            .resolve_ref(&c, &expr_of("ScrewJoint.pitch"), None)
            .unwrap_err();
        assert_eq!(e.code, "E002");
        // The fully qualified path still works.
        assert!(
            r.resolve_path(&c, &path_of("std::ScrewJoint.pitch"), None)
                .is_ok()
        );
    }

    // frob:ticket 01M4V13RVF8WWS2VMYE8WN9V3R
    // frob:tests crates/cadetta-sema/src/resolve.rs::check_use_targets kind=unit
    #[test]
    fn unused_use_with_missing_target_is_e002_eagerly() {
        let shop = file("pub model Real { v = 1 }\n");
        let user = file(
            "use acme.shop::Real\nuse acme.shop::Ghost\nuse acme.nowhere::Thing\nuse acme.shop::*\nmodel User { x = 1 }\n",
        );
        let (r, errs) = Resolver::new(vec![
            pkg("acme.rover", &["acme.shop"], vec![(&[], &user)]),
            pkg("acme.shop", &[], vec![(&[], &shop)]),
        ]);
        assert!(errs.is_empty());
        let got = r.check_use_targets();
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got.iter().all(|e| e.code == "E002"));
    }

    // frob:tests crates/cadetta-sema/src/imports.rs::declaration_order_does_not_change_resolution kind=unit
    #[test]
    fn declaration_order_does_not_change_resolution() {
        let shop = file("pub model Same { v = 1 }\n");
        let fwd = file("use acme.shop::Same\nmodel User { x = 1 }\nmodel Later { y = 1 }\n");
        let rev = file("model Later { y = 1 }\nmodel User { x = 1 }\nuse acme.shop::Same\n");
        let run = |main: &File| {
            let (r, _) = Resolver::new(vec![
                pkg("acme.rover", &["acme.shop"], vec![(&[], main)]),
                pkg("acme.shop", &[], vec![(&[], &shop)]),
            ]);
            r.resolve_ref(&ctx("acme.rover", &[], "User"), &expr_of("Same.v"), None)
                .map(|x| item_path(&x))
                .map_err(|e| e.code)
        };
        assert_eq!(run(&fwd), run(&rev));
        assert_eq!(run(&fwd), Ok("acme.shop::Same.v".to_owned()));
    }
}
