//! frob:ticket 01M4SPWT6N1HY3MHN544GZJYS9
//! `cadetta explain CODE`: print the lint catalog entry for a diagnostic code
//! (lints.md 3, config-cli.md 3).

use std::io::Write;

use cadetta_lint::engine::{CodeInfo, explain};
use serde_json::{Value, json};

use crate::output::Format;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Schema tag of the JSON output (`docs/schemas/explain.json`).
pub const SCHEMA: &str = "cadetta.explain/1";

const USAGE: &str = "usage: cadetta explain CODE";

/// The `cadetta.explain/1` record of one catalog entry.
pub fn to_json(info: &CodeInfo) -> Value {
    json!({
        "schema": SCHEMA,
        "code": info.code,
        "name": info.name,
        "default": info.default,
        "fixed": info.fixed,
        "meaning": info.meaning,
    })
}

/// The human text of one catalog entry.
pub fn to_text(info: &CodeInfo) -> String {
    let severity = serde_json::to_value(info.default)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    let configurable = if info.fixed {
        "fixed (never configurable or waivable)"
    } else {
        "configurable"
    };
    format!(
        "{} {}\ndefault severity: {severity} ({configurable})\n{}\n",
        info.code, info.name, info.meaning
    )
}

/// Run `cadetta explain`; `args` are the words after the verb. Returns the exit code.
pub fn run_explain(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "explain failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let [code] = args else {
        return fail(err, USAGE);
    };
    let Some(info) = explain(&code.to_ascii_uppercase()) else {
        return fail(err, &format!("unknown diagnostic code `{code}`"));
    };
    tracing::debug!(code = info.code, format = ?globals.format, "explaining code");
    let text = if globals.format == Format::Json {
        format!("{}\n", to_json(info))
    } else {
        to_text(info)
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
        let code = run_explain(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    // frob:tests crates/cadetta-cli/src/cmd/explain.rs::run_explain
    #[test]
    fn prints_the_catalog_entry() {
        let (code, out, _) = run(&["Q007"], Format::Text);
        assert_eq!(code, EXIT_OK);
        assert!(out.starts_with("Q007 "), "{out}");
        assert!(out.contains(explain("Q007").unwrap().meaning));
    }

    // frob:tests crates/cadetta-cli/src/cmd/explain.rs::run_explain
    #[test]
    fn unknown_code_and_missing_argument_are_usage_errors() {
        let (code, out, err) = run(&["Z999"], Format::Text);
        assert_eq!((code, out.as_str()), (EXIT_USAGE, ""));
        assert!(err.contains("unknown diagnostic code"));
        assert_eq!(run(&[], Format::Text).0, EXIT_USAGE);
        assert_eq!(run(&["Q007", "Q008"], Format::Text).0, EXIT_USAGE);
    }

    // frob:tests crates/cadetta-cli/src/cmd/explain.rs::to_json
    #[test]
    fn json_output_validates_against_explain_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/explain.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for info in cadetta_lint::engine::CATALOG {
            let (code, out, _) = run(&[info.code], Format::Json);
            assert_eq!(code, EXIT_OK);
            let value: Value = serde_json::from_str(&out).unwrap();
            assert!(validator.is_valid(&value), "{}: {value}", info.code);
            assert_eq!(value["code"], info.code);
        }
    }
}
