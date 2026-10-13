//! The IR wire form of a dimension vector (ir.md 3 `Value.dim`).
//!
//! The in-memory type is `cadetta_units::Dim`; this module owns only its JSON shape: integer
//! exponents for the SI bases, `angle`, and a currency map keyed by ISO 4217 code. With it a
//! consumer needs no unit table to convert (`.to(unit)` is `nominal / unit.factor` once the
//! dimensions agree).
// frob:ticket 01M4SWK92ASN873QYBKXGPTZV0

use std::collections::BTreeMap;

use cadetta_units::{Dim, Dimension};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// A dimension vector as serialized in the IR.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DimWire {
    /// Length exponent (metre).
    #[serde(default)]
    pub m: i32,
    /// Mass exponent (kilogram).
    #[serde(default)]
    pub kg: i32,
    /// Time exponent (second).
    #[serde(default)]
    pub s: i32,
    /// Electric current exponent (ampere).
    #[serde(default, rename = "A")]
    pub a: i32,
    /// Temperature exponent (kelvin).
    #[serde(default, rename = "K")]
    pub k: i32,
    /// Amount exponent (mole).
    #[serde(default)]
    pub mol: i32,
    /// Luminous intensity exponent (candela).
    #[serde(default)]
    pub cd: i32,
    /// Angle exponent.
    #[serde(default)]
    pub angle: i32,
    /// Currency exponents keyed by ISO 4217 code; zero entries never appear.
    #[serde(default)]
    pub currency: BTreeMap<String, i32>,
}

impl From<&Dim> for DimWire {
    /// Project a units `Dim` onto its wire form.
    fn from(d: &Dim) -> Self {
        let si = d.dimension.si();
        let currency = d.dimension.currencies().clone();
        debug!(dim = %d.dimension.render(), "ir: dim to wire");
        Self {
            m: si[0],
            kg: si[1],
            s: si[2],
            a: si[3],
            k: si[4],
            mol: si[5],
            cd: si[6],
            angle: d.angle,
            currency,
        }
    }
}

impl From<&DimWire> for Dim {
    /// Rebuild a units `Dim` from its wire form, dropping zero currency entries.
    fn from(w: &DimWire) -> Self {
        let mut dimension = Dimension::from_si([w.m, w.kg, w.s, w.a, w.k, w.mol, w.cd]);
        for (code, e) in w.currency.iter().filter(|(_, e)| **e != 0) {
            let base = Dimension::currency(code).powi(*e);
            dimension = dimension.mul(&base);
        }
        debug!(dim = %dimension.render(), "ir: dim from wire");
        Dim::new(dimension, w.angle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_units::{Magnitude, Rational, UnitTable};

    // frob:tests ir_dim_serialized_exponents
    #[test]
    fn ir_dim_serialized_exponents() {
        let t = UnitTable::builtin();
        let r = t.parse("USD*kg*m/s^2").unwrap();
        let w = DimWire::from(&r.dim);
        let j = serde_json::to_value(&w).unwrap();
        assert_eq!(j["m"], 1);
        assert_eq!(j["kg"], 1);
        assert_eq!(j["s"], -2);
        for k in ["A", "K", "mol", "cd", "angle"] {
            assert_eq!(j[k], 0, "{k}");
        }
        assert_eq!(j["currency"]["USD"], 1);
        let back: DimWire = serde_json::from_value(j).unwrap();
        assert_eq!(Dim::from(&back), r.dim);
    }

    // frob:tests ir_dim_converts_without_unit_table
    #[test]
    fn ir_dim_converts_without_unit_table() {
        let t = UnitTable::builtin();
        let mm = t.parse("mm").unwrap();
        let json = serde_json::to_string(&DimWire::from(&mm.dim)).unwrap();
        // A consumer holding only the IR value: dim plus nominal in SI base (0.025 m).
        let dim = Dim::from(&serde_json::from_str::<DimWire>(&json).unwrap());
        let nominal = Magnitude::new(Rational::new(25, 1000).unwrap(), 0);
        assert!(dim.check_same_dimension(&mm.dim).is_ok());
        let in_mm = nominal.div(&mm.factor).unwrap();
        assert_eq!(in_mm, Magnitude::new(Rational::from_int(25), 0));
    }
}
