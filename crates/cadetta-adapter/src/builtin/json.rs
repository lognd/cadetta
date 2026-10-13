//! Built-in JSON adapter: emits and reads `cadetta.params/1` (emitters/json.md).
//!
//! Values are decimal strings, never JSON numbers. The header (adapter,
//! IR digest, bindings digest) is excluded from change detection. There is
//! no `ref` mode: every locator is a value, so unknown keys are rejected.

use serde::{Deserialize, Serialize};

use crate::policy::{EmitRequest, PolicyError, to_decimal};
use crate::read_units::{NativeValue, ObservedValue, UnitPolicy};

/// Schema tag written to and required from every file.
pub const SCHEMA: &str = "cadetta.params/1";
/// Adapter name.
pub const ADAPTER: &str = "json";
/// Default decimal places (adapters.md 5).
pub const DEFAULT_PRECISION: u32 = 6;

/// The file header: adapter version and the two digests it was built from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    /// Adapter name and version, e.g. `json 0.1.0`.
    pub adapter: String,
    /// IR digest, `blake3:<hex>`.
    pub ir: String,
    /// Bindings digest, `blake3:<hex>`.
    pub bindings: String,
}

/// One emitted parameter; field order is the output key order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Param {
    /// Bound locator (kind `global`) or mangled path.
    pub name: String,
    /// Canonical path.
    pub path: String,
    /// Decimal string, or text for Text parameters.
    pub value: String,
    /// Unit; omitted for Counts, Ratios and Text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Trailing-comment equivalent; omitted when empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema: String,
    header: Header,
    params: Vec<Param>,
}

/// A failure emitting or reading a params file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonError {
    /// Not valid `cadetta.params/1` JSON (includes `ref` mode keys).
    Parse(String),
    /// The `schema` field is not `cadetta.params/1`.
    Schema(String),
    /// A value failed emit policy (Q004).
    Policy(PolicyError),
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "params file: {e}"),
            Self::Schema(s) => write!(f, "unsupported schema `{s}`, expected {SCHEMA}"),
            Self::Policy(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for JsonError {}

impl From<PolicyError> for JsonError {
    fn from(e: PolicyError) -> Self {
        Self::Policy(e)
    }
}

/// Builds a [`Param`] from a decided request; empty unit means unitless, empty note is omitted.
pub fn param_from_request(
    name: &str,
    req: &EmitRequest,
    precision: u32,
    note: &str,
) -> Result<Param, JsonError> {
    let value = to_decimal(req.value, precision)?;
    Ok(Param {
        name: name.to_string(),
        path: req.path.clone(),
        value,
        unit: (!req.unit.is_empty()).then(|| req.unit.clone()),
        note: (!note.is_empty()).then(|| note.to_string()),
    })
}

/// Serialises a params file: 2-space indent, LF, fixed key order, final newline.
pub fn emit(header: &Header, params: &[Param]) -> String {
    let file = File {
        schema: SCHEMA.to_string(),
        header: header.clone(),
        params: params.to_vec(),
    };
    let mut s = serde_json::to_string_pretty(&file).expect("plain structs serialise");
    s.push('\n');
    tracing::debug!(
        params = params.len(),
        bytes = s.len(),
        "json params emitted"
    );
    s
}

fn parse(text: &str) -> Result<File, JsonError> {
    let file: File = serde_json::from_str(text).map_err(|e| {
        tracing::warn!(error = %e, "params file rejected");
        JsonError::Parse(e.to_string())
    })?;
    if file.schema != SCHEMA {
        tracing::warn!(schema = file.schema.as_str(), "unsupported params schema");
        return Err(JsonError::Schema(file.schema));
    }
    Ok(file)
}

/// One parameter as read back, with its resolved unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadParam {
    /// Native name.
    pub name: String,
    /// Canonical path.
    pub path: String,
    /// Value with unit resolution applied.
    pub observed: ObservedValue,
}

/// Reads a params file; a missing `unit` takes the policy's native or assumed unit.
pub fn read(text: &str, units: &UnitPolicy) -> Result<Vec<ReadParam>, JsonError> {
    let file = parse(text)?;
    let out: Vec<ReadParam> = file
        .params
        .into_iter()
        .map(|p| ReadParam {
            observed: units.read(&NativeValue {
                literal: p.value,
                unit: p.unit,
            }),
            name: p.name,
            path: p.path,
        })
        .collect();
    tracing::debug!(params = out.len(), "json params read");
    Ok(out)
}

/// Compares two files ignoring the header (the `sync` no-op comparison, adapters.md 2).
pub fn same_modulo_header(a: &str, b: &str) -> Result<bool, JsonError> {
    Ok(parse(a)?.params == parse(b)?.params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{EmitPoint, Ratio};

    fn header(ir: &str) -> Header {
        Header {
            adapter: "json 0.1.0".into(),
            ir: ir.into(),
            bindings: "blake3:bb".into(),
        }
    }

    fn req(path: &str, num: i128, den: i128, unit: &str) -> EmitRequest {
        EmitRequest {
            path: path.into(),
            point: EmitPoint::Nominal,
            value: Ratio::new(num, den),
            unit: unit.into(),
        }
    }

    fn params() -> Vec<Param> {
        vec![
            param_from_request(
                "Plate_Thk",
                &req("acme::Cover.thickness", 127, 20, "mm"),
                6,
                "1/4in",
            )
            .unwrap(),
            param_from_request("Bolts", &req("acme::Cover.bolts", 6, 1, ""), 6, "").unwrap(),
        ]
    }

    // frob:tests crates/cadetta-adapter/src/builtin/json.rs::emit kind=unit
    #[test]
    fn emit_is_byte_identical_golden() {
        let a = emit(&header("blake3:aa"), &params());
        let b = emit(&header("blake3:aa"), &params());
        assert_eq!(a, b);
        let golden = "{\n  \"schema\": \"cadetta.params/1\",\n  \"header\": {\n    \"adapter\": \"json 0.1.0\",\n    \"ir\": \"blake3:aa\",\n    \"bindings\": \"blake3:bb\"\n  },\n  \"params\": [\n    {\n      \"name\": \"Plate_Thk\",\n      \"path\": \"acme::Cover.thickness\",\n      \"value\": \"6.35\",\n      \"unit\": \"mm\",\n      \"note\": \"1/4in\"\n    },\n    {\n      \"name\": \"Bolts\",\n      \"path\": \"acme::Cover.bolts\",\n      \"value\": \"6\"\n    }\n  ]\n}\n";
        assert_eq!(a, golden);
    }

    // frob:tests crates/cadetta-adapter/src/builtin/json.rs::read kind=unit
    #[test]
    fn read_returns_emitted_values() {
        let text = emit(&header("blake3:aa"), &params());
        let units = UnitPolicy {
            native_unit: Some("mm".into()),
            assume_unit: None,
        };
        let got = read(&text, &units).unwrap();
        assert_eq!(got[0].observed.literal, "6.35");
        assert_eq!(got[0].observed.unit.as_deref(), Some("mm"));
        assert!(!got[0].observed.unit_assumed);
        assert_eq!(got[1].observed.literal, "6");
        assert!(got[1].observed.unit_assumed);
    }

    // frob:tests crates/cadetta-adapter/src/builtin/json.rs::same_modulo_header kind=unit
    #[test]
    fn header_is_excluded_from_comparison() {
        let a = emit(&header("blake3:aa"), &params());
        let b = emit(&header("blake3:cc"), &params());
        assert_ne!(a, b);
        assert!(same_modulo_header(&a, &b).unwrap());
        let mut changed = params();
        changed[0].value = "7".into();
        assert!(!same_modulo_header(&a, &emit(&header("blake3:aa"), &changed)).unwrap());
    }

    // frob:tests crates/cadetta-adapter/src/builtin/json.rs::param_from_request kind=unit
    #[test]
    fn inexact_is_q004_and_values_are_strings() {
        let e = param_from_request("x", &req("p", 1, 3, "mm"), 6, "").unwrap_err();
        assert!(matches!(e, JsonError::Policy(PolicyError::Inexact { .. })));
        assert!(emit(&header("h"), &params()).contains("\"value\": \"6.35\""));
    }

    // frob:tests crates/cadetta-adapter/src/builtin/json.rs::read kind=unit
    #[test]
    fn ref_mode_and_bad_schema_rejected() {
        let ref_mode = r#"{"schema":"cadetta.params/1","header":{"adapter":"a","ir":"i","bindings":"b"},
            "params":[{"name":"n","path":"p","value":"1","mode":"ref"}]}"#;
        assert!(matches!(
            read(ref_mode, &UnitPolicy::default()),
            Err(JsonError::Parse(_))
        ));
        let bad =
            r#"{"schema":"nope/1","header":{"adapter":"a","ir":"i","bindings":"b"},"params":[]}"#;
        assert!(matches!(
            read(bad, &UnitPolicy::default()),
            Err(JsonError::Schema(_))
        ));
    }
}
