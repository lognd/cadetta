//! frob:ticket 01M4SXSGQQHR6RMBNJQJ3392AZ
//! Exact quantities for the golden compare: expected source literals and IR values.

use cadetta_ir::dim::DimWire;
use cadetta_units::{Magnitude, Rational, Sum, UnitTable};
use serde_json::Value as Json;

/// An exact magnitude with its dimension vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exact {
    /// `r * pi^k` in SI base units.
    pub magnitude: Magnitude,
    /// The dimension vector.
    pub dim: DimWire,
}

impl std::fmt::Display for Exact {
    /// Show the SI magnitude and the dimension.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.magnitude.rational())?;
        if self.magnitude.pi_exp() != 0 {
            write!(f, "*pi^{}", self.magnitude.pi_exp())?;
        }
        write!(f, " SI {}", dim_text(&self.dim))
    }
}

/// A compact dimension text such as `m^1 angle^1`.
fn dim_text(d: &DimWire) -> String {
    let parts = [
        ("m", d.m),
        ("kg", d.kg),
        ("s", d.s),
        ("A", d.a),
        ("K", d.k),
        ("mol", d.mol),
        ("cd", d.cd),
        ("angle", d.angle),
    ];
    let mut out: Vec<String> = parts
        .iter()
        .filter(|(_, e)| *e != 0)
        .map(|(n, e)| format!("{n}^{e}"))
        .collect();
    out.extend(d.currency.iter().map(|(c, e)| format!("{c}^{e}")));
    if out.is_empty() {
        "1".to_owned()
    } else {
        out.join(" ")
    }
}

/// Parse a source literal (`5.5mm`, `-16.25mm`, `3/786432lb`, `45deg`, `17`) into an exact quantity.
pub fn parse_literal(units: &UnitTable, text: &str) -> Result<Exact, String> {
    let text = text.trim();
    let bad = |why: &str| format!("literal `{text}`: {why}");
    let number_end = |s: &str| {
        let mut end = 0;
        let bytes = s.as_bytes();
        while end < bytes.len() {
            let c = bytes[end] as char;
            let exp_sign =
                matches!(c, '+' | '-') && end > 0 && matches!(bytes[end - 1], b'e' | b'E');
            let exp = matches!(c, 'e' | 'E')
                && end > 0
                && bytes.get(end + 1).is_some_and(|n| {
                    n.is_ascii_digit()
                        || (matches!(n, b'+' | b'-')
                            && bytes.get(end + 2).is_some_and(u8::is_ascii_digit))
                });
            if c.is_ascii_digit()
                || c == '.'
                || exp
                || exp_sign
                || (end == 0 && matches!(c, '+' | '-'))
            {
                end += 1;
            } else {
                break;
            }
        }
        end
    };
    let n_end = number_end(text);
    let mut value = Rational::parse_decimal(&text[..n_end]).map_err(|e| bad(&e.to_string()))?;
    let mut rest = &text[n_end..];
    if let Some(after) = rest.strip_prefix('/')
        && after.starts_with(|c: char| c.is_ascii_digit())
    {
        let d_end = number_end(after);
        let den = Rational::parse_decimal(&after[..d_end]).map_err(|e| bad(&e.to_string()))?;
        value = value.div(&den).map_err(|e| bad(&e.to_string()))?;
        rest = &after[d_end..];
    }
    let unit = rest.trim();
    let (factor, dim) = if unit.is_empty() {
        (Magnitude::new(Rational::one(), 0), DimWire::default())
    } else {
        let resolved = units.parse(unit).map_err(|e| bad(&e.to_string()))?;
        (resolved.factor.clone(), DimWire::from(&resolved.dim))
    };
    Ok(Exact {
        magnitude: Magnitude::new(value, 0).mul(&factor),
        dim,
    })
}

/// An exact `{ "r": "num/den", "pi": k }` object.
pub fn exact_of(json: &Json) -> Result<Magnitude, String> {
    let r = json
        .get("r")
        .and_then(Json::as_str)
        .ok_or("exact value has no `r`")?;
    let pi = json
        .get("pi")
        .and_then(Json::as_i64)
        .ok_or("exact value has no `pi`")?;
    let (n, d) = r
        .split_once('/')
        .ok_or_else(|| format!("bad rational `{r}`"))?;
    let num = Rational::parse_decimal(n).map_err(|e| e.to_string())?;
    let den = Rational::parse_decimal(d).map_err(|e| e.to_string())?;
    let q = num.div(&den).map_err(|e| e.to_string())?;
    let k = i32::try_from(pi).map_err(|e| e.to_string())?;
    Ok(Magnitude::new(q, k))
}

/// The nominal, minimum and maximum of an IR `Value` object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    /// The designed value.
    pub nominal: Exact,
    /// Nominal plus every lowest deviation.
    pub min: Exact,
    /// Nominal plus every highest deviation.
    pub max: Exact,
}

/// Read an IR `Value` into its exact range; an inexact nominal is an error.
pub fn range_of(value: &Json) -> Result<Range, String> {
    let nominal_json = value.get("nominal").ok_or("value has no nominal")?;
    if nominal_json.get("f64").is_some() {
        return Err("the implementation's value is inexact (Q006)".to_owned());
    }
    let dim: DimWire = serde_json::from_value(value.get("dim").cloned().ok_or("value has no dim")?)
        .map_err(|e| format!("bad dim: {e}"))?;
    let nominal = exact_of(nominal_json)?;
    let (mut lo, mut hi) = (nominal.clone(), nominal.clone());
    let terms = value
        .pointer("/tol/terms")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let add = |acc: &Magnitude, dev: &Magnitude| match acc.add(dev) {
        Sum::Exact(m) => Ok(m),
        Sum::Inexact(_) => Err("tolerance terms mix pi exponents (inexact)".to_owned()),
    };
    for term in &terms {
        lo = add(&lo, &exact_of(term.get("lo").ok_or("term has no lo")?)?)?;
        hi = add(&hi, &exact_of(term.get("hi").ok_or("term has no hi")?)?)?;
    }
    let with = |magnitude: Magnitude| Exact {
        magnitude,
        dim: dim.clone(),
    };
    Ok(Range {
        nominal: with(nominal),
        min: with(lo),
        max: with(hi),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests crates/cadetta-cli/tests/golden/quantity.rs::literals_compare_exactly_in_any_unit
    #[test]
    fn literals_compare_exactly_in_any_unit() {
        let u = UnitTable::builtin();
        let a = parse_literal(&u, "25.4mm").expect("mm");
        let b = parse_literal(&u, "1in").expect("in");
        assert_eq!(a, b);
        let c = parse_literal(&u, "1/32in").expect("fraction");
        let d = parse_literal(&u, "0.03125in").expect("decimal");
        assert_eq!(c, d);
        assert_ne!(a, parse_literal(&u, "25.5mm").expect("mm"));
        assert_ne!(a, parse_literal(&u, "25.4kg").expect("kg"));
        assert!(parse_literal(&u, "5 furlongs").is_err());
    }

    // frob:tests crates/cadetta-cli/tests/golden/quantity.rs::range_adds_tolerance_terms
    #[test]
    fn range_adds_tolerance_terms() {
        let v: Json = serde_json::json!({
            "nominal": {"r": "1/100", "pi": 0},
            "dim": {"m": 1},
            "tol": {"terms": [{"var": "a", "lo": {"r": "-1/1000", "pi": 0}, "hi": {"r": "2/1000", "pi": 0}}]}
        });
        let r = range_of(&v).expect("range");
        let u = UnitTable::builtin();
        assert_eq!(r.min, parse_literal(&u, "9mm").expect("min"));
        assert_eq!(r.max, parse_literal(&u, "12mm").expect("max"));
        let inexact = serde_json::json!({"nominal": {"f64": 1.0}, "dim": {}});
        assert!(range_of(&inexact).is_err());
    }
}
