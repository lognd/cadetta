//! frob:ticket 01M4SWN8DY2SQYR5GCTQQM16Z9
//! `cadetta cite pin NAME` (config-cli.md 3; obligations.md 2.3).
//!
//! Downloads the URL a `cite NAME: T { url = "https://..." }` points at into the
//! package's `cites/` directory, content-addressed as
//! `cites/<first 16 hex of blake3>-<file name>`, and rewrites the `cite`
//! to that package-relative path so S004 can detect drift. Pinning the
//! same bytes twice yields the same path. Downloading and all file access
//! go through [`Io`], so the command does no I/O of its own (CAP001).

use std::io;
use std::path::{Path, PathBuf};

use cadetta_syntax::ast::{CiteValue, ItemKind};
use cadetta_syntax::parser::parse;

use crate::project::Project;

/// Number of hex digits of the blake3 digest kept in a pinned file name.
const DIGEST_CHARS: usize = 16;

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
    /// No `cite NAME` exists in the scoped packages.
    #[error("no citation named `{0}`")]
    NotFound(String),
    /// More than one module declares `cite NAME`.
    #[error("citation `{0}` is declared in several modules: {1}")]
    Ambiguous(String, String),
    /// The citation is not a http(s) URL (a path or a standard).
    #[error("citation `{name}` is not a URL (`{target}`); nothing to pin")]
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
    /// The new `cite` target, relative to the package directory.
    pub local: String,
    /// The blake3 digest of the downloaded bytes (hex).
    pub digest: String,
}

/// A `cite NAME = "..."` found in one source text.
struct Found {
    target: String,
    /// Byte range of the string literal in the source.
    literal: std::ops::Range<usize>,
}

/// Locate `cite NAME` with a string target in `source`.
fn find_cite(source: &str, name: &str) -> Option<Found> {
    let parsed = parse(source);
    for item in parsed.file.items() {
        let ItemKind::Cite(cite) = &item.kind else {
            continue;
        };
        if cite.name.name != name {
            continue;
        }
        // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
        // Draft 1.5: the pinnable URL is the `url` field of the typed record; its string
        // literal is what gets rewritten (the pin redesign is another ticket).
        let Some(field) = cite.fields.iter().find(|f| f.name.name == "url") else {
            return Some(Found {
                target: String::new(),
                literal: item.span.start..item.span.start,
            });
        };
        let CiteValue::Text(target) = &field.value else {
            continue;
        };
        let text = source.get(field.span.start..field.span.end)?;
        let eq = text.find('=')? + 1;
        let rest = &text[eq..];
        let lead = rest.len() - rest.trim_start().len();
        let start = field.span.start + eq + lead;
        let end = start + rest.trim().len();
        return Some(Found {
            target: target.clone(),
            literal: start..end,
        });
    }
    None
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

/// Pin the URL citation `name` found in the packages `scope` selects.
pub fn pin(
    project: &Project,
    scope: &[usize],
    name: &str,
    io: &Io<'_>,
) -> Result<Pinned, CiteError> {
    let mut hits = Vec::new();
    for &i in scope {
        let Some(package) = project.packages.get(i) else {
            continue;
        };
        for module in &package.modules {
            if let Some(found) = find_cite(&module.source, name) {
                hits.push((package, module, found));
            }
        }
    }
    if hits.len() > 1 {
        let files: Vec<String> = hits
            .iter()
            .map(|(_, m, _)| m.file.display().to_string())
            .collect();
        return Err(CiteError::Ambiguous(name.to_owned(), files.join(", ")));
    }
    let Some((package, module, found)) = hits.pop() else {
        return Err(CiteError::NotFound(name.to_owned()));
    };
    let url = found.target;
    if !is_url(&url) {
        tracing::warn!(%name, target = %url, "citation is not a URL");
        return Err(CiteError::NotUrl {
            name: name.to_owned(),
            target: url,
        });
    }
    tracing::info!(%name, %url, "downloading citation");
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
    let mut rewritten = module.source.clone();
    rewritten.replace_range(found.literal, &format!("\"{local}\""));
    (io.write)(&module.file, rewritten.as_bytes()).map_err(|source| CiteError::Write {
        path: module.file.clone(),
        source,
    })?;
    tracing::info!(%name, %local, %digest, bytes = bytes.len(), "citation pinned");
    Ok(Pinned {
        source_file: module.file.clone(),
        url,
        local,
        digest,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::config::Config;
    use crate::project::{Module, PackageDir};

    const SRC: &str = "cite colson: web {\n    url = \"https://colson.example/2-series.pdf?v=2\"\n}\ncite local: web {\n    url = \"docs/r.md\"\n}\n";

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
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::pin
    #[test]
    fn url_cite_is_stored_in_cites_and_rewritten() {
        let writes: RefCell<Vec<(PathBuf, Vec<u8>)>> = RefCell::new(Vec::new());
        let io = Io {
            fetch: &|_| Ok(b"pdf bytes".to_vec()),
            write: &|p, b| {
                writes.borrow_mut().push((p.to_owned(), b.to_vec()));
                Ok(())
            },
        };
        let got = pin(&project(SRC), &[0], "colson", &io).unwrap();
        let digest = blake3::hash(b"pdf bytes").to_hex().to_string();
        let local = format!("cites/{}-2-series.pdf", &digest[..16]);
        assert_eq!(got.local, local);
        assert_eq!(got.digest, digest);
        let writes = writes.borrow();
        assert_eq!(
            writes[0],
            (PathBuf::from("/p").join(&local), b"pdf bytes".to_vec())
        );
        assert_eq!(writes[1].0, PathBuf::from("/p/src/lib.cdt"));
        let text = String::from_utf8(writes[1].1.clone()).unwrap();
        assert_eq!(
            text,
            format!(
                "cite colson: web {{\n    url = \"{local}\"\n}}\ncite local: web {{\n    url = \"docs/r.md\"\n}}\n"
            )
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::pin
    #[test]
    fn non_url_missing_and_failed_downloads_are_errors() {
        let io = Io {
            fetch: &|_| Err("offline".into()),
            write: &|_, _| Ok(()),
        };
        let p = project(SRC);
        assert!(matches!(
            pin(&p, &[0], "local", &io),
            Err(CiteError::NotUrl { .. })
        ));
        assert!(matches!(
            pin(&p, &[0], "nope", &io),
            Err(CiteError::NotFound(_))
        ));
        assert!(matches!(
            pin(&p, &[0], "colson", &io),
            Err(CiteError::Fetch { .. })
        ));
    }

    // frob:tests crates/cadetta-cli/src/cmd/cite.rs::pin
    #[test]
    fn same_name_in_two_modules_is_ambiguous() {
        let mut p = project(SRC);
        let mut m = p.packages[0].modules[0].clone();
        m.file = PathBuf::from("/p/src/b.cdt");
        p.packages[0].modules.push(m);
        let io = Io {
            fetch: &|_| Ok(vec![]),
            write: &|_, _| Ok(()),
        };
        assert!(matches!(
            pin(&p, &[0], "colson", &io),
            Err(CiteError::Ambiguous(..))
        ));
    }

    #[test]
    fn file_names_are_sanitised() {
        assert_eq!(
            file_name("https://x.example/a b/c%20d.pdf?x=1#f"),
            "c_20d.pdf"
        );
        assert_eq!(file_name("https://x.example/"), "download");
    }
}
