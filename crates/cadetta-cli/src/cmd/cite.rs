//! frob:ticket 01M4SWN8DY2SQYR5GCTQQM16Z9
//! frob:ticket 01M4VB63RTQENVNZ4DMXBFGJX3
//! `cadetta cite pin NAME` and `cadetta cite list` (config-cli.md 3; language.md 3.9.1;
//! obligations.md 2.3).
//!
//! `pin` downloads the `url` of a typed `cite NAME: T { ... }` into the package's `cites/`
//! directory, content-addressed as `cites/<first 16 hex of blake3>-<file name>`, sets the item's
//! `file` field to that path (replacing an earlier pin), keeps `url` and adds `accessed` when the
//! item has none, editing the source in place so S004 can detect drift. Pinning the same bytes
//! twice yields the same path. Downloading and all file access go through [`Io`], so the command
//! does no I/O of its own (CAP001).
//!
//! `list` numbers the citations the scoped design reaches by the walk of language.md 3.9.1 and
//! renders them as IEEE plain text, BibLaTeX or JSON.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};

use cadetta_ir::model::{Citation, CitationType, CiteField, Project as Ir};
use cadetta_obligations::as_of::Date;
use cadetta_sema::modules::module_path_of;
use cadetta_syntax::ast::{Cite, CiteFieldDecl, CiteValue, ItemKind};
use cadetta_syntax::format::format_source;
use cadetta_syntax::parser::parse;
use serde_json::{Value, json};

use crate::cmd::ir::package_id;
use crate::project::{Module, PackageDir, Project};

/// Number of hex digits of the blake3 digest kept in a pinned file name.
const DIGEST_CHARS: usize = 16;

/// The `schema` value of `cite list --format json` (docs/schemas/cite-list.json).
pub const LIST_SCHEMA: &str = "cadetta.cite-list/1";

/// Injected effects: fetching a URL and reading and writing files.
pub struct Io<'a> {
    /// Returns the bytes behind a URL, or why they could not be fetched.
    pub fetch: &'a dyn Fn(&str) -> Result<Vec<u8>, String>,
    /// Writes a file, creating missing parent directories.
    pub write: &'a dyn Fn(&Path, &[u8]) -> io::Result<()>,
}

/// Why `cite pin` failed; always exit code 2.
#[derive(Debug, thiserror::Error)]
pub enum CiteError {
    /// No `cite` item matches NAME in the scoped packages.
    #[error("no citation named `{0}`")]
    NotFound(String),
    /// NAME matches several `cite` items.
    #[error("citation `{0}` is ambiguous: {1}")]
    Ambiguous(String, String),
    /// The citation has no `url` field to pin.
    #[error("citation `{0}` has no `url`; nothing to pin")]
    NoUrl(String),
    /// The `url` is not an http(s) address.
    #[error("citation `{name}` has a `url` that is not http(s) (`{target}`); nothing to pin")]
    NotUrl {
        /// The citation name.
        name: String,
        /// What it points at, as written.
        target: String,
    },
    /// The download failed.
    #[error("could not download `{url}`: {reason}")]
    Fetch {
        /// The URL.
        url: String,
        /// What went wrong.
        reason: String,
    },
    /// A file could not be written.
    #[error("could not write {}: {source}", path.display())]
    Write {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: io::Error,
    },
}

/// The result of a successful pin.
#[derive(Debug, PartialEq, Eq)]
pub struct Pinned {
    /// The `.cdt` file that was rewritten.
    pub source_file: PathBuf,
    /// The URL that was downloaded.
    pub url: String,
    /// The new `file` value, relative to the package directory.
    pub local: String,
    /// The blake3 digest of the downloaded bytes (hex).
    pub digest: String,
    /// The `accessed` date that was added, when the item had none.
    pub accessed: Option<String>,
}

/// A `cite` item located in one source text.
struct Found {
    /// Canonical item path.
    path: String,
    /// The `url` text, when the item has one as a string.
    url: Option<String>,
    /// Byte range of the `file` value literal, when the item has a `file` field.
    file_value: Option<Range<usize>>,
    /// Whether the item has an `accessed` field.
    has_accessed: bool,
    /// Where a new field is inserted, and how.
    insert: Insertion,
}

/// Where and how to add a field to a `cite` body.
struct Insertion {
    /// Byte offset just after the last field.
    at: usize,
    /// The text that precedes a new field: a newline and indent, or `; `.
    lead: String,
}

/// The byte range of the value of `field` (after its `=`) in `source`.
fn value_range(source: &str, field: &CiteFieldDecl) -> Option<Range<usize>> {
    let text = source.get(field.span.start..field.span.end)?;
    let eq = text.find('=')? + 1;
    let rest = &text[eq..];
    let lead = rest.len() - rest.trim_start().len();
    let start = field.span.start + eq + lead;
    Some(start..start + rest.trim().len())
}

/// How a new field is added after the last field of `cite` (whose item spans `item`).
fn insertion(source: &str, cite: &Cite, item: Range<usize>) -> Option<Insertion> {
    let last = cite.fields.last()?;
    let tail = source.get(last.span.end..item.end)?;
    let lead = if tail.contains('\n') {
        let line_start = source[..last.span.start].rfind('\n').map_or(0, |i| i + 1);
        let indent = &source[line_start..last.span.start];
        let indent = if indent.trim().is_empty() {
            indent
        } else {
            "    "
        };
        format!("\n{indent}")
    } else {
        "; ".to_owned()
    };
    Some(Insertion {
        at: last.span.end,
        lead,
    })
}

/// Every `cite` item of `module` whose canonical path ends with the `::` segments `name`.
fn find_cites(package: &PackageDir, module: &Module, name: &[&str]) -> Vec<Found> {
    let Ok(mods) = module_path_of(&module.rel_path) else {
        return Vec::new();
    };
    let prefix = std::iter::once(package_id(package).name)
        .chain(mods)
        .collect::<Vec<_>>()
        .join("::");
    let parsed = parse(&module.source);
    let mut out = Vec::new();
    for item in parsed.file.items() {
        let ItemKind::Cite(cite) = &item.kind else {
            continue;
        };
        let path = format!("{prefix}::{}", cite.name.name);
        let segs: Vec<&str> = path.split("::").collect();
        if segs.len() < name.len() || segs[segs.len() - name.len()..] != *name {
            continue;
        }
        let field = |n: &str| cite.fields.iter().find(|f| f.name.name == n);
        let Some(insert) = insertion(&module.source, cite, item.span.start..item.span.end) else {
            continue;
        };
        out.push(Found {
            url: match field("url").map(|f| &f.value) {
                Some(CiteValue::Text(t)) => Some(t.clone()),
                _ => None,
            },
            file_value: field("file").and_then(|f| value_range(&module.source, f)),
            has_accessed: field("accessed").is_some(),
            insert,
            path,
        });
    }
    out
}

/// A safe file name for the last segment of `url` (query and fragment dropped).
fn file_name(url: &str) -> String {
    let no_query = url.split(['?', '#']).next().unwrap_or(url);
    let last = no_query.rsplit('/').next().unwrap_or("");
    let clean: String = last
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if clean.is_empty() {
        "download".to_owned()
    } else {
        clean
    }
}

/// True for the URL schemes a citation can be pinned from.
fn is_url(text: &str) -> bool {
    text.starts_with("http://") || text.starts_with("https://")
}

/// Rewrite `module`'s source: set `file` to `local` and add `accessed` when missing.
fn rewrite(module: &Module, found: &Found, local: &str, accessed: Option<&str>) -> String {
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let mut added = String::new();
    match &found.file_value {
        Some(range) => edits.push((range.clone(), format!("\"{local}\""))),
        None => {
            added.push_str(&found.insert.lead);
            added.push_str(&format!("file = \"{local}\""));
        }
    }
    if let Some(date) = accessed {
        added.push_str(&found.insert.lead);
        added.push_str(&format!("accessed = \"{date}\""));
    }
    if !added.is_empty() {
        edits.push((found.insert.at..found.insert.at, added));
    }
    edits.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
    let mut text = module.source.clone();
    for (range, with) in edits {
        text.replace_range(range, &with);
    }
    // A source already in canonical form stays canonical (field order and `=` alignment).
    if format_source(&module.source).is_ok_and(|f| f == module.source)
        && let Ok(canonical) = format_source(&text)
    {
        text = canonical;
    }
    text
}

/// Pin the URL citation `name` found in the packages `scope` selects; `today` is the `accessed`
/// date added when the item has none.
pub fn pin(
    project: &Project,
    scope: &[usize],
    name: &str,
    today: Date,
    io: &Io<'_>,
) -> Result<Pinned, CiteError> {
    let wanted: Vec<&str> = name.split("::").collect();
    let mut hits = Vec::new();
    for &i in scope {
        let Some(package) = project.packages.get(i) else {
            continue;
        };
        for module in &package.modules {
            for found in find_cites(package, module, &wanted) {
                hits.push((package, module, found));
            }
        }
    }
    if hits.len() > 1 {
        let paths: Vec<&str> = hits.iter().map(|(_, _, f)| f.path.as_str()).collect();
        tracing::warn!(%name, ?paths, "ambiguous citation name");
        return Err(CiteError::Ambiguous(name.to_owned(), paths.join(", ")));
    }
    let Some((package, module, found)) = hits.pop() else {
        tracing::warn!(%name, "no such citation");
        return Err(CiteError::NotFound(name.to_owned()));
    };
    let Some(url) = found.url.clone() else {
        tracing::warn!(path = %found.path, "citation has no url");
        return Err(CiteError::NoUrl(found.path));
    };
    if !is_url(&url) {
        tracing::warn!(path = %found.path, target = %url, "citation url is not http(s)");
        return Err(CiteError::NotUrl {
            name: found.path,
            target: url,
        });
    }
    tracing::info!(path = %found.path, %url, "downloading citation");
    let bytes = (io.fetch)(&url).map_err(|reason| {
        tracing::error!(%url, %reason, "download failed");
        CiteError::Fetch {
            url: url.clone(),
            reason,
        }
    })?;
    let digest = blake3::hash(&bytes).to_hex().to_string();
    let local = format!("cites/{}-{}", &digest[..DIGEST_CHARS], file_name(&url));
    let pinned_file = package.dir.join(&local);
    (io.write)(&pinned_file, &bytes).map_err(|source| CiteError::Write {
        path: pinned_file.clone(),
        source,
    })?;
    let accessed = (!found.has_accessed).then(|| today.to_string());
    let rewritten = rewrite(module, &found, &local, accessed.as_deref());
    (io.write)(&module.file, rewritten.as_bytes()).map_err(|source| CiteError::Write {
        path: module.file.clone(),
        source,
    })?;
    tracing::info!(path = %found.path, %local, %digest, bytes = bytes.len(), ?accessed, "citation pinned");
    Ok(Pinned {
        source_file: module.file.clone(),
        url,
        local,
        digest,
        accessed,
    })
}

// ---- cite list ----

/// The `--format` values of `cite list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFormat {
    /// IEEE plain text, one `[n] ...` line per citation.
    Ieee,
    /// BibLaTeX entries keyed by canonical path.
    Bibtex,
    /// The IR citations with their numbers (`cite-list.json`).
    Json,
}

impl std::str::FromStr for ListFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "ieee" => Ok(Self::Ieee),
            "bibtex" => Ok(Self::Bibtex),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "unknown cite list format `{other}` (expected ieee, bibtex or json)"
            )),
        }
    }
}

/// A node that can cite: its own citation and the nodes it reads.
struct Node {
    citation: Option<String>,
    deps: Vec<String>,
}

/// Number the citations reached from the nodes of `scoped` (in-scope IRs) by the walk of
/// language.md 3.9.1; `all` supplies the nodes and citations the dependencies resolve in.
///
/// Nodes are visited in canonical-path byte order, each node's own citation before those of what
/// it reads; a citation takes the next number when first met.
pub fn reached<'a>(
    scoped: &[&'a Ir],
    all: &[&'a Ir],
    in_scope: &dyn Fn(&str) -> bool,
) -> Vec<&'a Citation> {
    let mut nodes: BTreeMap<String, Node> = BTreeMap::new();
    for ir in all {
        for component in &ir.components {
            nodes.insert(
                component.path.to_string(),
                Node {
                    citation: component.citation.as_ref().and_then(|c| c.cite.clone()),
                    deps: Vec::new(),
                },
            );
            for member in &component.members {
                let mut deps: Vec<String> = member.deps.iter().map(ToString::to_string).collect();
                deps.sort();
                nodes.insert(
                    member.path.to_string(),
                    Node {
                        citation: member.citation.as_ref().and_then(|c| c.cite.clone()),
                        deps,
                    },
                );
            }
        }
    }
    let citations: HashMap<&str, &Citation> = all
        .iter()
        .flat_map(|ir| ir.citations.iter())
        .map(|c| (c.path.as_str(), c))
        .collect();
    let roots: Vec<String> = scoped
        .iter()
        .flat_map(|ir| {
            ir.components.iter().map(|c| c.path.to_string()).chain(
                ir.components
                    .iter()
                    .flat_map(|c| c.members.iter().map(|m| m.path.to_string())),
            )
        })
        .filter(|p| in_scope(p))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut out: Vec<&Citation> = Vec::new();
    let mut seen_nodes: HashSet<String> = HashSet::new();
    let mut seen_cites: HashSet<&str> = HashSet::new();
    for root in &roots {
        visit(
            root,
            &nodes,
            &citations,
            &mut seen_nodes,
            &mut seen_cites,
            &mut out,
        );
    }
    tracing::debug!(roots = roots.len(), reached = out.len(), "cite walk done");
    out
}

/// Depth-first step of [`reached`].
fn visit<'a>(
    path: &str,
    nodes: &BTreeMap<String, Node>,
    citations: &HashMap<&str, &'a Citation>,
    seen_nodes: &mut HashSet<String>,
    seen_cites: &mut HashSet<&'a str>,
    out: &mut Vec<&'a Citation>,
) {
    let Some(node) = nodes.get(path) else {
        tracing::trace!(%path, "cite walk: dependency is not a node");
        return;
    };
    if !seen_nodes.insert(path.to_owned()) {
        return;
    }
    if let Some(cite) = &node.citation {
        match citations.get(cite.as_str()) {
            Some(c) if seen_cites.insert(c.path.as_str()) => {
                tracing::trace!(%path, cite = %c.path, number = out.len() + 1, "cite walk: numbered");
                out.push(c);
            }
            Some(_) => {}
            None => tracing::debug!(%path, %cite, "cite walk: citation not in the IR"),
        }
    }
    for dep in &node.deps {
        visit(dep, nodes, citations, seen_nodes, seen_cites, out);
    }
}

/// IEEE month abbreviations, January first.
const MONTHS: [&str; 12] = [
    "Jan.", "Feb.", "Mar.", "Apr.", "May", "Jun.", "Jul.", "Aug.", "Sep.", "Oct.", "Nov.", "Dec.",
];

/// A `YYYY`, `YYYY-MM` or `YYYY-MM-DD` date at its own precision.
struct PartialDate<'a> {
    year: &'a str,
    month: Option<usize>,
    day: Option<u32>,
}

impl<'a> PartialDate<'a> {
    /// Parse `text`; `None` when it is not one of the three forms.
    fn parse(text: &'a str) -> Option<Self> {
        let mut parts = text.split('-');
        let year = parts.next().filter(|y| y.len() == 4 && is_digits(y))?;
        let month = match parts.next() {
            None => None,
            Some(m) => Some(
                m.parse::<usize>()
                    .ok()
                    .filter(|n| m.len() == 2 && (1..=12).contains(n))?,
            ),
        };
        let day = match parts.next() {
            None => None,
            Some(d) => Some(
                d.parse::<u32>()
                    .ok()
                    .filter(|n| d.len() == 2 && (1..=31).contains(n))?,
            ),
        };
        if parts.next().is_some() || (day.is_some() && month.is_none()) {
            return None;
        }
        Some(Self { year, month, day })
    }
}

/// True when `s` is non-empty ASCII digits.
fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// A date as IEEE prints it (`2026`, `Sep. 2026`, `Sep. 30, 2026`); other text is kept as is.
fn ieee_date(text: &str) -> String {
    match PartialDate::parse(text) {
        Some(PartialDate {
            year,
            month: Some(m),
            day: Some(d),
        }) => format!("{} {d}, {year}", MONTHS[m - 1]),
        Some(PartialDate {
            year,
            month: Some(m),
            day: None,
        }) => format!("{} {year}", MONTHS[m - 1]),
        Some(PartialDate { year, .. }) => year.to_owned(),
        None => text.to_owned(),
    }
}

/// The year of a `date` field; other text is kept as is.
fn ieee_year(text: &str) -> String {
    PartialDate::parse(text).map_or_else(|| text.to_owned(), |d| d.year.to_owned())
}

/// The non-empty text of field `name`; a list is joined with `, `.
fn field<'a>(c: &'a Citation, name: &str) -> Option<std::borrow::Cow<'a, str>> {
    match c.fields.get(name)? {
        CiteField::Text(t) if !t.trim().is_empty() => Some(t.as_str().into()),
        CiteField::List(l) if !l.is_empty() => Some(l.join(", ").into()),
        _ => None,
    }
}

/// The authors, joined as IEEE does: `A`, `A and B`, `A, B, and C`, or `A et al.` over six.
fn authors(c: &Citation) -> Option<String> {
    let list: Vec<&str> = match c.fields.get("author")? {
        CiteField::Text(t) => vec![t.as_str()],
        CiteField::List(l) => l.iter().map(String::as_str).collect(),
    };
    let list: Vec<&str> = list.into_iter().filter(|a| !a.trim().is_empty()).collect();
    match list.as_slice() {
        [] => None,
        [a] => Some((*a).to_owned()),
        [a, b] => Some(format!("{a} and {b}")),
        [first, ..] if list.len() > 6 => Some(format!("{first} et al.")),
        [init @ .., last] => Some(format!("{}, and {last}", init.join(", "))),
    }
}

/// The label a type prints before its number; empty for the types that have none.
fn label(ty: CitationType) -> Option<&'static str> {
    match ty {
        CitationType::Datasheet => Some("Datasheet"),
        CitationType::Drawing => Some("Drawing"),
        CitationType::Catalog => Some("Catalog"),
        CitationType::Quote => Some("Quotation"),
        _ => None,
    }
}

/// Join the present parts with `, `, closing a title's quote after its comma; ends without a
/// final period.
fn join_parts(parts: &[Part]) -> String {
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        match part {
            Part::Plain(t) => {
                out.push_str(t);
                if !last {
                    out.push_str(", ");
                }
            }
            Part::Title(t) => {
                out.push('"');
                out.push_str(t);
                out.push_str(if last { ".\"" } else { ",\" " });
            }
        }
    }
    out
}

/// One comma-separated part of an IEEE entry.
enum Part {
    /// Plain text.
    Plain(String),
    /// A quoted title; its comma sits inside the quotes.
    Title(String),
}

/// `text` with exactly one final period.
fn end_period(mut text: String) -> String {
    if !text.ends_with('.') && !text.ends_with(".\"") {
        text.push('.');
    }
    text
}

/// The IEEE plain-text rendering of `c` without its number (language.md 3.9.1).
pub fn ieee(c: &Citation) -> String {
    let text = match c.ty {
        CitationType::Book => ieee_book(c),
        CitationType::Article => ieee_article(c),
        _ => ieee_other(c),
    };
    end_period(text.trim().to_owned())
}

/// A book: `AUTHORS, TITLE, EDITION ed. LOCATION: PUBLISHER, YEAR.`
fn ieee_book(c: &Citation) -> String {
    let mut head: Vec<Part> = Vec::new();
    head.extend(authors(c).map(Part::Plain));
    head.extend(field(c, "title").map(|t| Part::Plain(t.into_owned())));
    let edition = field(c, "edition").map(|e| format!("{e} ed."));
    let imprint = match (field(c, "location"), field(c, "publisher")) {
        (Some(l), Some(p)) => Some(format!("{l}: {p}")),
        (Some(x), None) | (None, Some(x)) => Some(x.into_owned()),
        (None, None) => None,
    };
    let year = field(c, "date").map(|d| ieee_year(&d));
    let mut tail: Vec<Part> = Vec::new();
    tail.extend(imprint.map(Part::Plain));
    tail.extend(year.map(Part::Plain));
    match edition {
        // "ed." closes a sentence: the imprint follows after a space, not a comma.
        Some(ed) => {
            let mut first = head;
            first.push(Part::Plain(ed));
            let a = join_parts(&first);
            let a = a.trim_end_matches([',', ' ']);
            if tail.is_empty() {
                a.to_owned()
            } else {
                format!("{a} {}", join_parts(&tail))
            }
        }
        None => {
            head.extend(tail);
            join_parts(&head)
        }
    }
}

/// An article: `AUTHORS, "TITLE," JOURNAL, vol. V, no. I, pp. P, DATE, doi: DOI.`
fn ieee_article(c: &Citation) -> String {
    let mut parts: Vec<Part> = Vec::new();
    parts.extend(authors(c).map(Part::Plain));
    parts.extend(field(c, "title").map(|t| Part::Title(t.into_owned())));
    parts.extend(field(c, "journal").map(|j| Part::Plain(j.into_owned())));
    parts.extend(field(c, "volume").map(|v| Part::Plain(format!("vol. {v}"))));
    parts.extend(field(c, "issue").map(|v| Part::Plain(format!("no. {v}"))));
    parts.extend(field(c, "pages").map(|v| Part::Plain(format!("pp. {v}"))));
    parts.extend(field(c, "date").map(|d| Part::Plain(ieee_date(&d))));
    parts.extend(field(c, "doi").map(|v| Part::Plain(format!("doi: {v}"))));
    join_parts(&parts)
}

/// Any other type: `WHO, "TITLE," LABEL NUMBER, Rev. REV, EDITION ed., PUBLISHER, DATE.` then the
/// `[Online]` and `Accessed` parts when there is a `url`.
fn ieee_other(c: &Citation) -> String {
    let mut parts: Vec<Part> = Vec::new();
    parts.extend(
        authors(c)
            .or_else(|| field(c, "organization").map(|o| o.into_owned()))
            .map(Part::Plain),
    );
    parts.extend(field(c, "title").map(|t| Part::Title(t.into_owned())));
    let numbered = [
        label(c.ty).map(str::to_owned),
        field(c, "number").map(|n| n.into_owned()),
    ];
    let numbered: Vec<String> = numbered.into_iter().flatten().collect();
    if !numbered.is_empty() {
        parts.push(Part::Plain(numbered.join(" ")));
    }
    parts.extend(field(c, "rev").map(|v| Part::Plain(format!("Rev. {v}"))));
    parts.extend(field(c, "edition").map(|v| Part::Plain(format!("{v} ed."))));
    parts.extend(field(c, "publisher").map(|v| Part::Plain(v.into_owned())));
    parts.extend(field(c, "date").map(|d| Part::Plain(ieee_date(&d))));
    let mut text = join_parts(&parts);
    // "ed.," keeps its comma; a trailing separator before the end is dropped.
    text = text.trim_end_matches([',', ' ']).to_owned();
    if let Some(url) = field(c, "url") {
        let mut online = format!("[Online]. Available: {}", end_period(url.into_owned()));
        if let Some(accessed) = field(c, "accessed") {
            online.push_str(&format!(" Accessed: {}", end_period(ieee_date(&accessed))));
        }
        return if text.is_empty() {
            online
        } else {
            format!("{} {online}", end_period(text))
        };
    }
    text
}

/// The BibLaTeX entry type of a citation type.
fn bib_type(ty: CitationType) -> &'static str {
    match ty {
        CitationType::Standard => "standard",
        CitationType::Datasheet | CitationType::Catalog => "manual",
        CitationType::Book => "book",
        CitationType::Article => "article",
        CitationType::Web => "online",
        CitationType::Drawing | CitationType::Quote | CitationType::Document => "misc",
    }
}

/// Citation field names in the order BibLaTeX prints them, with the BibLaTeX field they map to.
const BIB_FIELDS: [(&str, &str); 19] = [
    ("author", "author"),
    ("organization", "organization"),
    ("title", "title"),
    ("number", "number"),
    ("rev", "version"),
    ("edition", "edition"),
    ("date", "date"),
    ("publisher", "publisher"),
    ("location", "location"),
    ("journal", "journal"),
    ("volume", "volume"),
    ("issue", "issue"),
    ("pages", "pages"),
    ("doi", "doi"),
    ("isbn", "isbn"),
    ("url", "url"),
    ("accessed", "urldate"),
    ("file", "file"),
    ("note", "note"),
];

/// One BibLaTeX entry keyed by the canonical path of `c`.
pub fn bibtex(c: &Citation) -> String {
    let mut out = format!("@{}{{{},\n", bib_type(c.ty), c.path);
    for (name, bib) in BIB_FIELDS {
        let value = match c.fields.get(name) {
            Some(CiteField::Text(t)) => t.clone(),
            Some(CiteField::List(l)) => l.join(" and "),
            None => continue,
        };
        let value = value.replace('{', "\\{").replace('}', "\\}");
        out.push_str(&format!("  {bib} = {{{value}}},\n"));
    }
    out.push_str("}\n");
    out
}

/// Render the numbered citations in `format` (a newline-terminated document).
pub fn render_list(cites: &[&Citation], format: ListFormat) -> String {
    match format {
        ListFormat::Ieee => cites
            .iter()
            .enumerate()
            .map(|(i, c)| format!("[{}] {}\n", i + 1, ieee(c)))
            .collect(),
        ListFormat::Bibtex => cites
            .iter()
            .map(|c| bibtex(c))
            .collect::<Vec<_>>()
            .join("\n"),
        ListFormat::Json => {
            let items: Vec<Value> = cites
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let mut v = serde_json::to_value(c).unwrap_or(Value::Null);
                    if let Some(m) = v.as_object_mut() {
                        m.insert("number".to_owned(), json!(i + 1));
                    }
                    v
                })
                .collect();
            let doc = json!({ "schema": LIST_SCHEMA, "citations": items });
            let mut text = serde_json::to_string_pretty(&doc).unwrap_or_default();
            text.push('\n');
            text
        }
    }
}

/// Number the citations reached in the scoped packages and render them in `format`.
pub fn list(
    project: &Project,
    scope: &[usize],
    run_scope: &crate::scope::Scope,
    format: ListFormat,
) -> Result<String, String> {
    let analysis = crate::pipeline::analyze(project, scope);
    if analysis.halted {
        return Err("a syntax error stopped analysis; run `cadetta check`".to_owned());
    }
    let all: Vec<&Ir> = analysis.packages.iter().map(|p| &p.ir).collect();
    let scoped: Vec<&Ir> = scope
        .iter()
        .filter_map(|&i| {
            let id = package_id(project.packages.get(i)?);
            analysis
                .packages
                .iter()
                .find(|p| p.name == id.name)
                .map(|p| &p.ir)
        })
        .collect();
    let cites = reached(&scoped, &all, &|p| run_scope.contains(p));
    tracing::info!(count = cites.len(), ?format, "cite list");
    Ok(render_list(&cites, format))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::config::Config;
    use crate::project::{Module, PackageDir};

    const SRC: &str = "cite colson: web {\n    title = \"Colson 2 series\"\n    url = \"https://colson.example/2-series.pdf?v=2\"\n}\ncite local: web {\n    url = \"docs/r.md\"\n}\ncite bare: web {\n    title = \"No url\"\n}\n";

    fn today() -> Date {
        Date::parse("2026-10-13").unwrap()
    }

    fn project(source: &str) -> Project {
        let config = Config::parse("[package]\nname = \"acme.p\"\nversion = \"0.1.0\"\n").unwrap();
        Project {
            root: PathBuf::from("/p"),
            config: config.clone(),
            packages: vec![PackageDir {
                dir: PathBuf::from("/p"),
                config,
                modules: vec![Module {
                    rel_path: "lib.cdt".into(),
                    file: PathBuf::from("/p/src/lib.cdt"),
                    source: source.to_owned(),
                }],
            }],
            dependencies: vec![],
        }
    }

    /// Run a successful pin and return the result with the rewritten source.
    fn pin_ok(source: &str, name: &str) -> (Pinned, String, Vec<(PathBuf, Vec<u8>)>) {
        let writes: RefCell<Vec<(PathBuf, Vec<u8>)>> = RefCell::new(Vec::new());
        let io = Io {
            fetch: &|_| Ok(b"pdf bytes".to_vec()),
            write: &|p, b| {
                writes.borrow_mut().push((p.to_owned(), b.to_vec()));
                Ok(())
            },
        };
        let got = pin(&project(source), &[0], name, today(), &io).unwrap();
        let writes = writes.into_inner();
        let text = String::from_utf8(writes[1].1.clone()).unwrap();
        (got, text, writes)
    }

    fn local() -> String {
        let digest = blake3::hash(b"pdf bytes").to_hex().to_string();
        format!("cites/{}-2-series.pdf", &digest[..16])
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::pin
    #[test]
    fn pin_stores_bytes_sets_file_keeps_url_and_adds_accessed() {
        let (got, text, writes) = pin_ok(SRC, "colson");
        let digest = blake3::hash(b"pdf bytes").to_hex().to_string();
        assert_eq!(got.local, local());
        assert_eq!(got.digest, digest);
        assert_eq!(got.accessed.as_deref(), Some("2026-10-13"));
        assert_eq!(
            writes[0],
            (PathBuf::from("/p").join(local()), b"pdf bytes".to_vec())
        );
        assert_eq!(writes[1].0, PathBuf::from("/p/src/lib.cdt"));
        // the rest of the file is untouched; canonical order puts accessed before file
        assert!(text.contains(&format!("file = \"{}\"", local())), "{text}");
        assert!(text.contains("url = \"https://colson.example/2-series.pdf?v=2\""));
        assert!(text.contains("accessed = \"2026-10-13\""));
        assert!(text.contains("cite local: web {\n    url = \"docs/r.md\"\n}"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::pin
    #[test]
    fn pin_replaces_an_earlier_file_and_keeps_an_existing_accessed() {
        let src = "cite c: web {\n    url = \"https://x.example/2-series.pdf\"\n    accessed = \"2025-01-02\"\n    file = \"cites/0000000000000000-old.pdf\"\n}\n";
        let (got, text, _) = pin_ok(src, "c");
        assert_eq!(got.accessed, None);
        assert!(!text.contains("old.pdf"), "{text}");
        assert!(text.contains(&format!("file = \"{}\"", local())));
        assert!(text.contains("accessed = \"2025-01-02\""));
        assert_eq!(text.matches("file").count(), 1);
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::pin
    #[test]
    fn pin_edits_a_single_line_body_and_unformatted_source_in_place() {
        let src = "cite  c : web { url = \"https://x.example/2-series.pdf\" }\n";
        let (_, text, _) = pin_ok(src, "c");
        assert_eq!(
            text,
            format!(
                "cite  c : web {{ url = \"https://x.example/2-series.pdf\"
    file = \"{}\"
    accessed = \"2026-10-13\" }}
",
                local()
            )
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::pin
    #[test]
    fn pin_matches_bare_relative_and_qualified_names() {
        for name in ["colson", "acme.p::colson"] {
            let (got, _, _) = pin_ok(SRC, name);
            assert_eq!(got.url, "https://colson.example/2-series.pdf?v=2");
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::pin
    #[test]
    fn pin_rejects_missing_url_unknown_non_http_and_failed_downloads() {
        let io = Io {
            fetch: &|_| Err("offline".into()),
            write: &|_, _| Ok(()),
        };
        let p = project(SRC);
        let go = |n: &str| pin(&p, &[0], n, today(), &io);
        assert!(matches!(go("bare"), Err(CiteError::NoUrl(_))));
        assert!(matches!(go("local"), Err(CiteError::NotUrl { .. })));
        assert!(matches!(go("nope"), Err(CiteError::NotFound(_))));
        assert!(matches!(go("colson"), Err(CiteError::Fetch { .. })));
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::pin
    #[test]
    fn same_name_in_two_modules_is_ambiguous_unless_qualified() {
        let mut p = project(SRC);
        let mut m = p.packages[0].modules[0].clone();
        m.rel_path = "b.cdt".into();
        m.file = PathBuf::from("/p/src/b.cdt");
        p.packages[0].modules.push(m);
        let io = Io {
            fetch: &|_| Ok(b"x".to_vec()),
            write: &|_, _| Ok(()),
        };
        assert!(matches!(
            pin(&p, &[0], "colson", today(), &io),
            Err(CiteError::Ambiguous(..))
        ));
        assert!(pin(&p, &[0], "b::colson", today(), &io).is_ok());
        assert!(pin(&p, &[0], "acme.p::colson", today(), &io).is_ok());
    }

    #[test]
    fn file_names_are_sanitised() {
        assert_eq!(
            file_name("https://x.example/a b/c%20d.pdf?x=1#f"),
            "c_20d.pdf"
        );
        assert_eq!(file_name("https://x.example/"), "download");
    }

    // ---- reference list ----

    fn mk(ty: CitationType, fields: &[(&str, &str)]) -> Citation {
        let fields = fields
            .iter()
            .map(|(k, v)| {
                let value = if *k == "author" {
                    CiteField::List(v.split(';').map(str::to_owned).collect())
                } else {
                    CiteField::Text((*v).to_owned())
                };
                ((*k).to_owned(), value)
            })
            .collect();
        Citation::new("p::c", ty, fields, None)
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::ieee
    #[test]
    fn ieee_book_template() {
        let c = mk(
            CitationType::Book,
            &[
                ("author", "J. Doe;A. Smith"),
                ("title", "Machinery's Handbook"),
                ("edition", "31st"),
                ("location", "New York"),
                ("publisher", "Industrial Press"),
                ("date", "2020-03-04"),
            ],
        );
        assert_eq!(
            ieee(&c),
            "J. Doe and A. Smith, Machinery's Handbook, 31st ed. New York: Industrial Press, 2020."
        );
        // no edition: the imprint follows a comma; no location: just the publisher
        let c = mk(
            CitationType::Book,
            &[("title", "T"), ("publisher", "P"), ("date", "2020")],
        );
        assert_eq!(ieee(&c), "T, P, 2020.");
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::ieee
    #[test]
    fn ieee_article_template() {
        let c = mk(
            CitationType::Article,
            &[
                ("author", "A. Author"),
                ("title", "On gears"),
                ("journal", "J. Mech. Des."),
                ("volume", "12"),
                ("issue", "3"),
                ("pages", "10-20"),
                ("date", "2021-06"),
                ("doi", "10.1000/xyz"),
            ],
        );
        assert_eq!(
            ieee(&c),
            "A. Author, \"On gears,\" J. Mech. Des., vol. 12, no. 3, pp. 10-20, Jun. 2021, doi: 10.1000/xyz."
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::ieee
    #[test]
    fn ieee_other_template_with_url_and_accessed() {
        let c = mk(
            CitationType::Datasheet,
            &[
                ("organization", "Sunon"),
                ("title", "MF80251V1 DC brushless fan"),
                ("number", "MF80251V1-1000U-A99"),
                ("rev", "A"),
                ("edition", "2nd"),
                ("publisher", "Sunon Inc"),
                ("date", "2024-03"),
                ("url", "https://sunon.example/mf.pdf"),
                ("accessed", "2026-09-30"),
            ],
        );
        assert_eq!(
            ieee(&c),
            "Sunon, \"MF80251V1 DC brushless fan,\" Datasheet MF80251V1-1000U-A99, Rev. A, 2nd ed., Sunon Inc, Mar. 2024. [Online]. Available: https://sunon.example/mf.pdf. Accessed: Sep. 30, 2026."
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::ieee
    #[test]
    fn ieee_who_is_authors_else_organization() {
        let a = mk(
            CitationType::Web,
            &[
                ("author", "J. Doe"),
                ("organization", "Acme"),
                ("title", "T"),
            ],
        );
        assert_eq!(ieee(&a), "J. Doe, \"T.\"");
        let o = mk(
            CitationType::Web,
            &[("organization", "Acme"), ("title", "T")],
        );
        assert_eq!(ieee(&o), "Acme, \"T.\"");
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::authors
    #[test]
    fn ieee_authors_join_and_truncate() {
        let n = |k: usize| {
            let names: Vec<String> = (1..=k).map(|i| format!("A{i}")).collect();
            authors(&mk(CitationType::Book, &[("author", &names.join(";"))])).unwrap()
        };
        assert_eq!(n(1), "A1");
        assert_eq!(n(2), "A1 and A2");
        assert_eq!(n(3), "A1, A2, and A3");
        assert_eq!(n(6), "A1, A2, A3, A4, A5, and A6");
        assert_eq!(n(7), "A1 et al.");
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::label
    #[test]
    fn ieee_labels_by_type() {
        let t = |ty| ieee(&mk(ty, &[("organization", "O"), ("number", "N 1")]));
        assert_eq!(t(CitationType::Datasheet), "O, Datasheet N 1.");
        assert_eq!(t(CitationType::Drawing), "O, Drawing N 1.");
        assert_eq!(t(CitationType::Catalog), "O, Catalog N 1.");
        assert_eq!(t(CitationType::Quote), "O, Quotation N 1.");
        assert_eq!(t(CitationType::Standard), "O, N 1.");
        assert_eq!(t(CitationType::Web), "O, N 1.");
        assert_eq!(t(CitationType::Document), "O, N 1.");
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::ieee_date
    #[test]
    fn ieee_dates_print_at_their_precision() {
        assert_eq!(ieee_date("2026"), "2026");
        assert_eq!(ieee_date("2026-09"), "Sep. 2026");
        assert_eq!(ieee_date("2026-09-30"), "Sep. 30, 2026");
        assert_eq!(ieee_date("2026-05-05"), "May 5, 2026");
        assert_eq!(ieee_date("2024-03"), "Mar. 2024");
        assert_eq!(ieee_date("someday"), "someday");
        let months: Vec<String> = (1..=12)
            .map(|m| ieee_date(&format!("2020-{m:02}")))
            .collect();
        assert_eq!(
            months.join(" "),
            "Jan. 2020 Feb. 2020 Mar. 2020 Apr. 2020 May 2020 Jun. 2020 Jul. 2020 Aug. 2020 Sep. 2020 Oct. 2020 Nov. 2020 Dec. 2020"
        );
        let c = mk(CitationType::Document, &[("title", "T"), ("date", "2026")]);
        assert_eq!(ieee(&c), "\"T,\" 2026.");
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::ieee
    #[test]
    fn ieee_online_parts_need_a_url() {
        let no_url = mk(
            CitationType::Web,
            &[
                ("organization", "O"),
                ("title", "T"),
                ("accessed", "2026-09-30"),
            ],
        );
        assert_eq!(ieee(&no_url), "O, \"T.\"");
        let url_only = mk(
            CitationType::Web,
            &[
                ("organization", "O"),
                ("date", "2026"),
                ("url", "https://a.example/x"),
            ],
        );
        assert_eq!(
            ieee(&url_only),
            "O, 2026. [Online]. Available: https://a.example/x."
        );
        let bare = mk(CitationType::Web, &[("url", "https://a.example/x")]);
        assert_eq!(ieee(&bare), "[Online]. Available: https://a.example/x.");
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::ieee
    #[test]
    fn ieee_separators_collapse_and_one_final_period() {
        // edition last: "ed." supplies the period, none is doubled
        let c = mk(
            CitationType::Catalog,
            &[("organization", "O"), ("edition", "2025")],
        );
        assert_eq!(ieee(&c), "O, Catalog, 2025 ed.");
        // "ed." followed by more parts keeps its comma
        let c = mk(
            CitationType::Catalog,
            &[("organization", "O"), ("edition", "2025"), ("date", "2025")],
        );
        assert_eq!(ieee(&c), "O, Catalog, 2025 ed., 2025.");
        // missing middle parts leave no stray separators
        let c = mk(
            CitationType::Drawing,
            &[("organization", "O"), ("rev", "03"), ("date", "2024-03-01")],
        );
        assert_eq!(ieee(&c), "O, Drawing, Rev. 03, Mar. 1, 2024.");
        // a url that ends in a period is not doubled
        let c = mk(
            CitationType::Web,
            &[("title", "T"), ("url", "https://a.example/x.")],
        );
        assert_eq!(ieee(&c), "\"T.\" [Online]. Available: https://a.example/x.");
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::bibtex
    #[test]
    fn bibtex_maps_types_and_fields() {
        let c = mk(
            CitationType::Datasheet,
            &[
                ("author", "A;B"),
                ("organization", "Sunon"),
                ("rev", "A"),
                ("url", "https://a.example/x"),
                ("accessed", "2026-09-30"),
            ],
        );
        assert_eq!(
            bibtex(&c),
            "@manual{p::c,\n  author = {A and B},\n  organization = {Sunon},\n  version = {A},\n  url = {https://a.example/x},\n  urldate = {2026-09-30},\n}\n"
        );
        let kinds: Vec<&str> = CitationType::ALL.iter().map(|t| bib_type(*t)).collect();
        assert_eq!(
            kinds,
            [
                "standard", "manual", "misc", "manual", "book", "article", "online", "misc", "misc"
            ]
        );
    }

    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("cadetta-cite-{}-{n}", std::process::id()));
            std::fs::create_dir_all(p.join("src")).unwrap();
            std::fs::write(
                p.join("cadetta.toml"),
                "[package]\nname = \"acme.t\"\nversion = \"0.1.0\"\n",
            )
            .unwrap();
            Self(p)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const DESIGN: &str = "cite alpha: standard {\n  organization = \"ISO\"\n  number = \"ISO 1\"\n  title = \"Alpha\"\n}\ncite beta: standard {\n  organization = \"ISO\"\n  number = \"ISO 2\"\n  title = \"Beta\"\n}\ncite zeta: standard {\n  organization = \"ISO\"\n  number = \"ISO 3\"\n  title = \"Zeta\"\n}\ncite unused: standard {\n  organization = \"ISO\"\n  number = \"ISO 4\"\n  title = \"Unused\"\n}\nmodel M {\n  a: Length = c * 2 @alpha\n  b: Length = 5mm @beta\n  c: Length = 1mm @zeta\n}\n";

    fn run_list(source: &str, globs: &[&str], format: ListFormat) -> Result<String, String> {
        let dir = Dir::new();
        std::fs::write(dir.0.join("src/lib.cdt"), source).unwrap();
        let (project, scope) = Project::open(Some(&dir.0), None).unwrap();
        let run_scope = crate::scope::Scope::new::<&str, &str>(&[], globs).unwrap();
        list(&project, &scope, &run_scope, format)
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::reached
    #[test]
    fn list_numbers_by_path_order_with_dependencies_after_their_reader() {
        let out = run_list(DESIGN, &[], ListFormat::Ieee).unwrap();
        // M.a (alpha) reads M.c (zeta), which is numbered before M.b (beta); `unused` is absent
        assert_eq!(
            out,
            "[1] ISO, \"Alpha,\" ISO 1.\n[2] ISO, \"Zeta,\" ISO 3.\n[3] ISO, \"Beta,\" ISO 2.\n"
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::reached
    #[test]
    fn list_scope_limits_the_roots_but_not_what_they_read() {
        let out = run_list(DESIGN, &["acme.t::M.b"], ListFormat::Ieee).unwrap();
        assert_eq!(out, "[1] ISO, \"Beta,\" ISO 2.\n");
        let out = run_list(DESIGN, &["acme.t::M.a"], ListFormat::Ieee).unwrap();
        assert_eq!(out.lines().count(), 2);
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::render_list
    #[test]
    fn list_bibtex_is_keyed_by_canonical_path() {
        let out = run_list(DESIGN, &[], ListFormat::Bibtex).unwrap();
        assert!(out.starts_with("@standard{acme.t::alpha,\n"), "{out}");
        assert!(out.contains("@standard{acme.t::zeta,"));
        assert!(!out.contains("unused"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::render_list
    #[test]
    fn list_json_validates_against_the_schema() {
        let out = run_list(DESIGN, &[], ListFormat::Json).unwrap();
        let doc: Value = serde_json::from_str(&out).unwrap();
        let schema: Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/cite-list.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&doc), "{doc}");
        assert_eq!(doc["schema"], LIST_SCHEMA);
        let numbers: Vec<u64> = doc["citations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["number"].as_u64().unwrap())
            .collect();
        assert_eq!(numbers, [1, 2, 3]);
        assert_eq!(doc["citations"][0]["path"], "acme.t::alpha");
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::list
    #[test]
    fn list_of_a_broken_design_is_an_error() {
        assert!(run_list("model {", &[], ListFormat::Ieee).is_err());
    }

    #[test]
    fn list_format_parses() {
        assert_eq!("ieee".parse(), Ok(ListFormat::Ieee));
        assert_eq!("bibtex".parse(), Ok(ListFormat::Bibtex));
        assert_eq!("json".parse(), Ok(ListFormat::Json));
        assert!("text".parse::<ListFormat>().is_err());
    }
}
