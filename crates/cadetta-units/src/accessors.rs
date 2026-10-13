// frob:ticket 01M4ST7A42Q225NZMARTM5V6J5
//! `.nominal .min .max .mid .mmc .lmc` on any quantity (quantities.md 5.2).
//!
//! Every accessor returns an untoleranced [`Plain`] value. An untoleranced
//! quantity is a [`Toleranced`] with zero deviations, so all accessors equal
//! the value. `.mmc`/`.lmc` need a role (5.6) and fail with E010 without one.

use tracing::{debug, trace};

use crate::dimension::Dim;
use crate::rational::{Magnitude, Rational};
use crate::role::{Role, RoleError, RoleInfo};
use crate::tolerance::{ToleranceError, Toleranced};

/// An untoleranced exact quantity: SI-base magnitude and dimension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plain {
    /// The dimension.
    pub dim: Dim,
    /// The exact SI-base magnitude.
    pub value: Magnitude,
}

/// Errors from reading an accessor.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AccessorError {
    /// `.mmc` or `.lmc` without a role (E010).
    #[error(transparent)]
    Role(#[from] RoleError),
    /// An underlying tolerance computation failed.
    #[error(transparent)]
    Tolerance(#[from] ToleranceError),
}

/// A quantity as the accessors see it: limits plus an optional role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quantity {
    tol: Toleranced,
    role: Option<RoleInfo>,
}

impl Quantity {
    /// An untoleranced quantity: every accessor returns `value`.
    pub fn untoleranced(dim: Dim, value: Magnitude) -> Result<Self, ToleranceError> {
        let zero = || (dim.clone(), Magnitude::new(Rational::from_int(0), 0));
        let tol = Toleranced::from_deviations(dim.clone(), value, zero(), zero())?;
        Ok(Self { tol, role: None })
    }

    /// A toleranced quantity without a role.
    pub fn toleranced(tol: Toleranced) -> Self {
        Self { tol, role: None }
    }

    /// Give the quantity a role (replaces any previous one).
    pub fn with_role(mut self, role: RoleInfo) -> Self {
        trace!(role = ?role.role, source = ?role.source, "accessors: role set");
        self.role = Some(role);
        self
    }

    /// The quantity's role, if any.
    pub fn role(&self) -> Option<RoleInfo> {
        self.role
    }

    fn plain(&self, value: Magnitude) -> Plain {
        Plain {
            dim: self.tol.dim().clone(),
            value,
        }
    }

    /// `.nominal`
    pub fn nominal(&self) -> Plain {
        self.plain(self.tol.nominal().clone())
    }

    /// `.min`
    pub fn min(&self) -> Result<Plain, AccessorError> {
        Ok(self.plain(self.tol.min()?))
    }

    /// `.max`
    pub fn max(&self) -> Result<Plain, AccessorError> {
        Ok(self.plain(self.tol.max()?))
    }

    /// `.mid`
    pub fn mid(&self) -> Result<Plain, AccessorError> {
        Ok(self.plain(self.tol.mid()?))
    }

    fn require_role(&self) -> Result<Role, AccessorError> {
        match self.role {
            Some(r) => Ok(r.role),
            None => {
                debug!("accessors: mmc/lmc without role (E010)");
                Err(RoleError::RoleRequired.into())
            }
        }
    }

    /// `.mmc`: the smallest size of a hole, the largest of a shaft.
    pub fn mmc(&self) -> Result<Plain, AccessorError> {
        match self.require_role()? {
            Role::Hole => self.min(),
            Role::Shaft => self.max(),
        }
    }

    /// `.lmc`: the largest size of a hole, the smallest of a shaft.
    pub fn lmc(&self) -> Result<Plain, AccessorError> {
        match self.require_role()? {
            Role::Hole => self.max(),
            Role::Shaft => self.min(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::Dimension;
    use crate::role::RoleSource;

    fn len() -> Dim {
        Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
    }

    fn mm(n: i64, d: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, d).unwrap(), 0)
    }

    fn h7() -> Toleranced {
        // 12 mm [+0.018 -0] in metres
        Toleranced::from_deviations(
            len(),
            mm(12, 1000),
            (len(), mm(0, 1)),
            (len(), mm(18, 1_000_000)),
        )
        .unwrap()
    }

    fn info(role: Role) -> RoleInfo {
        RoleInfo {
            role,
            source: RoleSource::Attribute,
        }
    }

    // frob:tests crates/cadetta-units/src/accessors.rs::Quantity.untoleranced kind=unit
    #[test]
    fn untoleranced_accessors_all_equal_value() {
        let q = Quantity::untoleranced(len(), mm(5, 1000)).unwrap();
        let v = mm(5, 1000);
        assert_eq!(q.nominal().value, v);
        assert_eq!(q.min().unwrap().value, v);
        assert_eq!(q.max().unwrap().value, v);
        assert_eq!(q.mid().unwrap().value, v);
    }

    // frob:tests crates/cadetta-units/src/accessors.rs::Quantity.mid kind=unit
    #[test]
    fn toleranced_accessors_are_plain() {
        let q = Quantity::toleranced(h7());
        assert_eq!(q.nominal().value, mm(12, 1000));
        assert_eq!(q.min().unwrap().value, mm(12, 1000));
        assert_eq!(q.max().unwrap().value, mm(12_018, 1_000_000));
        assert_eq!(q.mid().unwrap().value, mm(12_009, 1_000_000));
        assert_eq!(q.max().unwrap().dim, len());
    }

    // frob:tests crates/cadetta-units/src/accessors.rs::Quantity.mmc kind=unit
    #[test]
    fn mmc_lmc_follow_role() {
        let hole = Quantity::toleranced(h7()).with_role(info(Role::Hole));
        assert_eq!(hole.mmc().unwrap(), hole.min().unwrap());
        assert_eq!(hole.lmc().unwrap(), hole.max().unwrap());
        let shaft = Quantity::toleranced(h7()).with_role(info(Role::Shaft));
        assert_eq!(shaft.mmc().unwrap(), shaft.max().unwrap());
        assert_eq!(shaft.lmc().unwrap(), shaft.min().unwrap());
    }

    // frob:tests crates/cadetta-units/src/accessors.rs::Quantity.lmc kind=unit
    #[test]
    fn mmc_lmc_without_role_is_e010() {
        let q = Quantity::toleranced(h7());
        assert_eq!(q.mmc(), Err(AccessorError::Role(RoleError::RoleRequired)));
        assert_eq!(q.lmc(), Err(AccessorError::Role(RoleError::RoleRequired)));
    }
}
