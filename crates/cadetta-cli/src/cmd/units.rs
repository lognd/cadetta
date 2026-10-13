//! frob:ticket 01M4SPWT6N1HY3MHN544GZJYS9
//! `cadetta units [KIND]`: print the built-in kind and unit tables with exact SI factors
//! (quantities.md 3, 4; config-cli.md 3).

use std::io::Write;

use cadetta_ir::json::rational_text;
use cadetta_ir::model::Exact;
use cadetta_units::{KindTable, UnitKind, UnitTable};
use serde_json::{Value, json};

use crate::output::Format;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Schema tag of the JSON output (`docs/schemas/units.json`).
pub const SCHEMA: &str = "cadetta.units/1";

const USAGE: &str = "usage: cadetta units [KIND]";

/// One unit row: symbol, exact factor to SI and whether it is affine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRow {
    /// Unit symbol.
    pub name: String,
    /// Exact factor to SI as `(num/den text, pi exponent)`.
    pub factor: (String, i32),
    /// True for degC and degF.
    pub affine: bool,
}

/// One kind with its dimension and the units whose default kind it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindRow {
    /// Kind name.
    pub kind: String,
    /// Rendered dimension (`L^1 M^1 T^-2`; `1` when dimensionless).
    pub dim: String,
    /// Angle exponent.
    pub angle: i32,
    /// Atom units, sorted by symbol.
    pub units: Vec<UnitRow>,
    /// Compound unit expressions with this default kind.
    pub compounds: Vec<String>,
}

/// Build the rows of the built-in tables, optionally only for `kind`; `None` if `kind` is unknown.
pub fn rows(kind: Option<&str>) -> Option<Vec<KindRow>> {
    let kinds = KindTable::builtin();
    let table = UnitTable::builtin();
    let atoms = table.atoms();
    let mut all: Vec<KindRow> = kinds
        .all()
        .into_iter()
        .filter(|(name, _)| kind.is_none_or(|k| k == *name))
        .map(|(name, dim)| KindRow {
            kind: name.to_owned(),
            dim: dim.dimension.render(),
            angle: dim.angle,
            units: atoms
                .iter()
                .filter(|u| u.kind == UnitKind::Named(name.to_owned()))
                .map(|u| UnitRow {
                    name: u.name.clone(),
                    factor: (rational_text(&Exact(u.factor.clone())), u.factor.pi_exp()),
                    affine: u.affine,
                })
                .collect(),
            compounds: UnitTable::compounds()
                .iter()
                .filter(|(_, k)| *k == name)
                .map(|(expr, _)| (*expr).to_owned())
                .collect(),
        })
        .collect();
    all.sort_by(|a, b| a.kind.cmp(&b.kind));
    tracing::debug!(?kind, rows = all.len(), "built unit table rows");
    if kind.is_some() && all.is_empty() {
        None
    } else {
        Some(all)
    }
}

/// The `cadetta.units/1` record.
pub fn to_json(rows: &[KindRow]) -> Value {
    let kinds: Vec<Value> = rows
        .iter()
        .map(|k| {
            json!({
                "kind": k.kind,
                "dim": k.dim,
                "angle": k.angle,
                "units": k.units.iter().map(|u| json!({
                    "name": u.name,
                    "factor": { "r": u.factor.0, "pi": u.factor.1 },
                    "affine": u.affine,
                })).collect::<Vec<_>>(),
                "compounds": k.compounds,
            })
        })
        .collect();
    json!({ "schema": SCHEMA, "kinds": kinds })
}

/// The human text: one block per kind.
pub fn to_text(rows: &[KindRow]) -> String {
    let mut s = String::new();
    for k in rows {
        s.push_str(&format!("{} [{}", k.kind, k.dim));
        if k.angle != 0 {
            s.push_str(&format!(", angle^{}", k.angle));
        }
        s.push_str("]\n");
        for u in &k.units {
            let pi = match u.factor.1 {
                0 => String::new(),
                e => format!(" * pi^{e}"),
            };
            let affine = if u.affine { " (affine)" } else { "" };
            s.push_str(&format!("  {} = {}{pi}{affine}\n", u.name, u.factor.0));
        }
        for c in &k.compounds {
            s.push_str(&format!("  {c}\n"));
        }
    }
    s
}

/// Run `cadetta units`; `args` are the words after the verb. Returns the exit code.
pub fn run_units(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "units failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let kind = match args {
        [] => None,
        [kind] if !kind.starts_with('-') => Some(kind.as_str()),
        _ => return fail(err, USAGE),
    };
    let Some(rows) = rows(kind) else {
        return fail(err, &format!("unknown kind `{}`", kind.unwrap_or_default()));
    };
    let text = if globals.format == Format::Json {
        format!("{}\n", to_json(&rows))
    } else {
        to_text(&rows)
    };
    match out.write_all(text.as_bytes()) {
        Ok(()) => EXIT_OK,
        Err(error) => fail(err, &format!("could not write output: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str], format: Format) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let globals = Globals {
            format,
            cwd: None,
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_units(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    // frob:tests crates/cadetta-cli/src/cmd/units.rs::run_units
    #[test]
    fn length_lists_its_units_with_exact_factors() {
        let (code, out, _) = run(&["Length"], Format::Text);
        assert_eq!(code, EXIT_OK);
        assert!(out.starts_with("Length [L^1]\n"), "{out}");
        assert!(out.contains("  mm = 1/1000\n"), "{out}");
        assert!(out.contains("  in = 127/5000\n"), "{out}");
        assert!(!out.contains("Force"), "{out}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/units.rs::rows
    #[test]
    fn whole_table_and_unknown_kind() {
        let all = rows(None).unwrap();
        assert!(all.len() > 10);
        assert!(rows(Some("Nope")).is_none());
        assert_eq!(run(&["Nope"], Format::Text).0, EXIT_USAGE);
        assert_eq!(run(&["a", "b"], Format::Text).0, EXIT_USAGE);
        let torque = rows(Some("Torque")).unwrap();
        assert_eq!(torque[0].angle, -1);
        assert!(torque[0].compounds.contains(&"N*m".to_owned()));
    }

    // frob:tests crates/cadetta-cli/src/cmd/units.rs::to_json
    #[test]
    fn json_output_validates_against_units_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/units.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for args in [&[][..], &["Length"], &["Temperature"]] {
            let (code, out, _) = run(args, Format::Json);
            assert_eq!(code, EXIT_OK);
            let value: Value = serde_json::from_str(&out).unwrap();
            assert!(validator.is_valid(&value), "{args:?}: {value}");
        }
    }
}
