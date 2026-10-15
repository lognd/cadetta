//! Canonical paths: the identity of a node across source, IR, bindings, waivers and diagnostics.
//!
//! Grammar and rules: `docs/spec/ir.md` 2. Only the full form (package and item prefix) is
//! parsed here; the abbreviated in-source form is resolved by sema before a path is built.
// frob:ticket 01M4SWK9D2DYTDN1TW3013FBDV

use std::fmt;
use std::str::FromStr;

/// Where a path is being parsed: `[*]` is accepted only in bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathContext {
    /// A binding target (`Rover/axle[*].end_diameter` is allowed).
    Binding,
    /// Anywhere else (IR keys, waivers, diagnostics, expressions): `[*]` is rejected.
    General,
}

/// A path parse or rule failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    /// The text does not match the path grammar.
    #[error("malformed path `{path}`: {reason}")]
    Malformed {
        /// The offending text.
        path: String,
        /// What was wrong.
        reason: String,
    },
    /// `[*]` outside a binding (ir.md 2).
    #[error("`[*]` is only allowed in bindings: `{0}`")]
    StarOutsideBinding(String),
    /// A definition path was read for a parameter-dependent member (E011).
    #[error("E011: `{0}` is a definition path of a parameterized component; use an instance path")]
    E011(String),
}

/// An index: a literal, a range variable (`feet[i]`), or `*` (bindings only).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Index {
    /// A literal non-negative integer.
    Int(u64),
    /// A for-range variable such as `i`.
    Var(String),
    /// Every instance (`[*]`); only valid in bindings.
    Star,
}

/// One `/part[index]` step from the component down to an instance or part.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Step {
    /// The part or slot name.
    pub name: String,
    /// Optional index on the part.
    pub index: Option<Index>,
}

/// One segment of the member tail.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Seg {
    /// A named member (`holes`, `offset`, `x`).
    Name(String),
    /// A bare integer segment (placement op index, `place.0.x`).
    Num(u64),
    /// A `[index]` on the preceding segment.
    Index(Index),
}

/// A parsed canonical path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    /// The dotted package name, lower.dotted.
    pub package: String,
    /// Module segments (snake_case) between package and item.
    pub modules: Vec<String>,
    /// The component, interface or table name.
    pub item: String,
    /// Instance steps; empty for a definition path.
    pub steps: Vec<Step>,
    /// The member tail; empty only for a slot path (`Type/part`, no member).
    pub members: Vec<Seg>,
}

fn bad(path: &str, reason: impl Into<String>) -> PathError {
    let reason = reason.into();
    tracing::debug!(path, %reason, "path: malformed");
    PathError::Malformed {
        path: path.to_owned(),
        reason,
    }
}

fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(f) if f.is_ascii_alphabetic() || f == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

fn is_snake(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(f) if f.is_ascii_lowercase())
        && c.all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '_')
}

fn parse_index(whole: &str, text: &str, ctx: PathContext) -> Result<Index, PathError> {
    if text == "*" {
        return match ctx {
            PathContext::Binding => Ok(Index::Star),
            PathContext::General => {
                tracing::debug!(path = whole, "path: [*] rejected outside bindings");
                Err(PathError::StarOutsideBinding(whole.to_owned()))
            }
        };
    }
    if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
        return text
            .parse()
            .map(Index::Int)
            .map_err(|_| bad(whole, "index out of range"));
    }
    if is_ident(text) {
        return Ok(Index::Var(text.to_owned()));
    }
    Err(bad(whole, format!("bad index `{text}`")))
}

/// Split `name[idx]` into the name and optional index text.
fn split_index<'a>(whole: &str, seg: &'a str) -> Result<(&'a str, Option<&'a str>), PathError> {
    match seg.find('[') {
        None => Ok((seg, None)),
        Some(i) => {
            let inner = seg[i + 1..]
                .strip_suffix(']')
                .ok_or_else(|| bad(whole, format!("unclosed `[` in `{seg}`")))?;
            Ok((&seg[..i], Some(inner)))
        }
    }
}

impl Path {
    /// Parse a canonical path, accepting `[*]` only when `ctx` is [`PathContext::Binding`].
    pub fn parse(text: &str, ctx: PathContext) -> Result<Path, PathError> {
        let (prefix, rest) = text
            .split_once("::")
            .ok_or_else(|| bad(text, "missing `::`"))?;
        if prefix.is_empty() || !prefix.split('.').all(is_snake) {
            return Err(bad(text, "package must be lower.dotted"));
        }
        let mut parts: Vec<&str> = rest.split("::").collect();
        let tail = parts.pop().expect("split yields at least one piece");
        for m in &parts {
            if !is_snake(m) {
                return Err(bad(text, format!("module `{m}` must be snake_case")));
            }
        }
        // tail = Type [ "/" part[idx] ]* "." member...
        // A slot path (`Type/part[idx]`, a `Part.path`) names the instance itself, so it has no
        // member tail; any other path must carry one.
        let (head, mem) = match tail.find('.') {
            Some(dot) => (&tail[..dot], Some(&tail[dot + 1..])),
            None if tail.contains('/') => (tail, None),
            None => return Err(bad(text, "missing `.member`")),
        };
        let mut hs = head.split('/');
        let item = hs.next().unwrap_or_default();
        if !is_ident(item) || !item.starts_with(|c: char| c.is_ascii_uppercase()) {
            return Err(bad(text, format!("item `{item}` must start uppercase")));
        }
        let mut steps = Vec::new();
        for s in hs {
            let (name, idx) = split_index(text, s)?;
            if !is_ident(name) {
                return Err(bad(text, format!("bad part name `{name}`")));
            }
            let index = idx.map(|i| parse_index(text, i, ctx)).transpose()?;
            steps.push(Step {
                name: name.to_owned(),
                index,
            });
        }
        let members = mem.map_or(Ok(Vec::new()), |m| parse_members(text, m, ctx))?;
        tracing::trace!(path = text, "path: parsed");
        Ok(Path {
            package: prefix.to_owned(),
            modules: parts.into_iter().map(str::to_owned).collect(),
            item: item.to_owned(),
            steps,
            members,
        })
    }

    /// Whether this names a declaration (no instance steps) rather than an instance value.
    pub fn is_definition(&self) -> bool {
        self.steps.is_empty()
    }

    /// Enforce the instance-only rule: reading a parameter-dependent member of a parameterized
    /// component by definition path is E011 (ir.md 2; language.md 3.1).
    pub fn check_definition_read(&self, parameterized_dependent: bool) -> Result<(), PathError> {
        if self.is_definition() && parameterized_dependent {
            tracing::warn!(path = %self, "path: E011 definition read of parameter-dependent member");
            return Err(PathError::E011(self.to_string()));
        }
        Ok(())
    }

    /// The path of the first member segment's name, e.g. `holes` of `Bracket.holes[1].x`.
    pub fn member_root(&self) -> Option<&str> {
        match self.members.first() {
            Some(Seg::Name(n)) => Some(n),
            _ => None,
        }
    }
}

fn parse_members(whole: &str, mem: &str, ctx: PathContext) -> Result<Vec<Seg>, PathError> {
    let mut out = Vec::new();
    for seg in mem.split('.') {
        let (name, idx) = split_index(whole, seg)?;
        if name.bytes().all(|b| b.is_ascii_digit()) && !name.is_empty() {
            let n = name
                .parse()
                .map_err(|_| bad(whole, "number out of range"))?;
            out.push(Seg::Num(n));
        } else if is_ident(name) {
            out.push(Seg::Name(name.to_owned()));
        } else {
            return Err(bad(whole, format!("bad member segment `{seg}`")));
        }
        // Chained indexes `a[1][2]` are not in the grammar; one index per segment.
        if let Some(i) = idx {
            out.push(Seg::Index(parse_index(whole, i, ctx)?));
        }
    }
    Ok(out)
}

impl FromStr for Path {
    type Err = PathError;
    /// Parse in the general (non-binding) context.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Path::parse(s, PathContext::General)
    }
}

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Index::Int(n) => write!(f, "{n}"),
            Index::Var(v) => f.write_str(v),
            Index::Star => f.write_str("*"),
        }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.package)?;
        for m in &self.modules {
            write!(f, "::{m}")?;
        }
        write!(f, "::{}", self.item)?;
        for s in &self.steps {
            write!(f, "/{}", s.name)?;
            if let Some(i) = &s.index {
                write!(f, "[{i}]")?;
            }
        }
        for seg in &self.members {
            match seg {
                Seg::Index(i) => write!(f, "[{i}]")?,
                Seg::Name(s) => write!(f, ".{s}")?,
                Seg::Num(s) => write!(f, ".{s}")?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(s: &str) -> Path {
        let p = Path::parse(s, PathContext::Binding).unwrap();
        assert_eq!(p.to_string(), s);
        p
    }

    // frob:tests crates/cadetta-ir/src/path.rs::Path.parse
    #[test]
    fn indexed_and_placement_paths_round_trip() {
        rt("acme.rover::Bracket.holes[1].x");
        rt("acme.rover::Rover/wheel[2].hub_bore");
        let p = rt("acme.rover::Rover/feet[i].place.0.x");
        assert_eq!(p.steps[0].index, Some(Index::Var("i".into())));
        assert_eq!(
            p.members,
            vec![
                Seg::Name("place".into()),
                Seg::Num(0),
                Seg::Name("x".into())
            ]
        );
        rt("acme.rover::Rover/bracket.place.0.x");
    }

    // frob:tests crates/cadetta-ir/src/path.rs::Path.parse
    #[test]
    fn star_only_in_bindings() {
        let s = "acme.rover::Rover/axle[*].end_diameter";
        assert_eq!(rt(s).steps[0].index, Some(Index::Star));
        assert!(matches!(
            s.parse::<Path>(),
            Err(PathError::StarOutsideBinding(_))
        ));
        let m = "acme.rover::Bracket.holes[*].x";
        assert!(Path::parse(m, PathContext::General).is_err());
        assert!(Path::parse(m, PathContext::Binding).is_ok());
    }

    // frob:tests crates/cadetta-ir/src/path.rs::Path.parse
    #[test]
    fn datum_and_generator_members() {
        for s in [
            "acme.rover::Bracket.top.offset.x",
            "acme.rover::Bracket.top.rotate.z",
            "acme.rover::Bracket.holes.pitch_x",
            "acme.rover::Axle.seat.shaft",
        ] {
            let p = rt(s);
            assert!(p.is_definition());
            assert!(p.members.iter().all(|m| matches!(m, Seg::Name(_))));
        }
    }

    // frob:tests crates/cadetta-ir/src/path.rs::Path.check_definition_read
    #[test]
    fn definition_read_of_parameter_dependent_member_is_e011() {
        let d = rt("acme.rover::Axle.end_diameter");
        assert!(matches!(
            d.check_definition_read(true),
            Err(PathError::E011(_))
        ));
        assert!(d.check_definition_read(false).is_ok());
        let i = rt("acme.rover::Rover/axle[0].end_diameter");
        assert!(i.check_definition_read(true).is_ok());
    }

    // frob:ticket 01M4W47VTBXFMC70Q2R137W4EE
    // frob:tests crates/cadetta-ir/src/path.rs::Path.parse
    #[test]
    fn slot_path_has_no_member_but_a_bare_item_is_rejected() {
        let p = rt("acme.rover::Rover/wheel");
        assert_eq!(p.steps.len(), 1);
        assert!(p.members.is_empty());
        assert_eq!(p.to_string(), "acme.rover::Rover/wheel");
        assert_eq!(
            rt("acme.rover::Rover/wheel[2]").to_string(),
            "acme.rover::Rover/wheel[2]"
        );
        assert!("acme.rover::Rover".parse::<Path>().is_err());
    }

    // frob:tests crates/cadetta-ir/src/path.rs::Path.parse
    #[test]
    fn package_module_prefix() {
        let p = rt("acme.rover::drive::motor::Gm42A.rated_torque");
        assert_eq!(p.package, "acme.rover");
        assert_eq!(p.modules, vec!["drive", "motor"]);
        assert_eq!(p.item, "Gm42A");
        for bad in [
            "Acme.rover::A.x",
            "acme.rover::Drive::A.x",
            "acme.rover::drive-x::A.x",
            "acme.rover::a.x",
            "acme.rover::A",
            "acme::A.x[",
            ".x",
        ] {
            assert!(bad.parse::<Path>().is_err(), "{bad}");
        }
    }
}
