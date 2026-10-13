//! Selector grammar `KIND:GLOB[,KIND:GLOB...]` over obligation ids.
//!
//! Specification: `docs/spec/obligations.md` section 2.2 and
//! `docs/spec/config-cli.md` section 3.3.
//! frob:ticket 01M4ST8EJWTGQM9CMVBJVBATJT

use thiserror::Error;
use tracing::{debug, trace, warn};

use crate::graph::{IdError, ObligationId, ObligationKind};

/// Process exit code for a usage error (config-cli.md 3.3).
pub const EXIT_USAGE: i32 = 2;

/// Errors parsing or resolving a selector; every one is a usage error (exit 2).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SelectorError {
    /// The selector text is empty or has an empty comma-separated term.
    #[error("empty selector term in `{0}`")]
    Empty(String),
    /// A term lacks `:` or has an empty glob.
    #[error("malformed selector term `{0}`: expected KIND:GLOB")]
    Malformed(String),
    /// The KIND prefix is not an obligation kind.
    #[error("unknown obligation kind `{0}` in selector")]
    UnknownKind(String),
    /// A glob has an unterminated `[` index.
    #[error("unterminated `[` in glob `{0}`")]
    BadGlob(String),
    /// The selector matched no obligation.
    #[error("selector `{0}` matches no obligation")]
    NoMatch(String),
}

impl SelectorError {
    /// The process exit code for this error (always [`EXIT_USAGE`]).
    pub fn exit_code(&self) -> i32 {
        EXIT_USAGE
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Lit(char),
    /// `*`: any run of characters inside one path segment.
    Star,
    /// `**`: any run of characters, across segments.
    DoubleStar,
    /// `[*]`: any index, `[N]` or `[A..B]`.
    AnyIndex,
}

/// A path glob (obligations.md 2.2): `*` within a segment, `**` across segments, `[*]` any index.
///
/// Segments are separated by `::`, `.` and `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glob {
    src: String,
    toks: Vec<Tok>,
}

fn is_sep(c: char) -> bool {
    matches!(c, '.' | '/' | ':')
}

impl Glob {
    /// Compile a glob string.
    pub fn new(src: &str) -> Result<Self, SelectorError> {
        let cs: Vec<char> = src.chars().collect();
        let mut toks = Vec::new();
        let mut i = 0;
        while i < cs.len() {
            match cs[i] {
                '*' if cs.get(i + 1) == Some(&'*') => {
                    toks.push(Tok::DoubleStar);
                    i += 2;
                }
                '*' => {
                    toks.push(Tok::Star);
                    i += 1;
                }
                '[' if cs.get(i + 1) == Some(&'*') && cs.get(i + 2) == Some(&']') => {
                    toks.push(Tok::AnyIndex);
                    i += 3;
                }
                '[' => {
                    if !cs[i..].contains(&']') {
                        return Err(SelectorError::BadGlob(src.to_owned()));
                    }
                    toks.push(Tok::Lit('['));
                    i += 1;
                }
                c => {
                    toks.push(Tok::Lit(c));
                    i += 1;
                }
            }
        }
        Ok(Self {
            src: src.to_owned(),
            toks,
        })
    }

    /// The glob source text.
    pub fn as_str(&self) -> &str {
        &self.src
    }

    /// Whether `path` matches the whole glob.
    pub fn matches(&self, path: &str) -> bool {
        let cs: Vec<char> = path.chars().collect();
        go(&self.toks, &cs)
    }
}

/// Length of an index `[...]` at the start of `cs` (digits or `A..B`), if any.
fn index_len(cs: &[char]) -> Option<usize> {
    if cs.first() != Some(&'[') {
        return None;
    }
    let end = cs.iter().position(|&c| c == ']')?;
    let body = &cs[1..end];
    (!body.is_empty()
        && body
            .iter()
            .all(|c| c.is_ascii_digit() || matches!(*c, '.' | '*')))
    .then_some(end + 1)
}

fn go(toks: &[Tok], cs: &[char]) -> bool {
    let Some((t, rest)) = toks.split_first() else {
        return cs.is_empty();
    };
    match t {
        Tok::Lit(c) => cs.first() == Some(c) && go(rest, &cs[1..]),
        Tok::AnyIndex => index_len(cs).is_some_and(|n| go(rest, &cs[n..])),
        Tok::DoubleStar => (0..=cs.len()).any(|n| go(rest, &cs[n..])),
        Tok::Star => {
            for n in 0..=cs.len() {
                if go(rest, &cs[n..]) {
                    return true;
                }
                match cs.get(n) {
                    Some(&c) if !is_sep(c) => {}
                    _ => return false,
                }
            }
            false
        }
    }
}

/// One `KIND:GLOB` term of a selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    /// Obligation kind to match.
    pub kind: ObligationKind,
    /// Glob over the canonical path.
    pub glob: Glob,
}

impl Term {
    /// Whether the obligation id matches this term.
    pub fn matches(&self, id: &ObligationId) -> bool {
        id.kind() == self.kind && self.glob.matches(id.path())
    }
}

/// A parsed selector: one or more terms, matching the union of their ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    src: String,
    terms: Vec<Term>,
}

impl Selector {
    /// Parse `KIND:GLOB[,KIND:GLOB...]`.
    pub fn parse(s: &str) -> Result<Self, SelectorError> {
        let mut terms = Vec::new();
        for raw in s.split(',') {
            let raw = raw.trim();
            if raw.is_empty() {
                warn!(selector = s, "selector: empty term");
                return Err(SelectorError::Empty(s.to_owned()));
            }
            let (k, g) = match ObligationId::parse(raw) {
                Ok(id) => (id.kind(), id.path().to_owned()),
                Err(IdError::UnknownKind(k)) => return Err(SelectorError::UnknownKind(k)),
                Err(IdError::Malformed(_)) => return Err(SelectorError::Malformed(raw.to_owned())),
            };
            trace!(kind = k.name(), glob = %g, "selector: term");
            terms.push(Term {
                kind: k,
                glob: Glob::new(&g)?,
            });
        }
        debug!(selector = s, terms = terms.len(), "selector: parsed");
        Ok(Self {
            src: s.to_owned(),
            terms,
        })
    }

    /// The parsed terms.
    pub fn terms(&self) -> &[Term] {
        &self.terms
    }

    /// Whether any term matches the id.
    pub fn matches(&self, id: &ObligationId) -> bool {
        self.terms.iter().any(|t| t.matches(id))
    }

    /// The ids among `universe` that match, in input order; none is [`SelectorError::NoMatch`] (exit 2).
    pub fn resolve<'a>(
        &self,
        universe: impl IntoIterator<Item = &'a ObligationId>,
    ) -> Result<Vec<ObligationId>, SelectorError> {
        let hits: Vec<ObligationId> = universe
            .into_iter()
            .filter(|id| self.matches(id))
            .cloned()
            .collect();
        if hits.is_empty() {
            warn!(selector = %self.src, "selector: matches nothing");
            return Err(SelectorError::NoMatch(self.src.clone()));
        }
        debug!(selector = %self.src, matched = hits.len(), "selector: resolved");
        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[&str]) -> Vec<ObligationId> {
        v.iter().map(|s| ObligationId::parse(s).unwrap()).collect()
    }

    // frob:tests crates/cadetta-obligations/src/selector.rs::Selector.parse
    #[test]
    fn parses_comma_separated_terms() {
        let s = Selector::parse("fit:acme.rover::Axle.*,check:acme.rover::**").unwrap();
        assert_eq!(s.terms().len(), 2);
        assert_eq!(s.terms()[0].kind, ObligationKind::Fit);
    }

    #[test]
    fn rejects_bad_terms() {
        assert!(matches!(Selector::parse(""), Err(SelectorError::Empty(_))));
        assert!(matches!(
            Selector::parse("fit:a,"),
            Err(SelectorError::Empty(_))
        ));
        assert!(matches!(
            Selector::parse("fit"),
            Err(SelectorError::Malformed(_))
        ));
        assert!(matches!(
            Selector::parse("fit:"),
            Err(SelectorError::Malformed(_))
        ));
        assert!(matches!(
            Selector::parse("nope:x"),
            Err(SelectorError::UnknownKind(_))
        ));
        assert!(matches!(
            Selector::parse("fit:a[1"),
            Err(SelectorError::BadGlob(_))
        ));
    }

    // frob:tests crates/cadetta-obligations/src/selector.rs::Selector.resolve
    #[test]
    fn matches_kind_and_canonical_path() {
        let u = ids(&[
            "fit:acme.rover::Axle.press",
            "fit:acme.rover::Hub.press",
            "check:acme.rover::Axle.len",
            "check:acme.rover::sub::Deep.x",
        ]);
        let s = Selector::parse("fit:acme.rover::Axle.*,check:acme.rover::**").unwrap();
        let got: Vec<String> = s
            .resolve(&u)
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect();
        assert_eq!(
            got,
            [
                "fit:acme.rover::Axle.press",
                "check:acme.rover::Axle.len",
                "check:acme.rover::sub::Deep.x"
            ]
        );
    }

    #[test]
    fn star_stays_within_a_segment() {
        let g = Glob::new("acme.rover::*").unwrap();
        assert!(g.matches("acme.rover::Axle"));
        assert!(!g.matches("acme.rover::Axle.press"));
        assert!(!g.matches("acme.rover::a::b"));
        assert!(Glob::new("acme.**").unwrap().matches("acme.rover::a::b"));
    }

    #[test]
    fn any_index_matches_indices() {
        let g = Glob::new("a::Rover/axle[*].d").unwrap();
        assert!(g.matches("a::Rover/axle[2].d"));
        assert!(g.matches("a::Rover/axle[0..3].d"));
        assert!(g.matches("a::Rover/axle[*].d"));
        assert!(!g.matches("a::Rover/axle.d"));
    }

    #[test]
    fn sync_paths_are_file_paths() {
        let u = ids(&["sync:cad/axle.sldprt", "sync:cad/hub.sldprt"]);
        let s = Selector::parse("sync:cad/*.sldprt").unwrap();
        assert_eq!(s.resolve(&u).unwrap().len(), 2);
    }

    // frob:tests crates/cadetta-obligations/src/selector.rs::SelectorError.exit_code
    #[test]
    fn no_match_is_exit_2() {
        let u = ids(&["fit:a::X"]);
        let e = Selector::parse("check:a::*")
            .unwrap()
            .resolve(&u)
            .unwrap_err();
        assert!(matches!(e, SelectorError::NoMatch(_)));
        assert_eq!(e.exit_code(), 2);
        let e = Selector::parse("fit:b::*")
            .unwrap()
            .resolve(&u)
            .unwrap_err();
        assert_eq!(e.exit_code(), EXIT_USAGE);
    }
}
