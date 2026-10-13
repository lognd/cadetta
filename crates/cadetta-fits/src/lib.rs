//! ISO 286 fit tables and fit evaluation.
//!
//! Specification: `docs/spec/language.md section 3.7` and
//! `docs/spec/quantities.md section 5.1`.
//!
//! Every value is an exact integer count of nanometres (no floating
//! point): ISO 286-1 tabulates IT1..IT4 to 0.1 um, so nanometres are the
//! coarsest integer unit that holds every tabulated figure and `js`
//! half-widths exactly.
//!
//! Coverage (see `language.md` 3.7 "Tabulated classes"): nominal sizes
//! over 0 up to and including 500 mm; grades IT1..IT18; shaft letters
//! `c d e f g h js j k m n p` and hole letters `C D E F G H JS`. Any
//! other class is [`FitError::UnknownClass`] (F001) until its table is
//! added.

use tracing::{debug, trace, warn};

mod tables;

/// Nanometres per micrometre.
const NM_PER_UM: i64 = 1_000;
/// Nanometres per millimetre.
const NM_PER_MM: i64 = 1_000_000;

/// Which side of a fit a class describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Uppercase class letter: an internal feature (bore, slot).
    Hole,
    /// Lowercase class letter: an external feature (shaft, pin).
    Shaft,
}

/// A parsed ISO 286 class such as `H7` or `j5`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    /// Hole or shaft, from the letter case.
    pub side: Side,
    /// Fundamental-deviation letter, lowercased (`"h"`, `"js"`).
    pub letter: String,
    /// IT grade, 1..=18.
    pub grade: u8,
}

/// A resolved tolerance: deviations from the nominal, in nanometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolved {
    /// Hole or shaft.
    pub side: Side,
    /// Lower deviation (EI for holes, ei for shafts), nm.
    pub lower_nm: i64,
    /// Upper deviation (ES for holes, es for shafts), nm.
    pub upper_nm: i64,
}

/// Fit-resolution failures; each maps to a lint code in `lints.md`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FitError {
    /// F001: the class is not in the ISO 286 tables.
    #[error("F001: unknown ISO 286 class `{class}`")]
    UnknownClass {
        /// The class text as written.
        class: String,
    },
    /// F002: the nominal size is outside the table's range.
    #[error("F002: size {size_nm} nm is outside the ISO 286 table range (0 mm, 500 mm]")]
    SizeOutOfRange {
        /// The nominal size, nm.
        size_nm: i64,
    },
}

impl FitError {
    /// The lint code (`F001`, `F002`) this error is reported as.
    pub fn code(&self) -> &'static str {
        match self {
            FitError::UnknownClass { .. } => "F001",
            FitError::SizeOutOfRange { .. } => "F002",
        }
    }
}

/// Parses class text (`H7`, `js6`) into a [`Class`]; letters of one case only.
pub fn parse_class(text: &str) -> Result<Class, FitError> {
    let unknown = || {
        warn!(class = text, "F001 unknown ISO 286 class");
        FitError::UnknownClass {
            class: text.to_owned(),
        }
    };
    let split = text
        .find(|c: char| c.is_ascii_digit())
        .ok_or_else(unknown)?;
    let (letters, digits) = text.split_at(split);
    if letters.is_empty() || !letters.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(unknown());
    }
    let side = if letters.chars().all(|c| c.is_ascii_uppercase()) {
        Side::Hole
    } else if letters.chars().all(|c| c.is_ascii_lowercase()) {
        Side::Shaft
    } else {
        return Err(unknown());
    };
    if digits.len() > 2 || !digits.chars().all(|c| c.is_ascii_digit()) || digits.starts_with('0') {
        return Err(unknown());
    }
    let grade: u8 = digits.parse().map_err(|_| unknown())?;
    if !(1..=18).contains(&grade) {
        return Err(unknown());
    }
    trace!(class = text, ?side, grade, "parsed ISO 286 class");
    Ok(Class {
        side,
        letter: letters.to_ascii_lowercase(),
        grade,
    })
}

/// Resolves `class` text on a nominal `size_nm` to its tolerance (ISO 286-1/-2).
pub fn resolve(size_nm: i64, class: &str) -> Result<Resolved, FitError> {
    let class = parse_class(class)?;
    let range = tables::fine_range(size_nm).ok_or_else(|| {
        warn!(size_nm, "F002 size outside ISO 286 table range");
        FitError::SizeOutOfRange { size_nm }
    })?;
    let it = tables::it_nm(range.coarse, class.grade);
    let letter = class.letter.as_str();
    let unknown = || {
        warn!(letter, grade = class.grade, "F001 class not tabulated");
        FitError::UnknownClass {
            class: format!("{}{}", letter, class.grade),
        }
    };
    let out = if letter == "js" {
        Resolved {
            side: class.side,
            lower_nm: -it / 2,
            upper_nm: it / 2,
        }
    } else {
        match class.side {
            Side::Hole => {
                // Hole A..H: EI is the mirror of the shaft's upper deviation es.
                if !"cdefgh".contains(letter) {
                    return Err(unknown());
                }
                let es = tables::shaft_upper_um(letter, range).ok_or_else(unknown)? * NM_PER_UM;
                Resolved {
                    side: Side::Hole,
                    lower_nm: -es,
                    upper_nm: -es + it,
                }
            }
            Side::Shaft => {
                if "cdefgh".contains(letter) {
                    let es = tables::shaft_upper_um(letter, range).ok_or_else(unknown)? * NM_PER_UM;
                    Resolved {
                        side: Side::Shaft,
                        lower_nm: es - it,
                        upper_nm: es,
                    }
                } else {
                    let ei = tables::shaft_lower_um(letter, class.grade, range)
                        .ok_or_else(unknown)?
                        * NM_PER_UM;
                    Resolved {
                        side: Side::Shaft,
                        lower_nm: ei,
                        upper_nm: ei + it,
                    }
                }
            }
        }
    };
    debug!(size_nm, ?out, "resolved ISO 286 class");
    Ok(out)
}

/// Convenience: resolves `class` on a nominal given in whole millimetres.
pub fn resolve_mm(size_mm: i64, class: &str) -> Result<Resolved, FitError> {
    resolve(size_mm.saturating_mul(NM_PER_MM), class)
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests crates/cadetta-fits/src/lib.rs::resolve kind=unit
    // frob:ticket V5G43BT
    #[test]
    fn h7_20mm() {
        let r = resolve_mm(20, "H7").unwrap();
        assert_eq!((r.lower_nm, r.upper_nm), (0, 21_000));
        assert_eq!(r.side, Side::Hole);
    }

    // frob:tests crates/cadetta-fits/src/lib.rs::resolve kind=unit
    // frob:ticket V5G43BT
    #[test]
    fn out_of_range_is_f002() {
        for size in [0, -5, 500 * NM_PER_MM + 1, 600 * NM_PER_MM] {
            let e = resolve(size, "H7").unwrap_err();
            assert_eq!(e.code(), "F002");
        }
        assert!(resolve_mm(500, "H7").is_ok());
    }

    // frob:tests crates/cadetta-fits/src/lib.rs::resolve kind=unit
    // frob:ticket V5G43BT
    #[test]
    fn unknown_class_is_f001() {
        for c in ["Z99", "H0", "H19", "H", "7", "Hh7", "H7x", "ZC7", "K7"] {
            let e = resolve_mm(20, c).unwrap_err();
            assert_eq!(e.code(), "F001", "{c}");
        }
    }

    // frob:tests crates/cadetta-fits/src/lib.rs::resolve kind=unit
    // frob:ticket V5G43BT
    #[test]
    fn case_picks_side() {
        assert_eq!(resolve_mm(20, "H7").unwrap().side, Side::Hole);
        assert_eq!(resolve_mm(20, "h7").unwrap().side, Side::Shaft);
        assert_eq!(resolve_mm(20, "JS7").unwrap().side, Side::Hole);
        assert_eq!(resolve_mm(20, "js7").unwrap().side, Side::Shaft);
        let g = resolve_mm(20, "g6").unwrap();
        assert_eq!((g.lower_nm, g.upper_nm), (-20_000, -7_000));
        let big_g = resolve_mm(20, "G6").unwrap();
        assert_eq!((big_g.lower_nm, big_g.upper_nm), (7_000, 20_000));
    }

    // frob:tests crates/cadetta-fits/src/lib.rs::resolve kind=unit
    // frob:ticket V5G43BT
    #[test]
    fn class_on_any_expression_value() {
        // 8 mm j5 (golden 02): +4 um / -2 um, the nominal arriving as a computed size.
        let size_nm = 4 * NM_PER_MM + 4 * NM_PER_MM;
        let r = resolve(size_nm, "j5").unwrap();
        assert_eq!((r.lower_nm, r.upper_nm), (-2_000, 4_000));
    }

    // frob:tests crates/cadetta-fits/src/lib.rs::resolve kind=unit
    #[test]
    fn golden_values() {
        let t = |mm, c: &str| {
            let r = resolve_mm(mm, c).unwrap();
            (r.lower_nm / NM_PER_UM, r.upper_nm / NM_PER_UM)
        };
        assert_eq!(t(8, "g6"), (-14, -5));
        assert_eq!(t(10, "c11"), (-170, -80));
        assert_eq!(t(10, "H11"), (0, 90));
        assert_eq!(t(22, "H8"), (0, 33));
        assert_eq!(t(5, "H7"), (0, 12));
        assert_eq!(t(11, "h11"), (-110, 0));
        assert_eq!(t(20, "h11"), (-130, 0));
        assert_eq!(t(20, "p6"), (22, 35));
        assert_eq!(t(20, "k6"), (2, 15));
    }

    #[test]
    fn size_range_edges() {
        // (3, 6] mm: exactly 3 mm belongs to the first range, 6 mm to the second.
        assert_eq!(resolve_mm(3, "H7").unwrap().upper_nm, 10_000);
        assert_eq!(resolve(3 * NM_PER_MM + 1, "H7").unwrap().upper_nm, 12_000);
        assert_eq!(resolve_mm(6, "H7").unwrap().upper_nm, 12_000);
    }
}
