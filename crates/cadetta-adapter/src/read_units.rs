//! Unit assumption when reading native values (adapters.md 3).
//!
//! A native value without a unit takes `--assume-unit` when the run gives one, else the
//! locator's or adapter's native unit, and is recorded as `unit_assumed`. With no unit
//! information at all the value is read inexact with a reason. Read never raises
//! Q001: that code is a source error, not a read diagnostic.

/// The unit a read value ended up with, and how it was chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitResolution {
    /// The native value carried its own unit.
    Explicit(String),
    /// The unit was assumed (adapter native unit or `--assume-unit`).
    Assumed(String),
    /// No unit information at all; the value is inexact for the stated reason.
    Unknown {
        /// Why the value is not exact.
        reason: String,
    },
}

/// A native value as read, before unit resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeValue {
    /// Source literal of the number.
    pub literal: String,
    /// Unit the file attached to the value, if any.
    pub unit: Option<String>,
}

/// An observed-IR value after unit resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedValue {
    /// Source literal of the number.
    pub literal: String,
    /// Resolved unit, when one exists.
    pub unit: Option<String>,
    /// True when the unit was assumed (`unit_assumed` in observed IR).
    pub unit_assumed: bool,
    /// False when the value cannot be taken as exact.
    pub exact: bool,
    /// Reason the value is inexact, when `exact` is false.
    pub reason: Option<String>,
}

/// Where assumed units come from for one read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnitPolicy {
    /// Native unit declared by the adapter at `initialize`, if any.
    pub native_unit: Option<String>,
    /// Unit from `--assume-unit`, if given.
    pub assume_unit: Option<String>,
}

impl UnitPolicy {
    /// Resolves a unit for a value: own unit, then `--assume-unit`, then the native unit.
    // frob:ticket 01M4V7WDK6DPPA5Z9FEEPB7ZY9
    pub fn resolve(&self, unit: Option<&str>) -> UnitResolution {
        if let Some(u) = unit {
            return UnitResolution::Explicit(u.to_string());
        }
        if let Some(u) = self.assume_unit.as_ref().or(self.native_unit.as_ref()) {
            tracing::debug!(unit = u.as_str(), "assuming unit at read");
            return UnitResolution::Assumed(u.clone());
        }
        tracing::warn!("no unit information at read; value is inexact");
        UnitResolution::Unknown {
            reason: "no unit information in the format and no --assume-unit".to_string(),
        }
    }

    /// Reads a native value into an observed value; never raises Q001.
    pub fn read(&self, value: &NativeValue) -> ObservedValue {
        match self.resolve(value.unit.as_deref()) {
            UnitResolution::Explicit(u) => ObservedValue {
                literal: value.literal.clone(),
                unit: Some(u),
                unit_assumed: false,
                exact: true,
                reason: None,
            },
            UnitResolution::Assumed(u) => ObservedValue {
                literal: value.literal.clone(),
                unit: Some(u),
                unit_assumed: true,
                exact: true,
                reason: None,
            },
            UnitResolution::Unknown { reason } => ObservedValue {
                literal: value.literal.clone(),
                unit: None,
                unit_assumed: false,
                exact: false,
                reason: Some(reason),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare() -> NativeValue {
        NativeValue {
            literal: "12.5".into(),
            unit: None,
        }
    }

    #[test]
    fn unitless_takes_native_unit_and_is_assumed() {
        let p = UnitPolicy {
            native_unit: Some("mm".into()),
            assume_unit: None,
        };
        let v = p.read(&bare());
        assert_eq!(v.unit.as_deref(), Some("mm"));
        assert!(v.unit_assumed && v.exact);
    }

    #[test]
    fn no_unit_info_is_inexact_with_reason() {
        let v = UnitPolicy::default().read(&bare());
        assert!(!v.exact && !v.unit_assumed);
        assert!(v.reason.is_some());
    }

    #[test]
    fn assume_unit_applies_without_native() {
        let p = UnitPolicy {
            native_unit: None,
            assume_unit: Some("in".into()),
        };
        let v = p.read(&bare());
        assert_eq!(v.unit.as_deref(), Some("in"));
        assert!(v.unit_assumed);
    }

    // frob:ticket 01M4V7WDK6DPPA5Z9FEEPB7ZY9
    // frob:tests crates/cadetta-adapter/src/read_units.rs::UnitPolicy.resolve
    #[test]
    fn assume_unit_wins_over_native_unit() {
        let p = UnitPolicy {
            native_unit: Some("mm".into()),
            assume_unit: Some("in".into()),
        };
        assert_eq!(p.read(&bare()).unit.as_deref(), Some("in"));
    }

    #[test]
    fn read_never_emits_q001() {
        for p in [
            UnitPolicy::default(),
            UnitPolicy {
                native_unit: Some("mm".into()),
                assume_unit: None,
            },
        ] {
            let v = p.read(&bare());
            let text = format!("{v:?}");
            assert!(!text.contains("Q001"));
        }
    }

    #[test]
    fn explicit_unit_is_kept() {
        let p = UnitPolicy {
            native_unit: Some("mm".into()),
            assume_unit: None,
        };
        let v = p.read(&NativeValue {
            literal: "1".into(),
            unit: Some("in".into()),
        });
        assert_eq!(v.unit.as_deref(), Some("in"));
        assert!(!v.unit_assumed);
    }
}
