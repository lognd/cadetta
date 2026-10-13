//! The `cadetta.solve/1` request and response (solvers.md 3).
//!
//! Decisions made here (written into solvers.md 3): a response output must be
//! a JSON string that lexes as exactly one quantity literal. A JSON number, a
//! bare-number string, or a string holding any other expression is Q001
//! against the solver; a string with a lexical error, or a non-string,
//! non-number value, is a malformed response. Every other failure (bad JSON,
//! wrong or missing schema, wrong shape) is a typed [`ProtocolError`] that the
//! caller reports as S005 and keeps the previous result.

// frob:ticket 01M4STAMZCNT7MPR1PGZ5GVZZ5

use std::collections::BTreeMap;

use cadetta_syntax::lexer::{Tok, lex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, warn};

/// The schema tag carried by requests and responses.
pub const SCHEMA: &str = "cadetta.solve/1";

/// One request input: the exact literal plus SI conveniences.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct InputValue {
    /// The exact nominal literal, e.g. `300mm`.
    pub lit: String,
    /// The nominal in SI base units (convenience float).
    pub si: f64,
    /// Lower limit in SI base units; present only when toleranced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Upper limit in SI base units; present only when toleranced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

/// A `cadetta.solve/1` request.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Request {
    /// Always [`SCHEMA`].
    pub schema: String,
    /// The solver's path, e.g. `acme.rover::wall_sizing`.
    pub solver: String,
    /// Inputs by name, in sorted order.
    pub inputs: BTreeMap<String, InputValue>,
}

impl Request {
    /// Build a request for `solver` over `inputs`.
    pub fn new(solver: impl Into<String>, inputs: BTreeMap<String, InputValue>) -> Self {
        let solver = solver.into();
        debug!(%solver, n = inputs.len(), "building solve request");
        Self {
            schema: SCHEMA.to_string(),
            solver,
            inputs,
        }
    }

    /// Serialize to the JSON text sent to the solver.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("request is plain data and always serializes")
    }
}

/// Severity of a solver diagnostic.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational.
    Info,
    /// Worth attention.
    Warning,
    /// The solver reports a problem.
    Error,
}

/// One diagnostic from a solver.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SolverDiagnostic {
    /// How serious.
    pub severity: Severity,
    /// The message text.
    pub message: String,
}

/// A parsed response: outputs are validated literal strings.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Response {
    /// The solver's own version, when it reports one.
    pub version: Option<String>,
    /// Output literals by name (each lexes as one quantity literal).
    pub outputs: BTreeMap<String, String>,
    /// Diagnostics, possibly empty.
    pub diagnostics: Vec<SolverDiagnostic>,
}

/// Why a response could not be accepted.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum ProtocolError {
    /// The text is not JSON.
    #[error("response is not valid JSON: {0}")]
    MalformedJson(String),
    /// The JSON does not have the response shape.
    #[error("malformed response: {0}")]
    Malformed(String),
    /// `schema` is missing or is not `cadetta.solve/1`.
    #[error("wrong or missing schema: expected `cadetta.solve/1`, found {found}")]
    WrongSchema {
        /// What was found, rendered.
        found: String,
    },
    /// An output is not a quantity literal (Q001 against the solver).
    #[error("output `{name}` is not a quantity literal: {reason}")]
    NotLiteral {
        /// The output name.
        name: String,
        /// Why it was rejected.
        reason: String,
    },
}

impl ProtocolError {
    /// The catalog code the caller reports: Q001 for non-literal outputs, else S005.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotLiteral { .. } => "Q001",
            _ => "S005",
        }
    }
}

/// Check one output value is a single quantity literal string.
fn check_literal(name: &str, v: &Value) -> Result<String, ProtocolError> {
    let bad = |reason: &str| ProtocolError::NotLiteral {
        name: name.to_string(),
        reason: reason.to_string(),
    };
    let s = match v {
        Value::String(s) => s,
        Value::Number(_) => return Err(bad("a bare number has no unit")),
        _ => {
            return Err(ProtocolError::Malformed(format!(
                "output `{name}` must be a string"
            )));
        }
    };
    let lexed = lex(s.trim());
    if !lexed.diags.is_empty() {
        return Err(ProtocolError::Malformed(format!(
            "output `{name}` has a lexical error: {}",
            lexed.diags[0].message
        )));
    }
    let toks: Vec<&Tok> = lexed.tokens.iter().map(|t| &t.tok).collect();
    match toks.as_slice() {
        [Tok::Quantity(_), Tok::Eof] => Ok(s.trim().to_string()),
        [Tok::Number(_), Tok::Eof] => Err(bad("a bare number has no unit")),
        _ => Err(bad("an expression is not a literal")),
    }
}

/// Parse and validate a solver response (solvers.md 3).
pub fn parse_response(text: &str) -> Result<Response, ProtocolError> {
    let fail = |e: ProtocolError| {
        warn!(code = e.code(), error = %e, "solver response rejected");
        e
    };
    let root: Value = serde_json::from_str(text)
        .map_err(|e| fail(ProtocolError::MalformedJson(e.to_string())))?;
    let obj = root.as_object().ok_or_else(|| {
        fail(ProtocolError::Malformed(
            "top level must be an object".into(),
        ))
    })?;
    match obj.get("schema") {
        Some(Value::String(s)) if s == SCHEMA => {}
        other => {
            return Err(fail(ProtocolError::WrongSchema {
                found: other.map_or_else(|| "nothing".to_string(), Value::to_string),
            }));
        }
    }
    let version = match obj.get("version") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            return Err(fail(ProtocolError::Malformed(
                "`version` must be a string".into(),
            )));
        }
    };
    let raw = obj
        .get("outputs")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            fail(ProtocolError::Malformed(
                "`outputs` must be an object".into(),
            ))
        })?;
    let mut outputs = BTreeMap::new();
    for (name, v) in raw {
        outputs.insert(name.clone(), check_literal(name, v).map_err(fail)?);
    }
    let diagnostics = match obj.get("diagnostics") {
        None | Some(Value::Null) => Vec::new(),
        Some(d) => serde_json::from_value(d.clone())
            .map_err(|e| fail(ProtocolError::Malformed(format!("bad `diagnostics`: {e}"))))?,
    };
    debug!(n = outputs.len(), "parsed solve response");
    Ok(Response {
        version,
        outputs,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"schema":"cadetta.solve/1","version":"1.4.2",
        "outputs":{"min_wall":"3.2mm"},
        "diagnostics":[{"severity":"info","message":"converged in 14 iterations"}]}"#;

    // frob:tests crates/cadetta-solve/src/protocol.rs::Request.new kind=unit
    #[test]
    fn request_is_solve_1_json() {
        let mut inputs = BTreeMap::new();
        inputs.insert(
            "load".to_string(),
            InputValue {
                lit: "1200N".into(),
                si: 1200.0,
                min: None,
                max: None,
            },
        );
        inputs.insert(
            "span".to_string(),
            InputValue {
                lit: "300mm".into(),
                si: 0.3,
                min: Some(0.2995),
                max: Some(0.3005),
            },
        );
        let json = Request::new("acme.rover::wall_sizing", inputs).to_json();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["schema"], "cadetta.solve/1");
        assert_eq!(v["solver"], "acme.rover::wall_sizing");
        assert_eq!(v["inputs"]["load"]["lit"], "1200N");
        assert!(v["inputs"]["load"].get("min").is_none());
        assert_eq!(v["inputs"]["span"]["min"], 0.2995);
        assert_eq!(v["inputs"]["span"]["max"], 0.3005);
    }

    // frob:tests crates/cadetta-solve/src/protocol.rs::parse_response kind=unit
    #[test]
    fn good_response_parses() {
        let r = parse_response(GOOD).unwrap();
        assert_eq!(r.version.as_deref(), Some("1.4.2"));
        assert_eq!(r.outputs["min_wall"], "3.2mm");
        assert_eq!(r.diagnostics[0].severity, Severity::Info);
    }

    // frob:tests crates/cadetta-solve/src/protocol.rs::parse_response kind=unit
    #[test]
    fn expression_or_bare_output_is_q001() {
        for out in [r#""3.2""#, "3.2", r#""2mm + 1mm""#, r#""a.b""#] {
            let t = format!(r#"{{"schema":"cadetta.solve/1","outputs":{{"o":{out}}}}}"#);
            let e = parse_response(&t).unwrap_err();
            assert_eq!(e.code(), "Q001", "{out}: {e}");
        }
    }

    // frob:tests crates/cadetta-solve/src/protocol.rs::parse_response kind=unit
    #[test]
    fn malformed_or_wrong_version_is_typed_error() {
        let cases = [
            ("not json", "S005"),
            ("[]", "S005"),
            (r#"{"outputs":{}}"#, "S005"),
            (r#"{"schema":"cadetta.solve/2","outputs":{}}"#, "S005"),
            (r#"{"schema":"cadetta.solve/1"}"#, "S005"),
            (
                r#"{"schema":"cadetta.solve/1","outputs":{"o":true}}"#,
                "S005",
            ),
            (
                r#"{"schema":"cadetta.solve/1","outputs":{},"version":3}"#,
                "S005",
            ),
        ];
        for (t, code) in cases {
            let e = parse_response(t).unwrap_err();
            assert_eq!(e.code(), code, "{t}");
        }
        assert!(matches!(
            parse_response(r#"{"schema":"cadetta.solve/2","outputs":{}}"#),
            Err(ProtocolError::WrongSchema { .. })
        ));
    }
}
