// frob:ticket 01M4SPWNBYWT610BHC6GNSP35P
//! Built-in unit table with exact SI factors `r * pi^k` (quantities.md 3), unit
//! expression parsing (quantities.md 2, 4.1 rule 1) and user `unit`
//! declarations (language.md 3.10). No stored value is ever an `f64`.

use std::collections::{BTreeMap, HashMap};

use thiserror::Error;
use tracing::{debug, trace};

use crate::affine::{TempClass, TemperatureUnit};
use crate::dimension::{Dim, Dimension};
use crate::rational::{Magnitude, MagnitudeError, Rational};

/// The kind a unit (or unit expression) gives a literal (quantities.md 4.1 rule 1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnitKind {
    /// A named kind such as `Length` or `Force`.
    Named(String),
    /// A compound unit outside the table: anonymous, dimension and angle only.
    Anonymous,
}

impl UnitKind {
    fn named(name: &str) -> Self {
        Self::Named(name.to_string())
    }
}

/// Errors from unit lookup, parsing and registration, returned as values.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UnitError {
    /// The unit expression is empty.
    #[error("empty unit expression")]
    Empty,
    /// No unit with this name is known.
    #[error("unknown unit `{0}`")]
    Unknown(String),
    /// The expression is malformed (bad exponent, stray operator, ...).
    #[error("malformed unit expression `{0}`")]
    Syntax(String),
    /// An affine unit appeared in a compound expression or with a power.
    #[error("affine unit `{0}` cannot be multiplied, divided or raised to a power")]
    AffineInCompound(String),
    /// The name is already registered.
    #[error("unit `{0}` is already defined")]
    Duplicate(String),
    /// The name is not a valid unit name.
    #[error("invalid unit name `{0}`")]
    InvalidName(String),
    /// Exact arithmetic failed (for example a zero scale).
    #[error(transparent)]
    Magnitude(#[from] MagnitudeError),
}

/// A single registered unit: exact factor to SI, dimension and default kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitDef {
    /// Unit symbol.
    pub name: String,
    /// Exact factor `r * pi^k` to SI base units.
    pub factor: Magnitude,
    /// Dimension and angle exponent.
    pub dim: Dim,
    /// Default kind of a literal in this unit.
    pub kind: UnitKind,
    /// True for degC and degF, which carry an offset and cannot be composed.
    pub affine: bool,
}

/// A resolved unit expression: factor, dimension, angle and kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedUnit {
    /// Exact factor to SI base units.
    pub factor: Magnitude,
    /// Dimension and angle exponent.
    pub dim: Dim,
    /// Default kind of a literal in this unit expression.
    pub kind: UnitKind,
}

/// Canonical form of a unit expression: atoms (aliases resolved) with merged exponents.
type Form = Vec<(String, i32)>;

/// (name, num, den, pi exponent, defining expression, default kind).
type Row = (&'static str, i64, i64, i32, &'static str, &'static str);

/// Atoms defined from earlier atoms. Order matters: each row uses only earlier names.
const ATOMS: &[Row] = &[
    // Length
    ("mm", 1, 1000, 0, "m", "Length"),
    ("cm", 1, 100, 0, "m", "Length"),
    ("um", 1, 1_000_000, 0, "m", "Length"),
    ("nm", 1, 1_000_000_000, 0, "m", "Length"),
    ("km", 1000, 1, 0, "m", "Length"),
    ("in", 127, 5000, 0, "m", "Length"),
    ("ft", 12, 1, 0, "in", "Length"),
    ("yd", 3, 1, 0, "ft", "Length"),
    ("mil", 1, 1000, 0, "in", "Length"),
    ("thou", 1, 1000, 0, "in", "Length"),
    // Angle (rad is the base atom with angle exponent 1)
    ("deg", 1, 180, 1, "rad", "Angle"),
    ("arcmin", 1, 60, 0, "deg", "Angle"),
    ("arcsec", 1, 60, 0, "arcmin", "Angle"),
    ("rev", 2, 1, 1, "rad", "Angle"),
    // Volume
    ("L", 1, 1000, 0, "m^3", "Volume"),
    ("mL", 1, 1000, 0, "L", "Volume"),
    // Mass
    ("g", 1, 1000, 0, "kg", "Mass"),
    ("mg", 1, 1000, 0, "g", "Mass"),
    ("lb", 45_359_237, 100_000_000, 0, "kg", "Mass"),
    ("oz", 1, 16, 0, "lb", "Mass"),
    // Acceleration and Force (gn = 9.80665 m/s^2 exactly)
    ("gn", 980_665, 100_000, 0, "m/s^2", "Acceleration"),
    ("N", 1, 1, 0, "kg*m/s^2", "Force"),
    ("kN", 1000, 1, 0, "N", "Force"),
    ("lbf", 1, 1, 0, "lb*gn", "Force"),
    // Energy, Power
    ("J", 1, 1, 0, "N*m", "Energy"),
    ("kJ", 1000, 1, 0, "J", "Energy"),
    ("Wh", 3600, 1, 0, "J", "Energy"),
    ("W", 1, 1, 0, "J/s", "Power"),
    ("kW", 1000, 1, 0, "W", "Power"),
    ("hp", 550, 1, 0, "ft*lbf/s", "Power"),
    // Pressure
    ("Pa", 1, 1, 0, "N/m^2", "Pressure"),
    ("kPa", 1000, 1, 0, "Pa", "Pressure"),
    ("MPa", 1_000_000, 1, 0, "Pa", "Pressure"),
    ("GPa", 1_000_000_000, 1, 0, "Pa", "Pressure"),
    ("psi", 1, 1, 0, "lbf/in^2", "Pressure"),
    ("ksi", 1000, 1, 0, "psi", "Pressure"),
    ("bar", 100_000, 1, 0, "Pa", "Pressure"),
    ("mmH2O", 980_665, 100_000, 0, "Pa", "Pressure"),
    ("inH2O", 24_908_891, 100_000, 0, "Pa", "Pressure"),
    // Time
    ("ms", 1, 1000, 0, "s", "Time"),
    ("min", 60, 1, 0, "s", "Time"),
    ("h", 60, 1, 0, "min", "Time"),
    ("d", 24, 1, 0, "h", "Time"),
    ("wk", 7, 1, 0, "d", "Time"),
    // Frequency, AngularVelocity, VolumeFlow
    ("Hz", 1, 1, 0, "s^-1", "Frequency"),
    ("kHz", 1000, 1, 0, "Hz", "Frequency"),
    ("rpm", 1, 1, 0, "rev/min", "AngularVelocity"),
    ("cfm", 1, 1, 0, "ft^3/min", "VolumeFlow"),
    // Electrical
    ("mA", 1, 1000, 0, "A", "Current"),
    ("V", 1, 1, 0, "W/A", "Voltage"),
    ("mV", 1, 1000, 0, "V", "Voltage"),
    ("ohm", 1, 1, 0, "V/A", "Resistance"),
    ("kohm", 1000, 1, 0, "ohm", "Resistance"),
];

/// Compound unit forms in the table that carry a named default kind.
const COMPOUNDS: &[(&str, &str)] = &[
    ("N*m", "Torque"),
    ("N*mm", "Torque"),
    ("lbf*in", "Torque"),
    ("lbf*ft", "Torque"),
    ("m/s", "Velocity"),
    ("mm/s", "Velocity"),
    ("in/s", "Velocity"),
    ("ft/min", "Velocity"),
    ("m^3/s", "VolumeFlow"),
    ("m^3/h", "VolumeFlow"),
    ("L/min", "VolumeFlow"),
    ("m/s^2", "Acceleration"),
    ("rad/s", "AngularVelocity"),
    ("deg/s", "AngularVelocity"),
    ("mm/rev", "Lead"),
    ("in/rev", "Lead"),
    ("kg*m^2", "Inertia"),
    ("kg/m^3", "Density"),
    ("g/cm^3", "Density"),
    ("lb/in^3", "Density"),
    ("lb/ft^3", "Density"),
];

/// The registry of units: built-ins plus package `unit` declarations.
#[derive(Debug, Clone)]
pub struct UnitTable {
    atoms: HashMap<String, UnitDef>,
    forms: HashMap<Form, UnitKind>,
}

impl Default for UnitTable {
    fn default() -> Self {
        Self::builtin()
    }
}

fn base(name: &str, dim: Dim, kind: UnitKind) -> UnitDef {
    UnitDef {
        name: name.to_string(),
        factor: Magnitude::new(Rational::one(), 0),
        dim,
        kind,
        affine: false,
    }
}

fn si(i: usize) -> Dim {
    let mut v = [0; 7];
    v[i] = 1;
    Dim::new(Dimension::from_si(v), 0)
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['*', '/', '^', ' ', '\t'])
}

fn is_currency_code(name: &str) -> bool {
    name.len() == 3 && name.bytes().all(|b| b.is_ascii_uppercase())
}

impl UnitTable {
    /// The normative built-in table of quantities.md 3.
    pub fn builtin() -> Self {
        let mut t = Self {
            atoms: HashMap::new(),
            forms: HashMap::new(),
        };
        let bases = [
            ("m", si(0), "Length"),
            ("kg", si(1), "Mass"),
            ("s", si(2), "Time"),
            ("A", si(3), "Current"),
            ("K", si(4), "Temperature"),
        ];
        for (name, dim, kind) in bases {
            t.atoms
                .insert(name.to_string(), base(name, dim, UnitKind::named(kind)));
        }
        t.atoms.insert(
            "rad".to_string(),
            base(
                "rad",
                Dim::new(Dimension::dimensionless(), 1),
                UnitKind::named("Angle"),
            ),
        );
        let mut pct = base("%", Dim::default(), UnitKind::named("Ratio"));
        pct.factor = Magnitude::new(Rational::new(1, 100).expect("nonzero denominator"), 0);
        t.atoms.insert("%".to_string(), pct);
        for &(name, num, den, k, expr, kind) in ATOMS {
            let scale = Magnitude::new(Rational::new(num, den).expect("nonzero denominator"), k);
            t.define_with(name, &scale, expr, Some(UnitKind::named(kind)))
                .unwrap_or_else(|e| panic!("builtin unit table row `{name}`: {e}"));
        }
        // Temperature scale factors and affine flags come from affine.rs (single source).
        for unit in TemperatureUnit::ALL {
            if unit == TemperatureUnit::K {
                continue;
            }
            let kind = match unit.class() {
                TempClass::Absolute => "Temperature",
                TempClass::Delta => "TemperatureDelta",
            };
            let scale = Magnitude::new(unit.factor(), 0);
            t.define_with(unit.name(), &scale, "K", Some(UnitKind::named(kind)))
                .unwrap_or_else(|e| panic!("builtin temperature unit `{}`: {e}", unit.name()));
            let def = t.atoms.get_mut(unit.name()).expect("just defined");
            def.affine = unit.is_affine();
        }
        for &(expr, kind) in COMPOUNDS {
            let form = t
                .canonical_form(expr)
                .unwrap_or_else(|e| panic!("builtin compound `{expr}`: {e}"));
            t.forms.insert(form, UnitKind::named(kind));
        }
        debug!(
            atoms = t.atoms.len(),
            forms = t.forms.len(),
            "built unit table"
        );
        t
    }

    /// Look up one unit symbol (`"` and `'` alias `in` and `ft`; ISO 4217 codes are Money).
    pub fn lookup(&self, name: &str) -> Option<UnitDef> {
        let name = match name {
            "\"" => "in",
            "'" => "ft",
            other => other,
        };
        if let Some(u) = self.atoms.get(name) {
            return Some(u.clone());
        }
        if is_currency_code(name) {
            return Some(UnitDef {
                name: name.to_string(),
                factor: Magnitude::new(Rational::one(), 0),
                dim: Dim::new(Dimension::currency(name), 0),
                kind: UnitKind::named("Money"),
                affine: false,
            });
        }
        None
    }

    fn atom_name(name: &str) -> &str {
        match name {
            "\"" => "in",
            "'" => "ft",
            other => other,
        }
    }

    fn parse_terms(&self, text: &str) -> Result<Vec<(String, i32)>, UnitError> {
        let bad = || UnitError::Syntax(text.to_string());
        if text.is_empty() {
            return Err(UnitError::Empty);
        }
        let mut terms = Vec::new();
        let mut sign = 1;
        let mut rest = text;
        loop {
            let end = rest.find(['*', '/', '^']).unwrap_or(rest.len());
            let atom = &rest[..end];
            if atom.is_empty() {
                return Err(bad());
            }
            rest = &rest[end..];
            let mut exp = 1;
            if let Some(r) = rest.strip_prefix('^') {
                let end = r.find(['*', '/']).unwrap_or(r.len());
                exp = r[..end].parse::<i32>().map_err(|_| bad())?;
                rest = &r[end..];
            }
            terms.push((Self::atom_name(atom).to_string(), sign * exp));
            let Some(op) = rest.chars().next() else { break };
            sign = if op == '/' { -1 } else { 1 };
            rest = &rest[1..];
            if rest.is_empty() {
                return Err(bad());
            }
        }
        Ok(terms)
    }

    fn canonical_form(&self, text: &str) -> Result<Form, UnitError> {
        let mut merged: BTreeMap<String, i32> = BTreeMap::new();
        for (atom, exp) in self.parse_terms(text)? {
            *merged.entry(atom).or_insert(0) += exp;
        }
        Ok(merged.into_iter().filter(|&(_, e)| e != 0).collect())
    }

    /// Parse a unit expression (`N*m*s`, `mm/rev`, `5"`'s `"`) into factor, dimension and kind.
    pub fn parse(&self, text: &str) -> Result<ResolvedUnit, UnitError> {
        let terms = self.parse_terms(text)?;
        let mut factor = Magnitude::new(Rational::one(), 0);
        let mut dim = Dim::default();
        let compound = terms.len() > 1 || terms.first().is_some_and(|t| t.1 != 1);
        for (atom, exp) in &terms {
            let def = self
                .lookup(atom)
                .ok_or_else(|| UnitError::Unknown(atom.clone()))?;
            if def.affine && compound {
                return Err(UnitError::AffineInCompound(atom.clone()));
            }
            factor = factor.mul(&def.factor.powi(*exp)?);
            dim = dim.mul(&def.dim.powi(*exp));
        }
        let form = self.canonical_form(text)?;
        let kind = self.kind_of_form(&form, &dim);
        trace!(unit = text, ?kind, "resolved unit expression");
        Ok(ResolvedUnit { factor, dim, kind })
    }

    fn kind_of_form(&self, form: &Form, dim: &Dim) -> UnitKind {
        if let [(atom, exp)] = form.as_slice() {
            let def = self.lookup(atom);
            if *exp == 1 {
                if let Some(d) = def {
                    return d.kind;
                }
            } else if def.is_some_and(|d| d.kind == UnitKind::named("Length")) {
                match exp {
                    2 => return UnitKind::named("Area"),
                    3 => return UnitKind::named("Volume"),
                    _ => {}
                }
            }
        }
        match self.forms.get(form) {
            Some(k) => k.clone(),
            None => {
                trace!(?dim, "compound unit outside the table: anonymous kind");
                UnitKind::Anonymous
            }
        }
    }

    /// Register a user unit `name = scale * expr` whose kind is the expression's kind.
    pub fn define(&mut self, name: &str, scale: &Rational, expr: &str) -> Result<(), UnitError> {
        self.define_with(name, &Magnitude::new(scale.clone(), 0), expr, None)
    }

    /// Register a user unit from a literal such as `1000lbf` (language.md 3.10).
    pub fn define_literal(&mut self, name: &str, literal: &str) -> Result<(), UnitError> {
        let split = literal
            .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | 'e' | '-' | '+')))
            .unwrap_or(literal.len());
        let (num, expr) = literal.split_at(split);
        let scale = Rational::parse_decimal(num)?;
        self.define(name, &scale, expr)
    }

    fn define_with(
        &mut self,
        name: &str,
        scale: &Magnitude,
        expr: &str,
        kind: Option<UnitKind>,
    ) -> Result<(), UnitError> {
        if !valid_name(name) {
            return Err(UnitError::InvalidName(name.to_string()));
        }
        if self.lookup(name).is_some() {
            return Err(UnitError::Duplicate(name.to_string()));
        }
        let r = self.parse(expr)?;
        let def = UnitDef {
            name: name.to_string(),
            factor: scale.mul(&r.factor),
            dim: r.dim,
            kind: kind.unwrap_or(r.kind),
            affine: false,
        };
        debug!(unit = name, expr, "registered unit");
        self.atoms.insert(name.to_string(), def);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rat(n: i64, d: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, d).unwrap(), 0)
    }

    fn named(s: &str) -> UnitKind {
        UnitKind::named(s)
    }

    // frob:tests crates/cadetta-units/src/units.rs::UnitTable.lookup kind=unit
    #[test]
    fn inch_factor() {
        let t = UnitTable::builtin();
        assert_eq!(t.lookup("in").unwrap().factor, rat(127, 5000));
        assert_eq!(t.lookup("ft").unwrap().factor, rat(381, 1250));
        assert_eq!(t.lookup("mil").unwrap().factor, rat(127, 5_000_000));
    }

    // frob:tests crates/cadetta-units/src/units.rs::UnitTable.parse kind=unit
    #[test]
    fn every_table_unit_resolves() {
        let t = UnitTable::builtin();
        let rows: &[(&str, &str)] = &[
            ("m mm cm um nm km in ft yd mil thou", "Length"),
            ("rad deg arcmin arcsec rev", "Angle"),
            ("L mL", "Volume"),
            ("m^2 mm^2 in^2", "Area"),
            ("m^3 in^3", "Volume"),
            ("kg g mg lb oz", "Mass"),
            ("N kN lbf", "Force"),
            ("N*m N*mm lbf*in lbf*ft", "Torque"),
            ("J kJ Wh", "Energy"),
            ("W kW hp", "Power"),
            ("Pa kPa MPa GPa psi ksi bar mmH2O inH2O", "Pressure"),
            ("s ms min h d wk", "Time"),
            ("USD EUR GBP CNY JPY", "Money"),
            ("Hz kHz", "Frequency"),
            ("m/s mm/s in/s ft/min", "Velocity"),
            ("m^3/s m^3/h L/min cfm", "VolumeFlow"),
            ("m/s^2 gn", "Acceleration"),
            ("rad/s rpm deg/s", "AngularVelocity"),
            ("mm/rev in/rev", "Lead"),
            ("A mA", "Current"),
            ("V mV", "Voltage"),
            ("ohm kohm", "Resistance"),
            ("kg*m^2", "Inertia"),
            ("kg/m^3 g/cm^3 lb/in^3 lb/ft^3", "Density"),
            ("K degC degF", "Temperature"),
            ("dK ddegC ddegF", "TemperatureDelta"),
            ("%", "Ratio"),
        ];
        for (units, kind) in rows {
            for u in units.split(' ') {
                let r = t.parse(u).unwrap_or_else(|e| panic!("{u}: {e}"));
                assert_eq!(r.kind, named(kind), "{u}");
                assert!(!r.factor.rational().is_zero(), "{u}");
            }
        }
    }

    #[test]
    fn exact_factors() {
        let t = UnitTable::builtin();
        let f = |u: &str| t.parse(u).unwrap().factor;
        assert_eq!(f("lb"), rat(45_359_237, 100_000_000));
        assert_eq!(f("lbf"), rat(45_359_237 * 980_665, 100_000_000 * 100_000));
        assert_eq!(f("deg"), Magnitude::new(Rational::new(1, 180).unwrap(), 1));
        assert_eq!(f("rev"), Magnitude::new(Rational::new(2, 1).unwrap(), 1));
        assert_eq!(f("rpm"), Magnitude::new(Rational::new(1, 30).unwrap(), 1));
        assert_eq!(f("min"), rat(60, 1));
        assert_eq!(f("L"), rat(1, 1000));
        assert_eq!(f("degF"), rat(5, 9));
        assert_eq!(f("psi"), f("lbf").div(&f("in^2")).unwrap());
    }

    // frob:tests crates/cadetta-units/src/units.rs::UnitTable.parse kind=unit
    #[test]
    fn compound_is_anonymous() {
        let t = UnitTable::builtin();
        let r = t.parse("N*m*s").unwrap();
        assert_eq!(r.kind, UnitKind::Anonymous);
        assert_eq!(r.dim.dimension.si(), &[2, 1, -1, 0, 0, 0, 0]);
        assert_eq!(r.dim.angle, 0);
        // order-insensitive match against the table
        assert_eq!(t.parse("m*N").unwrap().kind, named("Torque"));
    }

    // frob:tests crates/cadetta-units/src/units.rs::UnitTable.lookup kind=unit
    #[test]
    fn quote_aliases() {
        let t = UnitTable::builtin();
        assert_eq!(
            t.lookup("\"").unwrap().factor,
            t.lookup("in").unwrap().factor
        );
        assert_eq!(
            t.lookup("'").unwrap().factor,
            t.lookup("ft").unwrap().factor
        );
        assert_eq!(t.parse("'/s").unwrap().kind, UnitKind::Anonymous);
        assert_eq!(t.parse("\"").unwrap().kind, named("Length"));
    }

    // frob:tests crates/cadetta-units/src/units.rs::UnitTable.parse kind=unit
    #[test]
    fn percent_is_ratio() {
        let t = UnitTable::builtin();
        let r = t.parse("%").unwrap();
        assert_eq!(r.kind, named("Ratio"));
        let five = Magnitude::new(Rational::from_int(5), 0);
        assert_eq!(five.mul(&r.factor), rat(1, 20));
        assert!(r.dim.dimension.is_dimensionless());
    }

    // frob:tests crates/cadetta-units/src/units.rs::UnitTable.define_literal kind=unit
    #[test]
    fn user_unit_resolves_like_builtin() {
        let mut t = UnitTable::builtin();
        t.define_literal("kip", "1000lbf").unwrap();
        let kip = t.parse("kip").unwrap();
        let lbf = t.parse("lbf").unwrap();
        assert_eq!(kip.kind, named("Force"));
        assert_eq!(kip.dim, lbf.dim);
        assert_eq!(kip.factor, rat(1000, 1).mul(&lbf.factor));
        // usable inside compounds, and not redefinable
        assert_eq!(t.parse("kip*in").unwrap().kind, UnitKind::Anonymous);
        assert_eq!(
            t.define_literal("kip", "2lbf"),
            Err(UnitError::Duplicate("kip".into()))
        );
        assert_eq!(
            t.define_literal("in", "2lbf"),
            Err(UnitError::Duplicate("in".into()))
        );
    }

    #[test]
    fn errors_are_values() {
        let t = UnitTable::builtin();
        assert_eq!(t.parse(""), Err(UnitError::Empty));
        assert_eq!(
            t.parse("furlong"),
            Err(UnitError::Unknown("furlong".into()))
        );
        assert!(matches!(t.parse("m*"), Err(UnitError::Syntax(_))));
        assert!(matches!(t.parse("m^x"), Err(UnitError::Syntax(_))));
        assert_eq!(
            t.parse("degC*m"),
            Err(UnitError::AffineInCompound("degC".into()))
        );
        assert!(t.parse("degC").is_ok());
    }

    // frob:tests crates/cadetta-units/src/units.rs::UnitTable.builtin kind=unit
    #[test]
    fn temperature_factors_agree_with_affine() {
        let t = UnitTable::builtin();
        for u in TemperatureUnit::ALL {
            let def = t.parse(u.name()).unwrap();
            assert_eq!(def.factor, Magnitude::new(u.factor(), 0), "{}", u.name());
            assert_eq!(
                t.lookup(u.name()).unwrap().affine,
                u.is_affine(),
                "{}",
                u.name()
            );
        }
    }
}
