//! Citations: typed `cite` items, `@path:"locator"` use sites and inheritance
//! (language.md 3.9, ir.md 3).
// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y

use std::collections::BTreeMap;
use std::path::Path;

use cadetta_ir::digest::digest_bytes;
use cadetta_ir::model::{Citation, CitationType, CiteBuiltin, CiteField, CiteRef};
use cadetta_syntax::ast::{self, CiteName, CiteValue, FileEntry, ItemKind, MemberKind};
use cadetta_syntax::lexer::Span;
use tracing::{debug, trace, warn};

use crate::modules::resolve_package_file;
use crate::resolve::{Ctx, ItemClass, ModuleSrc, PackageSrc, Resolved, Resolver};

/// The pseudo-citations, resolved before any path lookup (language.md 3.9).
const PSEUDO: &[(&str, CiteBuiltin)] = &[
    ("measured", CiteBuiltin::Measured),
    ("assumed", CiteBuiltin::Assumed),
    ("estimate", CiteBuiltin::Estimate),
];

/// Every field name a `cite` accepts, on any type (language.md 3.9).
pub const FIELDS: &[&str] = &[
    "author",
    "organization",
    "title",
    "number",
    "rev",
    "edition",
    "date",
    "publisher",
    "location",
    "journal",
    "volume",
    "issue",
    "pages",
    "doi",
    "isbn",
    "url",
    "accessed",
    "file",
    "note",
];

/// A citation diagnostic: an E-code, a message and where it is.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CiteDiag {
    /// The diagnostic code (`E002`, `E003`, `E006`).
    pub code: &'static str,
    /// The message.
    pub message: String,
    /// The primary span.
    pub span: Span,
}

fn diag(code: &'static str, span: Span, message: String) -> CiteDiag {
    warn!(code, %message, "cite: diagnostic");
    CiteDiag {
        code,
        message,
        span,
    }
}

/// True for `YYYY`, `YYYY-MM` or `YYYY-MM-DD` with a real month and day.
fn valid_date(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    let num = |p: &str, n: usize| p.len() == n && p.bytes().all(|b| b.is_ascii_digit());
    match parts.as_slice() {
        [y] => num(y, 4),
        [y, m] => num(y, 4) && num(m, 2) && matches!(m.parse::<u32>(), Ok(1..=12)),
        [y, m, d] => {
            if !(num(y, 4) && num(m, 2) && num(d, 2)) {
                return false;
            }
            let (y, m, d): (u32, u32, u32) = (
                y.parse().unwrap_or(0),
                m.parse().unwrap_or(0),
                d.parse().unwrap_or(0),
            );
            let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
            let max = match m {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 if leap => 29,
                2 => 28,
                _ => return false,
            };
            (1..=max).contains(&d)
        }
        _ => false,
    }
}

/// Checks one `cite` declaration: type, field names, shapes, dates, URLs, repeats, name.
pub fn validate_decl(c: &ast::Cite) -> Vec<CiteDiag> {
    let mut out = Vec::new();
    if PSEUDO.iter().any(|(n, _)| *n == c.name.name) {
        out.push(diag(
            "E003",
            c.name.span,
            format!(
                "`{}` is a pseudo-citation and cannot name a `cite`",
                c.name.name
            ),
        ));
    }
    if CitationType::parse(&c.ty.name).is_none() {
        let known: Vec<&str> = CitationType::ALL.iter().map(|t| t.name()).collect();
        out.push(diag(
            "E006",
            c.ty.span,
            format!(
                "unknown cite type `{}`; expected one of {}",
                c.ty.name,
                known.join(", ")
            ),
        ));
    }
    let mut seen: Vec<&str> = Vec::new();
    for f in &c.fields {
        let name = f.name.name.as_str();
        if !FIELDS.contains(&name) {
            out.push(diag(
                "E006",
                f.name.span,
                format!("unknown cite field `{name}`"),
            ));
            continue;
        }
        if seen.contains(&name) {
            out.push(diag(
                "E003",
                f.name.span,
                format!("cite field `{name}` is given twice"),
            ));
            continue;
        }
        seen.push(name);
        match (&f.value, name) {
            (CiteValue::Text(t), "date" | "accessed") if !valid_date(t) => out.push(diag(
                "E006",
                f.span,
                format!("`{name}` must be YYYY, YYYY-MM or YYYY-MM-DD, found \"{t}\""),
            )),
            (CiteValue::Text(t), "url")
                if !(t.starts_with("http://") || t.starts_with("https://")) =>
            {
                out.push(diag(
                    "E006",
                    f.span,
                    format!("`url` must start with http:// or https://, found \"{t}\""),
                ));
            }
            (CiteValue::Text(_), _) | (CiteValue::List(_), "author") => {}
            (CiteValue::List(_), _) => out.push(diag(
                "E006",
                f.span,
                format!("`{name}` takes a single string; only `author` may be a list"),
            )),
            (CiteValue::Other, _) => out.push(diag(
                "E006",
                f.span,
                format!("`{name}` must be a string (or a list of strings for `author`)"),
            )),
        }
    }
    out
}

/// Every use-site citation in a file: component, member, `require`, process, table and argument.
pub fn use_sites(file: &ast::File) -> Vec<&ast::Citation> {
    fn args<'a>(a: &'a Option<Vec<ast::Arg>>, out: &mut Vec<&'a ast::Citation>) {
        out.extend(a.iter().flatten().filter_map(|x| x.citation.as_ref()));
    }
    let mut out = Vec::new();
    for e in &file.entries {
        let FileEntry::Item(i) = e else { continue };
        match &i.kind {
            ItemKind::Component(c) => {
                out.extend(c.citation.as_ref());
                for m in &c.members {
                    out.extend(m.citation.as_ref());
                    match &m.kind {
                        MemberKind::Param(p) => out.extend(p.citation.as_ref()),
                        MemberKind::Require(r) => out.extend(r.citation.as_ref()),
                        // frob:ticket 01M4VB630NZKXA6Q9D7253FATR
                        MemberKind::Solver(sv) => out.extend(sv.citation.as_ref()),
                        MemberKind::Part(p) => {
                            args(&p.args, &mut out);
                            for ct in p.primary.iter().chain(p.alts.iter()) {
                                args(&ct.args, &mut out);
                            }
                        }
                        _ => {}
                    }
                }
            }
            ItemKind::Require(r) => out.extend(r.citation.as_ref()),
            ItemKind::Process(p) => out.extend(p.citation.as_ref()),
            ItemKind::Table(t) => out.extend(t.citation.as_ref()),
            ItemKind::Solver(sv) => out.extend(sv.citation.as_ref()),
            _ => {}
        }
    }
    out
}

/// Resolves citation paths and builds IR citation records over a set of packages.
pub struct CiteIndex<'a> {
    resolver: Resolver<'a>,
    decls: BTreeMap<String, &'a ast::Cite>,
}

impl<'a> CiteIndex<'a> {
    /// Indexes every `cite` item of `packages` by canonical path text.
    pub fn new(packages: &[PackageSrc<'a>]) -> Self {
        let (resolver, _) = Resolver::new(packages.to_vec());
        let mut decls = BTreeMap::new();
        for p in packages {
            for m in &p.modules {
                for e in &m.file.entries {
                    if let FileEntry::Item(i) = e
                        && let ItemKind::Cite(c) = &i.kind
                    {
                        let mut key = format!("{}::", p.name);
                        for s in &m.path {
                            key.push_str(s);
                            key.push_str("::");
                        }
                        key.push_str(&c.name.name);
                        decls.entry(key).or_insert(c);
                    }
                }
            }
        }
        debug!(cites = decls.len(), "cite: index built");
        Self { resolver, decls }
    }

    /// Lowers a use-site citation written in `package`/`module` to a [`CiteRef`].
    pub fn lower_ref(
        &self,
        package: &str,
        module: &[String],
        c: &ast::Citation,
    ) -> Result<CiteRef, CiteDiag> {
        let locator = c.locator.clone();
        match &c.name {
            CiteName::Literal(t) => Ok(CiteRef::note(t.clone(), locator)),
            CiteName::Named(p) => {
                if p.segments.len() == 1
                    && let Some((_, b)) = PSEUDO.iter().find(|(n, _)| *n == p.segments[0].name)
                {
                    return Ok(CiteRef::builtin(*b, locator));
                }
                let ctx = Ctx {
                    package: package.to_owned(),
                    module: module.to_vec(),
                    component: None,
                    locals: Vec::new(),
                };
                match self.resolver.resolve_path(&ctx, p, None) {
                    Ok(Resolved::Item {
                        path,
                        class: ItemClass::Cite,
                    }) => {
                        trace!(cite = %path, "cite: resolved");
                        Ok(CiteRef::item(path.to_string(), locator))
                    }
                    Ok(_) => Err(diag(
                        "E006",
                        c.span,
                        format!("`@{}` does not name a `cite` item", p.text()),
                    )),
                    Err(e) => Err(diag(e.code, e.span, e.message)),
                }
            }
        }
    }

    /// The IR record of the cite item at canonical `path`; `package_dir` hashes its `file`
    /// when the item lives in the package being lowered (`own_package`).
    pub fn citation(
        &self,
        path: &str,
        own_package: &str,
        package_dir: Option<&Path>,
    ) -> Option<Citation> {
        let decl = self.decls.get(path)?;
        let Some(ty) = CitationType::parse(&decl.ty.name) else {
            warn!(%path, ty = %decl.ty.name, "cite: unknown type, record skipped");
            return None;
        };
        let mut fields = BTreeMap::new();
        for f in &decl.fields {
            let v = match &f.value {
                CiteValue::Text(t) => CiteField::Text(t.clone()),
                CiteValue::List(l) if f.name.name == "author" => CiteField::List(l.clone()),
                _ => continue,
            };
            if FIELDS.contains(&f.name.name.as_str()) {
                fields.entry(f.name.name.clone()).or_insert(v);
            }
        }
        let in_pkg = path.split("::").next() == Some(own_package);
        let file_digest = match (fields.get("file"), package_dir) {
            (Some(CiteField::Text(rel)), Some(dir)) if in_pkg => resolve_package_file(dir, rel)
                .and_then(|p| match std::fs::read(&p) {
                    Ok(bytes) => Some(digest_bytes(&bytes)),
                    Err(e) => {
                        debug!(path = %p.display(), error = %e, "cited file unreadable");
                        None
                    }
                }),
            _ => None,
        };
        Some(Citation::new(path, ty, fields, file_digest))
    }

    /// All citation diagnostics of one module: declarations and use sites.
    pub fn check_module(&self, package: &str, module: &ModuleSrc<'_>) -> Vec<CiteDiag> {
        let mut out = Vec::new();
        for e in &module.file.entries {
            if let FileEntry::Item(i) = e
                && let ItemKind::Cite(c) = &i.kind
            {
                out.extend(validate_decl(c));
            }
        }
        for c in use_sites(module.file) {
            if let Err(d) = self.lower_ref(package, &module.path, c) {
                out.push(d);
            }
        }
        out
    }
}

/// A member's effective citation: its own if present, else the component's.
pub fn effective<'a>(
    member: Option<&'a ast::Citation>,
    component: Option<&'a ast::Citation>,
) -> Option<&'a ast::Citation> {
    member.or(component)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::parser::parse;

    fn check(src: &str) -> Vec<(&'static str, String)> {
        let parsed = parse(src);
        assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
        let enums: &[crate::resolve::EnumDecl] = &[];
        let pkgs = vec![PackageSrc {
            defaults: Default::default(),
            name: "p".into(),
            deps: vec![],
            modules: vec![ModuleSrc {
                path: vec![],
                file: &parsed.file,
                enums,
            }],
        }];
        let idx = CiteIndex::new(&pkgs);
        idx.check_module("p", &pkgs[0].modules[0])
            .into_iter()
            .map(|d| (d.code, d.message))
            .collect()
    }

    fn codes(src: &str) -> Vec<&'static str> {
        check(src).into_iter().map(|(c, _)| c).collect()
    }

    // frob:tests crates/cadetta-sema/src/cite.rs::validate_decl
    #[test]
    fn well_formed_record_is_clean() {
        let src = "cite a: standard {\n organization = \"ISO\"\n number = \"1\"\n date = \"2010-05\"\n author = [\"x\", \"y\"]\n url = \"https://x.example\"\n}\n";
        assert!(codes(src).is_empty());
    }

    // frob:tests crates/cadetta-sema/src/cite.rs::validate_decl
    #[test]
    fn unknown_type_field_date_url_and_shape_are_e006() {
        assert_eq!(codes("cite a: nope { title = \"t\" }\n"), ["E006"]);
        assert_eq!(codes("cite a: web { colour = \"t\" }\n"), ["E006"]);
        assert_eq!(codes("cite a: web { date = \"2010-13\" }\n"), ["E006"]);
        assert_eq!(codes("cite a: web { accessed = \"May\" }\n"), ["E006"]);
        assert_eq!(codes("cite a: web { url = \"ftp://x\" }\n"), ["E006"]);
        assert_eq!(codes("cite a: web { title = [\"a\"] }\n"), ["E006"]);
        assert_eq!(codes("cite a: web { title = 3 }\n"), ["E006"]);
    }

    // frob:tests crates/cadetta-sema/src/cite.rs::validate_decl
    #[test]
    fn repeated_field_and_pseudo_name_are_e003() {
        assert_eq!(
            codes("cite a: web { title = \"a\"; title = \"b\" }\n"),
            ["E003"]
        );
        assert_eq!(codes("cite measured: web { title = \"a\" }\n"), ["E003"]);
    }

    // frob:tests crates/cadetta-sema/src/cite.rs::valid_date
    #[test]
    fn date_forms() {
        for ok in ["2010", "2010-05", "2010-05-31"] {
            assert!(valid_date(ok), "{ok}");
        }
        for bad in ["", "10", "2010-00", "2010-05-32", "2010/05", "2010-5"] {
            assert!(!valid_date(bad), "{bad}");
        }
    }

    // frob:tests crates/cadetta-sema/src/cite.rs::lower_ref
    #[test]
    fn use_sites_lower_to_cite_note_builtin_and_locator() {
        let src = "cite a: web { title = \"t\" }\n\
                   pub ots C {\n  x = 1mm @a:\"p.2\"\n  y = 2mm @\"note\"\n  z = 3mm @measured:\"caliper\"\n}\n";
        let parsed = parse(src);
        let enums: &[crate::resolve::EnumDecl] = &[];
        let pkgs = vec![PackageSrc {
            defaults: Default::default(),
            name: "p".into(),
            deps: vec![],
            modules: vec![ModuleSrc {
                path: vec![],
                file: &parsed.file,
                enums,
            }],
        }];
        let idx = CiteIndex::new(&pkgs);
        let refs: Vec<CiteRef> = use_sites(&parsed.file)
            .into_iter()
            .map(|c| idx.lower_ref("p", &[], c).unwrap())
            .collect();
        assert_eq!(refs.len(), 3, "{refs:?}");
        assert_eq!(refs[0], CiteRef::item("p::a", Some("p.2".into())));
        assert_eq!(refs[1], CiteRef::note("note", None));
        assert_eq!(
            refs[2],
            CiteRef::builtin(CiteBuiltin::Measured, Some("caliper".into()))
        );
        let c = idx.citation("p::a", "p", None).unwrap();
        assert_eq!(c.ty, CitationType::Web);
        assert_eq!(c.text("title"), Some("t"));
        assert!(c.file_digest.is_none());
    }

    // frob:tests crates/cadetta-sema/src/cite.rs::check_module
    #[test]
    fn unknown_and_non_cite_targets_are_errors() {
        let c = codes("pub ots C {\n  x = 1mm @nowhere\n}\n");
        assert_eq!(c, ["E002"]);
        let c = codes("pub ots D {\n  q = 1mm\n}\npub ots C {\n  x = 1mm @D\n}\n");
        assert_eq!(c, ["E006"]);
    }
}
