//! Generated JSON Schemas of the `impact` and `compare` query reports.
//!
//! Specification: `docs/spec/obligations.md` 5. [`impact_schema_json`] and
//! [`compare_schema_json`] render the one definition of each wire shape deterministically
//! (sorted keys, two-space indent, trailing newline); `docs/schemas/impact.json` and
//! `docs/schemas/compare.json` are their committed output, and the tests here fail when a
//! committed file drifts from the definition.
// frob:ticket 01M4ST8FEBCF9PC9SR0ZKTYT70

use serde_json::{Value, json};
use tracing::debug;

/// The `cadetta.impact/1` schema as a JSON value.
pub fn impact_schema() -> Value {
    json!(
    {
      "$id": "cadetta.impact/1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "additionalProperties": false,
      "description": "Output of `cadetta impact PATH --format json`: everything downstream of one path.",
      "properties": {
        "schema": { "const": "cadetta.impact/1" },
        "path": { "description": "The path asked about.", "type": "string" },
        "dependents": {
          "description": "Downstream nodes, nearest first, then sorted by path.",
          "type": "array",
          "items": { "$ref": "#/$defs/Dependent" }
        },
        "obligations": {
          "description": "Obligations that go stale or whose verdict may change.",
          "type": "array",
          "items": { "$ref": "#/$defs/Obligation" }
        }
      },
      "required": ["schema", "path", "dependents", "obligations"],
      "title": "ImpactOutput",
      "type": "object",
      "$defs": {
        "Dependent": {
          "additionalProperties": false,
          "properties": {
            "path": { "type": "string" },
            "kind": { "enum": ["member", "check", "solver"] },
            "via": { "description": "The node it reads that leads back to PATH.", "type": "string" },
            "depth": { "description": "Reads between this node and PATH, at least 1.", "type": "integer", "minimum": 1 },
            "effect": { "enum": ["changes", "stale", "may_flip"] }
          },
          "required": ["path", "kind", "via", "depth", "effect"],
          "type": "object"
        },
        "Obligation": {
          "additionalProperties": false,
          "properties": {
            "id": { "description": "Obligation id, `KIND:PATH`.", "type": "string" },
            "effect": { "enum": ["stale", "may_flip"] }
          },
          "required": ["id", "effect"],
          "type": "object"
        }
      }
    }
    )
}

/// The `cadetta.compare/1` schema as a JSON value.
pub fn compare_schema() -> Value {
    json!(
    {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "title": "Compare",
      "description": "A `cadetta.compare/1` report: one column per implementer of an interface.",
      "type": "object",
      "required": ["schema", "iface", "slots", "columns"],
      "additionalProperties": false,
      "properties": {
        "schema": { "const": "cadetta.compare/1" },
        "iface": { "type": "string" },
        "slots": { "description": "Canonical paths of the slots typed by the interface, sorted.", "type": "array", "items": { "type": "string" } },
        "columns": {
          "description": "Implementers, sorted by component path.",
          "type": "array",
          "items": { "$ref": "#/$defs/Column" }
        }
      },
      "$defs": {
        "Cell": {
          "type": "object",
          "additionalProperties": false,
          "required": ["name", "value", "bound"],
          "properties": {
            "name": { "type": "string" },
            "value": { "description": "Authored value, or null.", "type": ["string", "null"] },
            "bound": { "description": "Bound text such as `<= 3`, or null.", "type": ["string", "null"] }
          }
        },
        "Column": {
          "type": "object",
          "additionalProperties": false,
          "required": ["component", "members", "commercial", "lifecycle", "slots", "verdicts"],
          "properties": {
            "component": { "type": "string" },
            "members": { "type": "array", "items": { "$ref": "#/$defs/Cell" } },
            "commercial": { "type": "array", "items": { "$ref": "#/$defs/Cell" } },
            "lifecycle": { "type": ["string", "null"] },
            "slots": { "type": "array", "items": { "type": "string" } },
            "verdicts": {
              "type": "array",
              "items": {
                "type": "object",
                "additionalProperties": false,
                "required": ["slot", "check", "verdict"],
                "properties": {
                  "slot": { "type": "string" },
                  "check": { "type": "string" },
                  "verdict": { "enum": ["pass", "fail", "unknown", "not evaluated"] }
                }
              }
            }
          }
        }
      }
    }
    )
}

/// Render `schema` with sorted keys, two-space indent and a trailing newline.
fn render(schema: &Value) -> String {
    let mut text = serde_json::to_string_pretty(schema).expect("a schema serializes");
    text.push('\n');
    text
}

/// The text of `docs/schemas/impact.json`.
pub fn impact_schema_json() -> String {
    debug!("rendering impact schema");
    render(&impact_schema())
}

/// The text of `docs/schemas/compare.json`.
pub fn compare_schema_json() -> String {
    debug!("rendering compare schema");
    render(&compare_schema())
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:ticket 01M4ST8FEBCF9PC9SR0ZKTYT70
    // frob:tests crates/cadetta-obligations/src/schema.rs::impact_schema_json kind=unit
    #[test]
    fn impact_file_matches_definition() {
        let on_disk = include_str!("../../../docs/schemas/impact.json");
        assert_eq!(
            on_disk,
            impact_schema_json(),
            "docs/schemas/impact.json is stale; replace it with impact_schema_json()"
        );
    }

    // frob:ticket 01M4ST8FEBCF9PC9SR0ZKTYT70
    // frob:tests crates/cadetta-obligations/src/schema.rs::compare_schema_json kind=unit
    #[test]
    fn compare_file_matches_definition() {
        let on_disk = include_str!("../../../docs/schemas/compare.json");
        assert_eq!(
            on_disk,
            compare_schema_json(),
            "docs/schemas/compare.json is stale; replace it with compare_schema_json()"
        );
    }
}
