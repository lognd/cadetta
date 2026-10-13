// frob:ticket 01M4ST7A42Q225NZMARTM5V6J5
//! Hole and shaft roles (quantities.md 5.6).
//!
//! A role comes from an ISO class letter case, the side a parameter plays in
//! a `fit`, or `#[role(hole)]`; a pure reference inherits its referent's role.

use thiserror::Error;
use tracing::{debug, trace};

/// Which side of a mating pair a toleranced value plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Internal feature: maximum material is the smallest size.
    Hole,
    /// External feature: maximum material is the largest size.
    Shaft,
}

/// Where a role was taken from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleSource {
    /// The case of an ISO 286 class letter (`H7` hole, `g6` shaft).
    IsoClass,
    /// The side a parameter plays in a `fit`.
    FitSide,
    /// An explicit `#[role(..)]` attribute.
    Attribute,
    /// Inherited through a pure reference.
    Inherited,
}

/// Role errors; `RoleRequired` is lint E010.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RoleError {
    /// `.mmc` or `.lmc` was read on a value with no role (E010).
    #[error("E010: mmc/lmc needs a role (hole or shaft)")]
    RoleRequired,
    /// The class text does not start with an ISO 286 letter.
    #[error("not an ISO 286 class letter: {0:?}")]
    BadClassLetter(char),
    /// The `#[role(..)]` argument is neither `hole` nor `shaft`.
    #[error("unknown role {0:?}; expected hole or shaft")]
    UnknownRole(String),
}

impl Role {
    /// The role named by a `#[role(..)]` attribute argument.
    pub fn from_attribute(arg: &str) -> Result<Self, RoleError> {
        match arg {
            "hole" => Ok(Self::Hole),
            "shaft" => Ok(Self::Shaft),
            other => {
                debug!(arg = other, "role: unknown attribute argument");
                Err(RoleError::UnknownRole(other.to_string()))
            }
        }
    }

    /// The role of an ISO 286 class letter: uppercase hole, lowercase shaft.
    pub fn from_class_letter(letter: char) -> Result<Self, RoleError> {
        if letter.is_ascii_uppercase() {
            Ok(Self::Hole)
        } else if letter.is_ascii_lowercase() {
            Ok(Self::Shaft)
        } else {
            debug!(%letter, "role: not a class letter");
            Err(RoleError::BadClassLetter(letter))
        }
    }

    /// The opposite role (the mating side of a fit).
    pub fn mate(self) -> Self {
        match self {
            Self::Hole => Self::Shaft,
            Self::Shaft => Self::Hole,
        }
    }
}

/// A role together with where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoleInfo {
    /// The role.
    pub role: Role,
    /// Its provenance.
    pub source: RoleSource,
}

/// Resolve a parameter's role: its own wins; a pure reference inherits.
///
/// `own` is the role the parameter states or derives itself; `referent` is
/// the role of the value it purely refers to (quantities.md 5.6).
pub fn resolve_role(own: Option<RoleInfo>, referent: Option<RoleInfo>) -> Option<RoleInfo> {
    match (own, referent) {
        (Some(o), _) => Some(o),
        (None, Some(r)) => {
            trace!(role = ?r.role, "role: inherited from referent");
            Some(RoleInfo {
                role: r.role,
                source: RoleSource::Inherited,
            })
        }
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests crates/cadetta-units/src/role.rs::Role.from_class_letter kind=unit
    #[test]
    fn class_letter_case_gives_role() {
        assert_eq!(Role::from_class_letter('H'), Ok(Role::Hole));
        assert_eq!(Role::from_class_letter('g'), Ok(Role::Shaft));
        assert!(Role::from_class_letter('7').is_err());
    }

    // frob:tests crates/cadetta-units/src/role.rs::Role.from_attribute kind=unit
    #[test]
    fn attribute_gives_role() {
        assert_eq!(Role::from_attribute("hole"), Ok(Role::Hole));
        assert_eq!(Role::from_attribute("shaft"), Ok(Role::Shaft));
        assert!(Role::from_attribute("boss").is_err());
        assert_eq!(Role::Hole.mate(), Role::Shaft);
    }

    // frob:tests crates/cadetta-units/src/role.rs::resolve_role kind=unit
    #[test]
    fn reference_inherits_referent_role() {
        let r = RoleInfo {
            role: Role::Hole,
            source: RoleSource::FitSide,
        };
        let got = resolve_role(None, Some(r)).unwrap();
        assert_eq!(got.role, Role::Hole);
        assert_eq!(got.source, RoleSource::Inherited);
        assert_eq!(resolve_role(None, None), None);
        let own = RoleInfo {
            role: Role::Shaft,
            source: RoleSource::Attribute,
        };
        assert_eq!(resolve_role(Some(own), Some(r)), Some(own));
    }
}
