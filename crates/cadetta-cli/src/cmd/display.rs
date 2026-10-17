//! frob:ticket 01M4V1HED894WSWEEJ9K2NMWYA
//! `cadetta display VALUE SPEC`: print one quantity through the display formatter
//! (quantities.md 6), the conformance surface of the display rules.

use std::io::Write;

use cadetta_units::{Dim, DisplaySpec, Magnitude, Rational, UnitTable, format_value};
use serde_json::{Value, json};

use crate::output::Format;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Schema tag of the JSON output (`docs/schemas/display.json`).
pub const SCHEMA: &str = "cadetta.display/1";

const USAGE: &str = "usage: cadetta display VALUE SPEC";

/// Split a literal such as `45.9375in`, `0.0125` or the exact fraction `1/3m` into its exact SI
/// value and dimension; a `/` belongs to the number only when a digit follows it.
fn parse_value(table: &UnitTable, literal: &str) -> Result<(Magnitude, Dim), String> {
    let bytes = literal.as_bytes();
    let split = literal
        .char_indices()
        .find(|&(i, c)| {
            let numeric = c.is_ascii_digit() || matches!(c, '.' | 'e' | '-' | '+');
            let fraction = c == '/' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit);
            !(numeric || fraction)
        })
        .map_or(literal.len(), |(i, _)| i);
    let (num, unit) = literal.split_at(split);
    let n = match num.split_once('/') {
        Some((top, bottom)) => {
            let top =
                Rational::parse_decimal(top).map_err(|e| format!("value `{literal}`: {e}"))?;
            let bottom =
                Rational::parse_decimal(bottom).map_err(|e| format!("value `{literal}`: {e}"))?;
            tracing::debug!(literal, "exact fraction numerator");
            top.div(&bottom)
                .map_err(|e| format!("value `{literal}`: {e}"))?
        }
        None => Rational::parse_decimal(num).map_err(|e| format!("value `{literal}`: {e}"))?,
    };
    let n = Magnitude::new(n, 0);
    if unit.is_empty() {
        return Ok((n, Dim::default()));
    }
    let resolved = table
        .parse(unit)
        .map_err(|e| format!("value `{literal}`: {e}"))?;
    Ok((n.mul(&resolved.factor), resolved.dim))
}

/// Format the quantity literal `value` by the display `spec` text; errors are messages.
pub fn display_text(value: &str, spec: &str) -> Result<String, String> {
    let table = UnitTable::builtin();
    let (magnitude, dim) = parse_value(&table, value)?;
    let spec = DisplaySpec::parse(spec).map_err(|e| e.to_string())?;
    let text = format_value(&table, &magnitude, &dim, &spec).map_err(|e| e.to_string())?;
    tracing::debug!(value, ?spec, %text, "formatted quantity");
    Ok(text)
}

/// The `cadetta.display/1` record.
pub fn to_json(value: &str, spec: &str, text: &str) -> Value {
    json!({ "schema": SCHEMA, "value": value, "spec": spec, "text": text })
}

/// Run `cadetta display`; `args` are the words after the verb. Returns the exit code.
pub fn run_display(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "display failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let [value, spec] = args else {
        return fail(err, USAGE);
    };
    let text = match display_text(value, spec) {
        Ok(text) => text,
        Err(message) => return fail(err, &message),
    };
    let body = if globals.format == Format::Json {
        format!("{}\n", to_json(value, spec, &text))
    } else {
        format!("{text}\n")
    };
    match out.write_all(body.as_bytes()) {
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
        let code = run_display(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    // frob:tests crates/cadetta-cli/src/cmd/display.rs::display_text
    #[test]
    fn formats_the_documented_examples() {
        assert_eq!(display_text("45.9375in", "in/32").unwrap(), "45-15/16in");
        assert_eq!(display_text("45.9375in", "ft_in").unwrap(), "3'9-15/16\"");
        assert_eq!(display_text("0.0125", "%").unwrap(), "1.3%");
        assert_eq!(display_text("0.25m", "mm").unwrap(), "250mm");
    }

    // frob:tests crates/cadetta-cli/src/cmd/display.rs::display_text
    #[test]
    fn exact_fraction_numerator() {
        assert_eq!(display_text("1/3m", "mm").unwrap(), "333.333333mm");
        assert!(display_text("1/0m", "mm").is_err());
    }

    // frob:tests crates/cadetta-cli/src/cmd/display.rs::display_text
    #[test]
    fn bad_input_is_an_error_message() {
        assert!(display_text("12xyz", "mm").is_err());
        assert!(display_text("1m", "").is_err());
        assert!(display_text("1m", "kg").is_err());
        assert!(display_text("abc", "mm").is_err());
    }

    // frob:tests crates/cadetta-cli/src/cmd/display.rs::run_display
    #[test]
    fn text_json_and_usage() {
        let (code, out, _) = run(&["0.5in", "in/32"], Format::Text);
        assert_eq!((code, out.as_str()), (EXIT_OK, "1/2in\n"));
        let (code, out, _) = run(&["0.25m", "mm"], Format::Json);
        assert_eq!(code, EXIT_OK);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["text"], "250mm");
        assert_eq!(run(&["1m"], Format::Text).0, EXIT_USAGE);
        assert_eq!(run(&["1m", "kg"], Format::Text).0, EXIT_USAGE);
    }

    // frob:tests crates/cadetta-cli/src/cmd/display.rs::to_json
    #[test]
    fn json_output_validates_against_display_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/display.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let (code, out, _) = run(&["45.9375in", "ft_in"], Format::Json);
        assert_eq!(code, EXIT_OK);
        let value: Value = serde_json::from_str(&out).unwrap();
        assert!(validator.is_valid(&value), "{value}");
    }
}
