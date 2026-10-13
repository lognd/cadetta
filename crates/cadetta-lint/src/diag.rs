//! The `cadetta.diag/1` diagnostic record (lints.md 1.1): one shape for
//! the compiler, lints, solvers and adapters, with deterministic order.

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

/// The schema tag every record carries.
pub const SCHEMA: &str = "cadetta.diag/1";

/// The pseudo-file lock locations sort under.
const LOCK_FILE: &str = "cadetta.lock";

/// Effective severity of a diagnostic after precedence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Fails the run.
    Error,
    /// Reported, does not fail the run by default.
    Warning,
    /// Informational.
    Info,
    /// Suggestion only.
    Hint,
}

/// A 1-based line and column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Position {
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub col: u32,
}

/// A start/end range within one file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Range {
    /// Start of the range.
    pub start: Position,
    /// End of the range.
    pub end: Position,
}

/// Where to look: a source span, a native CAD locator or a lock line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Location {
    /// A span in a `.cdt` file (required for every design-IR diagnostic).
    Source {
        /// Path of the source file.
        file: String,
        /// Start of the span.
        start: Position,
        /// End of the span.
        end: Position,
    },
    /// A locator inside a native CAD file (R rules, adapters).
    Native {
        /// Adapter name.
        adapter: String,
        /// Native file path.
        file: String,
        /// Adapter-defined locator.
        locator: String,
    },
    /// A line of the lock file (O, W and sync records).
    Lock {
        /// 1-based line.
        line: u32,
    },
}

/// A related location or fact attached to a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Note {
    /// The fact, one sentence.
    pub message: String,
    /// Where it applies, if anywhere.
    pub location: Option<Location>,
}

/// One text edit of a suggested fix.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Edit {
    /// File to edit.
    pub file: String,
    /// Range to replace.
    pub range: Range,
    /// Replacement text.
    pub text: String,
}

/// A suggested fix; never applied without a flag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Fix {
    /// Short description of the fix.
    pub title: String,
    /// The edits that make it up.
    pub edits: Vec<Edit>,
}

/// The reason a diagnostic was suppressed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Waived {
    /// Identifier of the waiver.
    pub waiver: String,
    /// The recorded justification.
    pub because: String,
}

/// A `cadetta.diag/1` record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    /// Always `cadetta.diag/1`.
    pub schema: String,
    /// Catalog code.
    pub code: String,
    /// Effective severity.
    pub severity: Severity,
    /// Canonical path of the subject, or null.
    pub path: Option<String>,
    /// `SLOT=ALTERNATE` under variant evaluation, else null.
    pub alt: Option<String>,
    /// One sentence, no trailing period.
    pub message: String,
    /// Where to look, or null.
    pub location: Option<Location>,
    /// Related locations and facts.
    pub notes: Vec<Note>,
    /// Suggested edits.
    pub fixes: Vec<Fix>,
    /// Null, or why the diagnostic is suppressed.
    pub waived: Option<Waived>,
}

impl Diagnostic {
    /// A bare record with the schema tag set and every optional field empty.
    pub fn new(code: impl Into<String>, severity: Severity, message: impl Into<String>) -> Self {
        let code = code.into();
        tracing::trace!(%code, ?severity, "diag: new record");
        Self {
            schema: SCHEMA.to_owned(),
            code,
            severity,
            path: None,
            alt: None,
            message: message.into(),
            location: None,
            notes: Vec::new(),
            fixes: Vec::new(),
            waived: None,
        }
    }

    /// The (file, line, code, path) key that orders a run (lints.md 1.1).
    pub fn sort_key(&self) -> (&str, u32, &str, &str) {
        let (file, line) = match &self.location {
            Some(Location::Source { file, start, .. }) => (file.as_str(), start.line),
            Some(Location::Native { file, .. }) => (file.as_str(), 0),
            Some(Location::Lock { line }) => (LOCK_FILE, *line),
            None => ("", 0),
        };
        (file, line, &self.code, self.path.as_deref().unwrap_or(""))
    }
}

/// Sort a run into the deterministic (file, line, code, path) order.
pub fn sort(diags: &mut [Diagnostic]) {
    tracing::debug!(count = diags.len(), "diag: sorting run");
    diags.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
}

/// The JSON Schema of [`Diagnostic`], pretty-printed with a trailing newline.
pub fn schema_json() -> String {
    let mut text = serde_json::to_string_pretty(&schema_for!(Diagnostic))
        .expect("a schema is always serializable");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn src(file: &str, line: u32) -> Option<Location> {
        let p = Position { line, col: 1 };
        Some(Location::Source {
            file: file.into(),
            start: p,
            end: p,
        })
    }

    fn sample() -> Diagnostic {
        let mut d = Diagnostic::new(
            "C001",
            Severity::Error,
            "false for every value in tolerance",
        );
        d.path = Some("acme.rover::Rover.load_ok".into());
        d.alt = Some("wheel=Colson2Series".into());
        d.location = src("cad/rover.cdt", 41);
        d.notes.push(Note {
            message: "wheel.mass = 1.2kg".into(),
            location: None,
        });
        d
    }

    // frob:ticket PE4EC23
    // frob:tests cadetta-lint::diag::serialize_has_all_fields
    #[test]
    fn serialize_has_all_fields() {
        let v = serde_json::to_value(sample()).unwrap();
        let obj = v.as_object().unwrap();
        for k in [
            "schema", "code", "severity", "path", "alt", "message", "location", "notes", "fixes",
            "waived",
        ] {
            assert!(obj.contains_key(k), "missing {k}");
        }
        assert_eq!(obj["schema"], "cadetta.diag/1");
        assert_eq!(obj["severity"], "error");
        assert!(obj["waived"].is_null());
        let bare = serde_json::to_value(Diagnostic::new("N001", Severity::Hint, "m")).unwrap();
        assert_eq!(bare.as_object().unwrap().len(), 10);
        let back: Diagnostic = serde_json::from_value(v).unwrap();
        assert_eq!(back, sample());
    }

    // frob:ticket PE4EC23
    // frob:tests cadetta-lint::diag::location_kinds
    #[test]
    fn location_kinds() {
        let s = serde_json::to_value(src("a.cdt", 2).unwrap()).unwrap();
        assert_eq!(s["kind"], "source");
        assert_eq!(s["start"]["line"], 2);
        assert!(s["end"]["col"].is_number());
        let n = serde_json::to_value(Location::Native {
            adapter: "freecad".into(),
            file: "a.FCStd".into(),
            locator: "Sketch/Len".into(),
        })
        .unwrap();
        assert_eq!(n["kind"], "native");
        assert_eq!(n["locator"], "Sketch/Len");
        let l = serde_json::to_value(Location::Lock { line: 7 }).unwrap();
        assert_eq!(l, serde_json::json!({"kind": "lock", "line": 7}));
        let bad = serde_json::from_str::<Location>(r#"{"kind":"other"}"#);
        assert!(bad.is_err());
    }

    // frob:ticket PE4EC23
    // frob:tests cadetta-lint::diag::sorts_by_file_line_code_path
    #[test]
    fn sorts_by_file_line_code_path() {
        let mk = |file: &str, line, code: &str, path: &str| {
            let mut d = Diagnostic::new(code, Severity::Warning, "m");
            d.location = src(file, line);
            d.path = Some(path.into());
            d
        };
        let mut run = vec![
            mk("b.cdt", 1, "A001", "x"),
            mk("a.cdt", 9, "A001", "x"),
            mk("a.cdt", 2, "C002", "x"),
            mk("a.cdt", 2, "C001", "z"),
            mk("a.cdt", 2, "C001", "y"),
        ];
        sort(&mut run);
        let keys: Vec<_> = run
            .iter()
            .map(|d| {
                (
                    d.sort_key().0.to_owned(),
                    d.sort_key().1,
                    d.code.clone(),
                    d.path.clone().unwrap(),
                )
            })
            .collect();
        let expect = [
            ("a.cdt", 2, "C001", "y"),
            ("a.cdt", 2, "C001", "z"),
            ("a.cdt", 2, "C002", "x"),
            ("a.cdt", 9, "A001", "x"),
            ("b.cdt", 1, "A001", "x"),
        ];
        for (k, e) in keys.iter().zip(expect) {
            assert_eq!((k.0.as_str(), k.1, k.2.as_str(), k.3.as_str()), e);
        }
    }

    // frob:ticket PE4EC23
    // frob:tests cadetta-lint::diag::schema_file_matches_type
    #[test]
    fn schema_file_matches_type() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/schemas/diag.json");
        let generated = schema_json();
        if std::env::var_os("CADETTA_UPDATE_SCHEMA").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &generated).unwrap();
        }
        let on_disk = std::fs::read_to_string(&path).expect("docs/schemas/diag.json exists");
        assert_eq!(
            on_disk, generated,
            "regenerate with CADETTA_UPDATE_SCHEMA=1"
        );
    }

    // frob:ticket PE4EC23
    // frob:tests cadetta-lint::diag::foreign_records_roundtrip_unchanged
    #[test]
    fn foreign_records_roundtrip_unchanged() {
        // A solver/adapter response record parses into the same type and
        // re-serializes to identical JSON (no field is dropped or renamed).
        let wire = r#"{"schema":"cadetta.diag/1","code":"R004","severity":"warning","path":null,"alt":null,"message":"feature failed","location":{"kind":"native","adapter":"freecad","file":"a.FCStd","locator":"Pad"},"notes":[],"fixes":[{"title":"t","edits":[{"file":"f","range":{"start":{"line":1,"col":1},"end":{"line":1,"col":2}},"text":"x"}]}],"waived":{"waiver":"W1","because":"ok"}}"#;
        let d: Diagnostic = serde_json::from_str(wire).unwrap();
        let again = serde_json::to_value(&d).unwrap();
        assert_eq!(
            again,
            serde_json::from_str::<serde_json::Value>(wire).unwrap()
        );
    }
}
