//! Import naming: components, members, collisions, expressions, file paths (adapters.md 7.2).
//!
//! Every function is pure and deterministic: the same native names in the same
//! order always give the same Cadetta names.
// frob:ticket 01M4SWMCQF9HTJKW2Y7JR3ZKRA

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path as FsPath;

use tracing::{debug, trace};

/// Trailing unit tokens stripped from native names (lower case).
const UNIT_SUFFIXES: &[&str] = &[
    "mm", "cm", "m", "um", "in", "ft", "deg", "rad", "kg", "g", "lb", "lbf", "n", "nm", "pa",
    "kpa", "mpa", "gpa", "psi", "ksi", "s", "ms", "hz", "rpm",
];

/// Directory of imported component files, relative to the package root.
pub const IMPORTED_DIR: &str = "cad/imported";

/// A failure rewriting a native expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingError {
    /// A quoted native name has no proposed member.
    UnknownReference(String),
    /// A quote was opened and never closed.
    UnterminatedQuote,
}

impl fmt::Display for NamingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownReference(n) => write!(f, "native name \"{n}\" has no proposed member"),
            Self::UnterminatedQuote => write!(f, "unterminated quote in native expression"),
        }
    }
}

impl std::error::Error for NamingError {}

/// Split a native name into lower-case words at separators and camel boundaries.
fn words(name: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev: Option<char> = None;
    let chars: Vec<char> = name.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_ascii_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev = None;
            continue;
        }
        let next_lower = chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase());
        let boundary = c.is_ascii_uppercase()
            && prev
                .is_some_and(|p| p.is_ascii_lowercase() || (p.is_ascii_uppercase() && next_lower));
        if boundary && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        cur.push(c.to_ascii_lowercase());
        prev = Some(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Component name: the file stem in UpperCamel (`cover_plate.SLDPRT` -> `CoverPlate`).
pub fn component_name(file: &str) -> String {
    let stem = FsPath::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file);
    let s: String = stem
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut cs = w.chars();
            let first = cs.next().map(|c| c.to_ascii_uppercase());
            first.into_iter().chain(cs).collect::<String>()
        })
        .collect();
    let name = if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("C{s}")
    } else {
        s
    };
    trace!(file, %name, "component name");
    name
}

/// Relative path of the generated component file (`cad/imported/CoverPlate.cdt`).
pub fn component_file(file: &str) -> String {
    format!("{IMPORTED_DIR}/{}.cdt", component_name(file))
}

/// Member name: snake_case with a trailing unit token stripped (`Bore_mm` -> `bore`).
pub fn member_name(native: &str) -> String {
    let mut ws = words(native);
    if ws.len() > 1
        && ws
            .last()
            .is_some_and(|w| UNIT_SUFFIXES.contains(&w.as_str()))
    {
        ws.pop();
    }
    let s = ws.join("_");
    if s.is_empty() {
        "item".to_string()
    } else if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("p_{s}")
    } else {
        s
    }
}

/// One allocated member name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allocated {
    /// The unique member name.
    pub name: String,
    /// The name collided and was suffixed; the proposal flags it.
    pub collided: bool,
}

/// Allocates unique member names within one component, suffixing `_2`, `_3`.
#[derive(Debug, Default)]
pub struct MemberNames {
    used: BTreeSet<String>,
}

impl MemberNames {
    /// Start with no names taken.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate the name for `native`, suffixing on collision with an earlier one.
    pub fn allocate(&mut self, native: &str) -> Allocated {
        let base = member_name(native);
        let mut name = base.clone();
        let mut n = 1u32;
        while self.used.contains(&name) {
            n += 1;
            name = format!("{base}_{n}");
        }
        let collided = n > 1;
        if collided {
            debug!(native, %name, "member name collision suffixed");
        }
        self.used.insert(name.clone());
        Allocated { name, collided }
    }
}

/// Rewrite a native expression: each `"Native Name"` becomes its proposed member
/// from `members` (native name -> member); the rest is kept verbatim.
pub fn rewrite_expression(
    expr: &str,
    members: &BTreeMap<String, String>,
) -> Result<String, NamingError> {
    let mut out = String::new();
    let mut rest = expr;
    while let Some(start) = rest.find('"') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after.find('"').ok_or(NamingError::UnterminatedQuote)?;
        let native = &after[..end];
        let member = members
            .get(native)
            .ok_or_else(|| NamingError::UnknownReference(native.to_string()))?;
        out.push_str(member);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_is_upper_camel_stem() {
        assert_eq!(component_name("cover_plate.SLDPRT"), "CoverPlate");
        assert_eq!(component_name("parts/3d-bracket.SLDPRT"), "C3dBracket");
        assert_eq!(
            component_file("cover_plate.SLDPRT"),
            "cad/imported/CoverPlate.cdt"
        );
    }

    #[test]
    fn member_snake_and_unit_strip() {
        assert_eq!(member_name("Plate_Thk"), "plate_thk");
        assert_eq!(member_name("Bore_mm"), "bore");
        assert_eq!(member_name("plate thickness"), "plate_thickness");
        assert_eq!(member_name("PlateThk"), "plate_thk");
        assert_eq!(member_name("mm"), "mm");
        assert_eq!(member_name("2x_hole"), "p_2x_hole");
    }

    #[test]
    fn collisions_are_suffixed_and_flagged() {
        let mut m = MemberNames::new();
        let a = m.allocate("Bore_mm");
        let b = m.allocate("bore");
        let c = m.allocate("BORE");
        assert_eq!((a.name.as_str(), a.collided), ("bore", false));
        assert_eq!((b.name.as_str(), b.collided), ("bore_2", true));
        assert_eq!((c.name.as_str(), c.collided), ("bore_3", true));
    }

    #[test]
    fn expression_references_proposed_members() {
        let mut map = BTreeMap::new();
        map.insert("A".to_string(), "a".to_string());
        map.insert("B".to_string(), "b".to_string());
        assert_eq!(rewrite_expression("\"B\" * 2", &map).unwrap(), "b * 2");
        assert_eq!(
            rewrite_expression("\"Z\" * 2", &map),
            Err(NamingError::UnknownReference("Z".into()))
        );
        assert_eq!(
            rewrite_expression("\"B * 2", &map),
            Err(NamingError::UnterminatedQuote)
        );
    }

    #[test]
    fn deterministic() {
        let run = || {
            let mut m = MemberNames::new();
            ["X_mm", "x", "Y"].map(|n| m.allocate(n).name)
        };
        assert_eq!(run(), run());
    }
}
