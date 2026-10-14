//! The `PKG::module::Item/part[i].member[j]` canonical path as sema builds it (ir.md 2).
//!
//! Shared by sema and the geometry crate so neither depends on the other for
//! path identity; sema re-exports these from `resolve`.
// frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC

use std::fmt;

/// A numeric index or the `[*]` wildcard of a binding path (ir.md 2).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Index {
    /// `[2]`
    At(u64),
    /// `[*]`, valid only in bindings.
    Any,
}

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Index::At(n) => write!(f, "{n}"),
            Index::Any => f.write_str("*"),
        }
    }
}

/// One `/part[index]` segment of an instance path.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PartSeg {
    /// The slot name.
    pub name: String,
    /// The instance index, absent for a scalar slot or the whole array.
    pub index: Option<Index>,
}

/// One segment after the item: `.name` or `[index]`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum MemberSeg {
    /// `.name`
    Name(String),
    /// `[index]`
    Index(Index),
}

/// A canonical path `PKG::module::Item/part[i].member[j]` (ir.md 2).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct CanonPath {
    /// The dotted package name.
    pub package: String,
    /// The module path below the package root (empty for `lib.cdt`).
    pub modules: Vec<String>,
    /// The item (type) name.
    pub item: String,
    /// The `/part[index]` instance segments.
    pub parts: Vec<PartSeg>,
    /// The member chain.
    pub members: Vec<MemberSeg>,
}

impl CanonPath {
    /// The path of an item with no parts or members.
    pub fn item(package: &str, modules: &[String], item: &str) -> Self {
        Self {
            package: package.to_owned(),
            modules: modules.to_vec(),
            item: item.to_owned(),
            parts: Vec::new(),
            members: Vec::new(),
        }
    }

    /// This path with the parts and members dropped.
    pub fn item_only(&self) -> Self {
        Self::item(&self.package, &self.modules, &self.item)
    }

    /// Parses ir.md 2 text such as `acme.rover::Rover/wheel[2].hub_bore`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let bad = |why: &str| format!("canonical path `{text}`: {why}");
        let (package, mut rest) = text.split_once("::").ok_or_else(|| bad("missing `::`"))?;
        if package.is_empty() || !package.split('.').all(is_ident) {
            return Err(bad("bad package name"));
        }
        let mut idents = Vec::new();
        loop {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            let (id, after) = rest.split_at(end);
            if !is_ident(id) {
                return Err(bad("bad module or item name"));
            }
            idents.push(id.to_owned());
            match after.strip_prefix("::") {
                Some(r) => rest = r,
                None => {
                    rest = after;
                    break;
                }
            }
        }
        let item = idents.pop().unwrap_or_default();
        let mut out = Self::item(package, &idents, &item);
        while let Some(r) = rest.strip_prefix('/') {
            let (name, r) = take_word(r);
            if !is_ident(name) {
                return Err(bad("bad part name"));
            }
            let (index, r) = take_index(r).map_err(|e| bad(&e))?;
            out.parts.push(PartSeg {
                name: name.to_owned(),
                index,
            });
            rest = r;
        }
        while !rest.is_empty() {
            if let Some(r) = rest.strip_prefix('.') {
                let (name, r) = take_word(r);
                if name.is_empty() {
                    return Err(bad("empty member"));
                }
                out.members.push(MemberSeg::Name(name.to_owned()));
                rest = r;
            } else if rest.starts_with('[') {
                let (index, r) = take_index(rest).map_err(|e| bad(&e))?;
                out.members
                    .push(MemberSeg::Index(index.ok_or_else(|| bad("empty index"))?));
                rest = r;
            } else {
                return Err(bad("unexpected text"));
            }
        }
        Ok(out)
    }
}

fn is_ident(s: &str) -> bool {
    let mut cs = s.chars();
    matches!(cs.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn take_word(s: &str) -> (&str, &str) {
    let end = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    s.split_at(end)
}

fn take_index(s: &str) -> Result<(Option<Index>, &str), String> {
    let Some(r) = s.strip_prefix('[') else {
        return Ok((None, s));
    };
    let (body, r) = r.split_once(']').ok_or("unclosed `[`")?;
    let index = if body == "*" {
        Index::Any
    } else {
        Index::At(body.parse().map_err(|_| format!("bad index `{body}`"))?)
    };
    Ok((Some(index), r))
}

impl fmt::Display for CanonPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::", self.package)?;
        for m in &self.modules {
            write!(f, "{m}::")?;
        }
        f.write_str(&self.item)?;
        for p in &self.parts {
            write!(f, "/{}", p.name)?;
            if let Some(i) = p.index {
                write!(f, "[{i}]")?;
            }
        }
        for m in &self.members {
            match m {
                MemberSeg::Name(n) => write!(f, ".{n}")?,
                MemberSeg::Index(i) => write!(f, "[{i}]")?,
            }
        }
        Ok(())
    }
}
