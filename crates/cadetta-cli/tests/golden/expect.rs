//! frob:ticket 01M4SXSGQQHR6RMBNJQJ3392AZ
//! The `expected.toml` model of a golden case (tests/golden/README.md).

use std::path::Path;

use serde::Deserialize;

/// One golden case's expectations.
#[derive(Debug, Deserialize)]
pub struct Expected {
    /// The `[case]` table.
    pub case: CaseInfo,
    /// Expected evaluated values.
    #[serde(default, rename = "value")]
    pub values: Vec<ValueExp>,
    /// Expected check, fit, bound and require verdicts.
    #[serde(default, rename = "verdict")]
    pub verdicts: Vec<VerdictExp>,
    /// Expected diagnostics (closed for errors and warnings).
    #[serde(default, rename = "diagnostic")]
    pub diagnostics: Vec<DiagnosticExp>,
    /// Expected artifacts.
    #[serde(default, rename = "artifact")]
    pub artifacts: Vec<ArtifactExp>,
}

/// The `[case]` table.
#[derive(Debug, Deserialize)]
pub struct CaseInfo {
    /// The spec draft the case is written against.
    pub spec: String,
    /// `hand-evaluated` or `confirmed`.
    pub status: String,
}

/// One `[[value]]`: a quantity pinned at an instance path.
#[derive(Debug, Deserialize)]
pub struct ValueExp {
    /// Canonical instance path.
    pub path: String,
    /// Nominal, as a source literal.
    pub nominal: String,
    /// Lower limit after tolerance.
    pub min: Option<String>,
    /// Upper limit after tolerance.
    pub max: Option<String>,
    /// Variant key `SLOT=ALT`.
    pub variant: Option<String>,
}

/// One `[[verdict]]`.
#[derive(Debug, Deserialize)]
pub struct VerdictExp {
    /// Canonical check path.
    pub path: String,
    /// `holds`, `violates` or `undecided`.
    pub result: String,
    /// Expected conservative flag, when listed.
    pub conservative: Option<bool>,
    /// Variant key `SLOT=ALT`.
    pub variant: Option<String>,
}

/// One `[[diagnostic]]`.
#[derive(Debug, Deserialize)]
pub struct DiagnosticExp {
    /// Diagnostic code.
    pub code: String,
    /// Subject path, when listed.
    pub path: Option<String>,
    /// Variant key, when listed.
    pub variant: Option<String>,
}

/// One `[[artifact]]`.
#[derive(Debug, Deserialize)]
pub struct ArtifactExp {
    /// The command, run at the case root.
    pub command: String,
    /// The file it writes, relative to the case root.
    pub produces: String,
    /// The expected file, relative to the case root.
    pub expected: String,
    /// Line patterns (regular expressions) masked in both files.
    #[serde(default)]
    pub mask: Vec<String>,
}

/// Parse `expected.toml` text.
pub fn parse(text: &str) -> Result<Expected, String> {
    toml::from_str(text).map_err(|e| format!("expected.toml: {e}"))
}

/// Read and parse the `expected.toml` of a case directory.
pub fn load(case: &Path) -> Result<Expected, String> {
    let path = case.join("expected.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text)
}
