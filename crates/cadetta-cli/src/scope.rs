//! frob:ticket 01M4SWN8JNSJZ12W5X2V99RR6A
//! Scoping a run to part of a project: `--module PATH` prefixes and
//! canonical-path globs (config-cli.md 3, obligations.md 2.2).
//!
//! A node is in scope when it matches any module prefix (if any were
//! given) and any glob (if any were given); no filters means everything.

use thiserror::Error;

/// A malformed `--module` value or glob (a usage error, exit 2).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScopeError {
    /// A glob or module prefix was empty.
    #[error("empty {what} in scope")]
    Empty {
        /// Which kind of term was empty: `glob` or `module`.
        what: &'static str,
    },
    /// A glob had a `[` with no closing `]`.
    #[error("unterminated `[` in glob `{0}`")]
    Unterminated(String),
}

/// One glob path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Seg {
    /// `**`: zero or more segments.
    Deep,
    /// Any index segment (`[*]`).
    AnyIndex,
    /// A segment with optional `*` wildcards, kept as literal pieces.
    Text(Vec<String>),
}

/// A compiled canonical-path glob (`*` within a segment, `**` across, `[*]` any index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glob {
    segs: Vec<Seg>,
}

/// Split a path into segments on `::`, `.`, `/`, with `[..]` indices as their own segments.
fn split_path(path: &str) -> Result<Vec<String>, ScopeError> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = path.chars().peekable();
    let flush = |cur: &mut String, out: &mut Vec<String>| {
        if !cur.is_empty() {
            out.push(std::mem::take(cur));
        }
    };
    while let Some(c) = chars.next() {
        match c {
            ':' if chars.peek() == Some(&':') => {
                chars.next();
                flush(&mut cur, &mut out);
            }
            '.' if chars.peek() == Some(&'.') && cur.starts_with('[') => cur.push('.'),
            '.' | '/' => flush(&mut cur, &mut out),
            '[' => {
                flush(&mut cur, &mut out);
                cur.push('[');
                loop {
                    match chars.next() {
                        Some(']') => {
                            cur.push(']');
                            break;
                        }
                        Some(d) => cur.push(d),
                        None => return Err(ScopeError::Unterminated(path.to_owned())),
                    }
                }
                flush(&mut cur, &mut out);
            }
            _ => cur.push(c),
        }
    }
    flush(&mut cur, &mut out);
    Ok(out)
}

impl Glob {
    /// Compile a glob, rejecting empty and unterminated terms.
    pub fn new(text: &str) -> Result<Self, ScopeError> {
        if text.trim().is_empty() {
            return Err(ScopeError::Empty { what: "glob" });
        }
        let segs = split_path(text.trim())?
            .into_iter()
            .map(|s| match s.as_str() {
                "**" => Seg::Deep,
                "[*]" => Seg::AnyIndex,
                _ => Seg::Text(s.split('*').map(str::to_owned).collect()),
            })
            .collect();
        Ok(Self { segs })
    }

    /// Whether the canonical `path` matches this glob in full.
    pub fn matches(&self, path: &str) -> bool {
        // Paths that cannot be split (unterminated index) never match.
        let Ok(parts) = split_path(path) else {
            return false;
        };
        match_segs(&self.segs, &parts)
    }
}

/// Match glob segments against path segments, backtracking over `**`.
fn match_segs(glob: &[Seg], path: &[String]) -> bool {
    match glob.split_first() {
        None => path.is_empty(),
        Some((Seg::Deep, rest)) => (0..=path.len()).any(|n| match_segs(rest, &path[n..])),
        Some((seg, rest)) => path
            .split_first()
            .is_some_and(|(p, tail)| match_seg(seg, p) && match_segs(rest, tail)),
    }
}

/// Match one non-deep glob segment against one path segment.
fn match_seg(seg: &Seg, part: &str) -> bool {
    match seg {
        Seg::Deep => unreachable!("deep segments are handled by match_segs"),
        Seg::AnyIndex => part.starts_with('[') && part.ends_with(']'),
        Seg::Text(pieces) => {
            let (first, rest) = pieces.split_first().expect("split yields one piece");
            let Some(mut tail) = part.strip_prefix(first.as_str()) else {
                return false;
            };
            let Some((last, middle)) = rest.split_last() else {
                return tail.is_empty();
            };
            for m in middle {
                match tail.find(m.as_str()) {
                    Some(i) => tail = &tail[i + m.len()..],
                    None => return false,
                }
            }
            tail.len() >= last.len() && tail.ends_with(last.as_str())
        }
    }
}

/// A module path prefix as `::`-separated segments (`drive/motor` and `drive::motor` agree).
#[derive(Debug, Clone, PartialEq, Eq)]
struct ModulePrefix(Vec<String>);

impl ModulePrefix {
    /// Whether `path` (`pkg::mod::...::Item.member`) lies under this prefix.
    ///
    /// The prefix is compared segment-wise (`drive` never matches
    /// `driveline`), either relative to the package or fully qualified.
    fn matches(&self, path: &str) -> bool {
        let segs: Vec<&str> = path.split("::").collect();
        let starts = |from: usize| {
            segs.len() >= from + self.0.len()
                && self
                    .0
                    .iter()
                    .zip(&segs[from..])
                    .all(|(want, got)| want == got || (got.split('.').next() == Some(want)))
        };
        starts(0) || starts(1)
    }
}

/// The set of nodes a command run applies to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    modules: Vec<ModulePrefix>,
    globs: Vec<Glob>,
}

impl Scope {
    /// Build a scope from `--module` values and glob arguments.
    pub fn new<M: AsRef<str>, G: AsRef<str>>(
        modules: &[M],
        globs: &[G],
    ) -> Result<Self, ScopeError> {
        let modules = modules
            .iter()
            .map(|m| {
                let m = m.as_ref().trim();
                let segs: Vec<String> = m
                    .replace('/', "::")
                    .split("::")
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect();
                if segs.is_empty() {
                    Err(ScopeError::Empty { what: "module" })
                } else {
                    Ok(ModulePrefix(segs))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let globs = globs
            .iter()
            .map(|g| Glob::new(g.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        tracing::debug!(modules = modules.len(), globs = globs.len(), "scope built");
        Ok(Self { modules, globs })
    }

    /// Whether no filter was given (every node is in scope).
    pub fn is_all(&self) -> bool {
        self.modules.is_empty() && self.globs.is_empty()
    }

    /// Whether the canonical `path` is in scope.
    pub fn contains(&self, path: &str) -> bool {
        let in_module = self.modules.is_empty() || self.modules.iter().any(|m| m.matches(path));
        let in_glob = self.globs.is_empty() || self.globs.iter().any(|g| g.matches(path));
        in_module && in_glob
    }

    /// The paths of `all` that are in scope, in input order.
    pub fn filter<'a, I: IntoIterator<Item = &'a str>>(&self, all: I) -> Vec<&'a str> {
        all.into_iter().filter(|p| self.contains(p)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODES: [&str; 5] = [
        "acme.rover::drive::motor::Gm42A.rated_torque",
        "acme.rover::drive::Gearbox.ratio",
        "acme.rover::driveline::X.y",
        "acme.rover::frame::Rail.site[2].offset",
        "acme.rover::Axle.press",
    ];

    fn scope(m: &[&str], g: &[&str]) -> Vec<&'static str> {
        Scope::new(m, g).unwrap().filter(NODES)
    }

    // frob:tests crates/cadetta-cli/src/scope.rs::tests::module_prefix_scopes_nodes
    #[test]
    fn module_prefix_scopes_nodes() {
        assert_eq!(
            scope(&["drive"], &[]),
            vec![NODES[0], NODES[1]],
            "segment-wise, not string prefix"
        );
        assert_eq!(scope(&["drive/motor"], &[]), vec![NODES[0]]);
        assert_eq!(scope(&["acme.rover::drive::motor"], &[]), vec![NODES[0]]);
        assert_eq!(
            scope(&["frame", "drive::motor"], &[]),
            vec![NODES[0], NODES[3]]
        );
    }

    // frob:tests crates/cadetta-cli/src/scope.rs::tests::globs_scope_nodes
    #[test]
    fn globs_scope_nodes() {
        assert_eq!(scope(&[], &["acme.rover::Axle.*"]), vec![NODES[4]]);
        assert_eq!(scope(&[], &["acme.rover::*"]), Vec::<&str>::new());
        assert_eq!(
            scope(&[], &["acme.rover::drive::**"]),
            vec![NODES[0], NODES[1]]
        );
        assert_eq!(scope(&[], &["**.site[*].offset"]), vec![NODES[3]]);
        assert_eq!(
            scope(&[], &["**::Gear*.ratio", "**.press"]),
            vec![NODES[1], NODES[4]]
        );
    }

    // frob:tests crates/cadetta-cli/src/scope.rs::tests::module_and_glob_intersect
    #[test]
    fn module_and_glob_intersect() {
        assert_eq!(scope(&["drive"], &["**.ratio"]), vec![NODES[1]]);
        assert!(Scope::new::<&str, &str>(&[], &[]).unwrap().is_all());
        assert_eq!(scope(&[], &[]).len(), NODES.len());
    }

    // frob:tests crates/cadetta-cli/src/scope.rs::tests::malformed_terms_are_errors
    #[test]
    fn malformed_terms_are_errors() {
        assert_eq!(Glob::new(" "), Err(ScopeError::Empty { what: "glob" }));
        assert!(matches!(
            Glob::new("a.b[2"),
            Err(ScopeError::Unterminated(_))
        ));
        assert_eq!(
            Scope::new(&[""], &[] as &[&str]),
            Err(ScopeError::Empty { what: "module" })
        );
    }
}
