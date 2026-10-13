//! The SARIF 2.1.0 renderer for code scanning (lints.md 1.2).
//!
//! Pure: it returns a `String` of pretty JSON. Rules come from the codes
//! present in the run, sorted. Waived diagnostics are omitted (suppressed
//! diagnostics appear in JSON only). Native locations keep their adapter and
//! locator in `properties`; lock locations map to `cadetta.lock`.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::diag::{Diagnostic, Location, Position, Range, Severity};

/// The SARIF schema URI.
const SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";

/// The SARIF result level for a severity.
fn level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info | Severity::Hint => "note",
    }
}

/// A SARIF region from 1-based start and end positions.
fn region(start: Position, end: Position) -> Value {
    json!({
        "startLine": start.line,
        "startColumn": start.col,
        "endLine": end.line,
        "endColumn": end.col,
    })
}

/// Map a diagnostic location to a SARIF location object.
fn location(loc: &Location) -> Value {
    match loc {
        Location::Source { file, start, end } => json!({
            "physicalLocation": {
                "artifactLocation": { "uri": file },
                "region": region(*start, *end),
            }
        }),
        Location::Native {
            adapter,
            file,
            locator,
        } => json!({
            "physicalLocation": {
                "artifactLocation": { "uri": file },
                "region": { "startLine": 1, "startColumn": 1 },
            },
            "properties": { "adapter": adapter, "locator": locator },
        }),
        Location::Lock { line } => json!({
            "physicalLocation": {
                "artifactLocation": { "uri": "cadetta.lock" },
                "region": { "startLine": line, "startColumn": 1 },
            }
        }),
    }
}

/// Map a fix edit range to a SARIF deleted region.
fn range(r: &Range) -> Value {
    region(r.start, r.end)
}

/// Render one diagnostic as a SARIF result.
fn result(d: &Diagnostic, rule_index: usize) -> Value {
    tracing::trace!(code = %d.code, "render: sarif result");
    let mut r = json!({
        "ruleId": d.code,
        "ruleIndex": rule_index,
        "level": level(d.severity),
        "message": { "text": d.message },
    });
    let obj = r.as_object_mut().expect("json object");
    if let Some(loc) = &d.location {
        let mut l = location(loc);
        if let Some(path) = &d.path {
            l["logicalLocations"] = json!([{ "fullyQualifiedName": path }]);
        }
        obj.insert("locations".into(), json!([l]));
    } else if let Some(path) = &d.path {
        obj.insert(
            "locations".into(),
            json!([{ "logicalLocations": [{ "fullyQualifiedName": path }] }]),
        );
    }
    let related: Vec<Value> = d
        .notes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mut v = n.location.as_ref().map_or_else(|| json!({}), location);
            v["id"] = json!(i);
            v["message"] = json!({ "text": n.message });
            v
        })
        .collect();
    if !related.is_empty() {
        obj.insert("relatedLocations".into(), Value::Array(related));
    }
    let fixes: Vec<Value> = d
        .fixes
        .iter()
        .map(|f| {
            let changes: Vec<Value> = f
                .edits
                .iter()
                .map(|e| {
                    json!({
                        "artifactLocation": { "uri": e.file },
                        "replacements": [{
                            "deletedRegion": range(&e.range),
                            "insertedContent": { "text": e.text },
                        }],
                    })
                })
                .collect();
            json!({ "description": { "text": f.title }, "artifactChanges": changes })
        })
        .collect();
    if !fixes.is_empty() {
        obj.insert("fixes".into(), Value::Array(fixes));
    }
    if let Some(alt) = &d.alt {
        obj.insert("properties".into(), json!({ "alt": alt }));
    }
    r
}

/// Render a run of diagnostics as one SARIF 2.1.0 log, in the order given.
pub fn render_all(diags: &[Diagnostic]) -> String {
    let live: Vec<&Diagnostic> = diags.iter().filter(|d| d.waived.is_none()).collect();
    tracing::debug!(count = live.len(), "render: sarif run");
    let codes: Vec<&str> = live
        .iter()
        .map(|d| d.code.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let rules: Vec<Value> = codes
        .iter()
        .map(|c| json!({ "id": c, "name": c, "shortDescription": { "text": c } }))
        .collect();
    let results: Vec<Value> = live
        .iter()
        .map(|d| {
            let idx = codes.iter().position(|c| *c == d.code).unwrap_or(0);
            result(d, idx)
        })
        .collect();
    let log = json!({
        "$schema": SCHEMA,
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": {
                "name": "cadetta",
                "version": env!("CARGO_PKG_VERSION"),
                "informationUri": "https://github.com/lognd/cadetta",
                "rules": rules,
            }},
            "results": results,
        }],
    });
    let mut out = serde_json::to_string_pretty(&log).expect("sarif serializes");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::{Fix, Note, Waived};

    fn parse(diags: &[Diagnostic]) -> Value {
        serde_json::from_str(&render_all(diags)).expect("valid json")
    }

    // frob:ticket 01M4SWKA4EXSKCGVPWK9026WEG
    // frob:tests crates/cadetta-lint/src/render/sarif.rs::render_all kind=unit
    #[test]
    fn log_shape_rules_and_results() {
        let mut d = Diagnostic::new("C001", Severity::Error, "bad");
        d.path = Some("acme::Rover.load_ok".into());
        d.alt = Some("wheel=A".into());
        d.location = Some(Location::Source {
            file: "cad/rover.cdt".into(),
            start: Position { line: 41, col: 3 },
            end: Position { line: 41, col: 25 },
        });
        d.notes.push(Note {
            message: "n".into(),
            location: None,
        });
        d.fixes.push(Fix {
            title: "t".into(),
            edits: vec![],
        });
        let h = Diagnostic::new("N001", Severity::Hint, "h");
        let v = parse(&[d.clone(), h, d]);
        assert_eq!(v["version"], "2.1.0");
        let run = &v["runs"][0];
        assert_eq!(run["tool"]["driver"]["rules"].as_array().unwrap().len(), 2);
        let res = run["results"].as_array().unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0]["ruleId"], "C001");
        assert_eq!(res[0]["ruleIndex"], 0);
        assert_eq!(res[1]["level"], "note");
        assert_eq!(res[1]["ruleIndex"], 1);
        let loc = &res[0]["locations"][0];
        assert_eq!(
            loc["physicalLocation"]["artifactLocation"]["uri"],
            "cad/rover.cdt"
        );
        assert_eq!(loc["physicalLocation"]["region"]["startLine"], 41);
        assert_eq!(
            loc["logicalLocations"][0]["fullyQualifiedName"],
            "acme::Rover.load_ok"
        );
        assert_eq!(res[0]["properties"]["alt"], "wheel=A");
        assert_eq!(res[0]["relatedLocations"][0]["message"]["text"], "n");
        assert_eq!(res[0]["fixes"][0]["description"]["text"], "t");
    }

    // frob:ticket 01M4SWKA4EXSKCGVPWK9026WEG
    // frob:tests crates/cadetta-lint/src/render/sarif.rs::location kind=unit
    #[test]
    fn native_and_lock_locations_keep_path() {
        let mut n = Diagnostic::new("R001", Severity::Warning, "drift");
        n.location = Some(Location::Native {
            adapter: "freecad".into(),
            file: "cad/a.FCStd".into(),
            locator: "Sketch.Constraints[3]".into(),
        });
        let mut l = Diagnostic::new("O001", Severity::Warning, "stale");
        l.location = Some(Location::Lock { line: 7 });
        let mut w = Diagnostic::new("N001", Severity::Hint, "waived");
        w.waived = Some(Waived {
            waiver: "w".into(),
            because: "b".into(),
        });
        let v = parse(&[n, l, w]);
        let res = v["runs"][0]["results"].as_array().unwrap();
        assert_eq!(res.len(), 2, "waived omitted");
        let nl = &res[0]["locations"][0];
        assert_eq!(
            nl["physicalLocation"]["artifactLocation"]["uri"],
            "cad/a.FCStd"
        );
        assert_eq!(nl["properties"]["locator"], "Sketch.Constraints[3]");
        let ll = &res[1]["locations"][0]["physicalLocation"];
        assert_eq!(ll["artifactLocation"]["uri"], "cadetta.lock");
        assert_eq!(ll["region"]["startLine"], 7);
    }
}
