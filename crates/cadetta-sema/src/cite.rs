//! Citations: `cite` declarations, `@name:"locator"` use sites and inheritance
//! (language.md 3.9, ir.md 3).

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use cadetta_ir::digest::digest_bytes;
use cadetta_ir::model::{Citation, CitationKind};
use cadetta_syntax::ast::{self, CiteName, CiteTarget};
use tracing::{debug, trace, warn};

use crate::modules::resolve_package_file;

/// The built-in pseudo-citations and their kinds (language.md 3.9).
const PSEUDO: &[(&str, CitationKind)] = &[
    ("measured", CitationKind::Measured),
    ("assumed", CitationKind::Assumed),
    ("estimate", CitationKind::Estimate),
];

/// A citation name that no `cite` declares and no pseudo-citation matches.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UnknownCite {
    /// The unresolved name.
    pub name: String,
}

impl fmt::Display for UnknownCite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown citation `@{}`", self.name)
    }
}

impl std::error::Error for UnknownCite {}

/// A declared `cite name = ...`: its kind and target text.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Decl {
    kind: CitationKind,
    target: String,
}

/// The `cite` declarations visible in one scope.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CiteTable {
    decls: BTreeMap<String, Decl>,
}

/// True for a web address target.
fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

impl CiteTable {
    /// Builds the table from `cite` declarations; a later duplicate replaces an earlier one.
    pub fn new<'a>(cites: impl IntoIterator<Item = &'a ast::Cite>) -> Self {
        let mut decls = BTreeMap::new();
        for c in cites {
            let (kind, target) = match &c.target {
                CiteTarget::Str(s) if is_url(s) => (CitationKind::Url, s.clone()),
                CiteTarget::Str(s) => (CitationKind::File, s.clone()),
                CiteTarget::Path(p) => (CitationKind::Standard, p.text()),
            };
            debug!(name = %c.name.name, ?kind, %target, "cite declared");
            decls.insert(c.name.name.clone(), Decl { kind, target });
        }
        Self { decls }
    }

    /// Resolves a use-site citation to its IR form.
    ///
    /// `package_dir` locates local file citations so their digest can be
    /// recorded; a missing file leaves the digest empty (S002 reports it). A
    /// literal citation is text and is never touched on disk.
    pub fn resolve(
        &self,
        package_dir: Option<&Path>,
        c: &ast::Citation,
    ) -> Result<Citation, UnknownCite> {
        let locator = c.locator.clone();
        let (name, target, kind) = match &c.name {
            CiteName::Literal(text) => (text.clone(), text.clone(), CitationKind::Standard),
            CiteName::Named(id) => {
                if let Some((_, kind)) = PSEUDO.iter().find(|(n, _)| *n == id.name) {
                    (id.name.clone(), id.name.clone(), *kind)
                } else if let Some(d) = self.decls.get(&id.name) {
                    (id.name.clone(), d.target.clone(), d.kind)
                } else {
                    warn!(name = %id.name, "unknown citation");
                    return Err(UnknownCite {
                        name: id.name.clone(),
                    });
                }
            }
        };
        let file_digest = match (kind, package_dir, &c.name) {
            (CitationKind::File, Some(dir), CiteName::Named(_)) => {
                resolve_package_file(dir, &target).and_then(|p| match std::fs::read(&p) {
                    Ok(bytes) => Some(digest_bytes(&bytes)),
                    Err(e) => {
                        debug!(path = %p.display(), error = %e, "cited file unreadable");
                        None
                    }
                })
            }
            _ => None,
        };
        trace!(%name, ?kind, digest = file_digest.is_some(), "citation resolved");
        Ok(Citation {
            name,
            target,
            locator,
            kind,
            file_digest,
        })
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
    use cadetta_syntax::ast::{FileEntry, ItemKind, MemberKind};
    use cadetta_syntax::parser::parse;

    const SRC: &str = "cite web = \"https://example.com/d.pdf\"\ncite spec = \"Cargo.toml\"\ncite iso = std::iso273\nots P @web:\"p.2\" {\n  a = 1mm\n  b = 2mm @spec\n  c = 3mm @\"ASTM A276\"\n  d = 4mm @measured:\"ana, 2026-10\"\n  e = 5mm @iso:\"table 1\"\n}\n";

    fn setup() -> (ast::File, CiteTable) {
        let p = parse(SRC);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        let cites: Vec<&ast::Cite> = p
            .file
            .entries
            .iter()
            .filter_map(|e| match e {
                FileEntry::Item(i) => match &i.kind {
                    ItemKind::Cite(c) => Some(c),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let t = CiteTable::new(cites);
        (p.file, t)
    }

    fn component(f: &ast::File) -> &ast::Component {
        f.components().next().expect("component")
    }

    fn member_cites(f: &ast::File) -> Vec<Option<ast::Citation>> {
        component(f)
            .members
            .iter()
            .filter_map(|m| match &m.kind {
                MemberKind::Param(p) => Some(p.citation.clone()),
                _ => None,
            })
            .collect()
    }

    // frob:tests crates/cadetta-sema/src/cite.rs::CiteTable.resolve kind=unit
    #[test]
    fn kinds_resolve() {
        let (f, t) = setup();
        let ms = member_cites(&f);
        let comp = component(&f).citation.clone().expect("component cite");
        let web = t.resolve(None, &comp).expect("web");
        assert_eq!(web.kind, CitationKind::Url);
        assert_eq!(web.locator.as_deref(), Some("p.2"));
        let lit = t.resolve(None, ms[2].as_ref().expect("c")).expect("lit");
        assert_eq!(lit.target, "ASTM A276");
        assert_eq!(lit.kind, CitationKind::Standard);
        let me = t
            .resolve(None, ms[3].as_ref().expect("d"))
            .expect("measured");
        assert_eq!(me.kind, CitationKind::Measured);
        assert_eq!(me.locator.as_deref(), Some("ana, 2026-10"));
        let std = t.resolve(None, ms[4].as_ref().expect("e")).expect("std");
        assert_eq!(std.kind, CitationKind::Standard);
        assert_eq!(std.target, "std::iso273");
    }

    // frob:tests crates/cadetta-sema/src/cite.rs::effective kind=unit
    #[test]
    fn member_citation_wins_and_others_inherit() {
        let (f, _) = setup();
        let ms = member_cites(&f);
        let comp = component(&f).citation.as_ref();
        assert_eq!(effective(ms[0].as_ref(), comp), comp);
        assert_eq!(effective(ms[1].as_ref(), comp), ms[1].as_ref());
    }

    #[test]
    fn unknown_name_is_an_error() {
        let (_, t) = setup();
        let p = parse("ots P { a = 1mm @nope }\n");
        let c = p.file.components().next().expect("c").members[0]
            .kind
            .clone();
        let MemberKind::Param(pr) = c else {
            panic!("param")
        };
        let err = t.resolve(None, pr.citation.as_ref().expect("cite"));
        assert_eq!(err.unwrap_err().name, "nope");
    }

    // frob:tests crates/cadetta-sema/src/cite.rs::CiteTable.resolve kind=unit
    #[test]
    fn local_file_digest_recorded_literal_not_checked() {
        let (f, t) = setup();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let ms = member_cites(&f);
        let file = t
            .resolve(Some(dir), ms[1].as_ref().expect("b"))
            .expect("file");
        assert_eq!(file.kind, CitationKind::File);
        let bytes = include_bytes!("../Cargo.toml");
        assert_eq!(file.file_digest, Some(digest_bytes(bytes)));
        // a literal never reads the disk
        let lit = t
            .resolve(Some(dir), ms[2].as_ref().expect("c"))
            .expect("lit");
        assert_eq!(lit.file_digest, None);
        // a missing file is not an error here (S002 reports it)
        let p = parse("cite gone = \"nope/missing.txt\"\nots P { a = 1mm @gone }\n");
        let t2 = CiteTable::new(p.file.entries.iter().filter_map(|e| match e {
            FileEntry::Item(i) => match &i.kind {
                ItemKind::Cite(c) => Some(c),
                _ => None,
            },
            _ => None,
        }));
        let MemberKind::Param(pr) = &p.file.components().next().expect("c").members[0].kind else {
            panic!("param")
        };
        let gone = t2
            .resolve(Some(dir), pr.citation.as_ref().expect("cite"))
            .expect("gone");
        assert_eq!(gone.file_digest, None);
    }
}
