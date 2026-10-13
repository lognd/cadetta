//! frob:ticket 01M4V0NSCWCW8W0P7DG628M8AP
//! The `expect.json` model (tests/conformance/README.md).

use serde::Deserialize;

/// One expected diagnostic.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ExpectedDiag {
    /// Diagnostic code, e.g. `Q006`.
    pub code: String,
    /// `error`, `warning` or `info`.
    pub severity: String,
    /// Optional canonical path; matched exactly when present.
    #[serde(default)]
    pub path: Option<String>,
}

/// Expectations for one fixture of a pair.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Block {
    /// The closed list of errors and warnings (plus any listed info).
    #[serde(default)]
    pub diagnostics: Vec<ExpectedDiag>,
    /// Evaluation-rule values; not checkable until `eval` exists.
    #[serde(default)]
    pub values: Vec<serde_json::Value>,
    /// Explicit exit status.
    #[serde(default)]
    pub exit: Option<i32>,
}

/// A parsed `NAME.expect.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Expect {
    /// Spec document stem.
    pub doc: String,
    /// `source`, `ir` or `binding`.
    pub kind: String,
    /// Command to run; default `check`.
    #[serde(default)]
    pub entry: Option<String>,
    /// The must-fire block.
    pub must_fire: Block,
    /// The must-not-fire block.
    pub must_not_fire: Block,
}
