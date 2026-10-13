//! Import proposal: `build/import/proposal.toml` and `proposal.cdt` (adapters.md 7.1, 7.3).
//!
//! Propose mode turns observed IR into one `[[item]]` per native item, sorted by
//! `file` then `locator`, plus a `.cdt` scaffold. It writes those two files and
//! nothing else, and never touches `.cdt` or bindings outside `build/import/`.
//! Naming here is the minimal placeholder (observed member name, UpperCamel file
//! stem); the full 7.2 rules belong to the naming ticket.
// frob:ticket 01M4SWMCMY3QH9282ZJ07XBBB7

use std::fmt;
use std::fs;
use std::path::{Path as FsPath, PathBuf};

use cadetta_ir::model::{LocatorKind, LocatorMode, Project, Provenance};
use serde::{Deserialize, Serialize};

/// Directory of the proposal files, relative to the package root.
pub const PROPOSAL_DIR: &str = "build/import";
/// File name of the editable proposal.
pub const PROPOSAL_TOML: &str = "proposal.toml";
/// File name of the `.cdt` scaffold.
pub const PROPOSAL_CDT: &str = "proposal.cdt";

/// Review status of one proposed item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Import as proposed.
    Accept,
    /// Import under the edited `path`.
    Rename,
    /// Do not import; `reason` says why.
    Skip,
}

/// One native item in the proposal (adapters.md 7.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// The native file.
    pub file: String,
    /// The native locator.
    pub locator: String,
    /// `global | dimension | property | frame`.
    pub kind: String,
    /// The native name.
    pub native_name: String,
    /// The native expression, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    /// The value as a source literal.
    pub value: String,
    /// The unit recorded for the value, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// The unit was assumed (adapter native unit or `--assume-unit`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unit_assumed: bool,
    /// The proposed canonical path.
    pub path: String,
    /// The proposed component type.
    #[serde(rename = "type")]
    pub component: String,
    /// Review status.
    pub status: Status,
    /// Why the item is skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The whole proposal: the adapter and its items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    /// The adapter id that read the files.
    pub adapter: String,
    /// Items sorted by `file` then `locator`.
    #[serde(rename = "item")]
    pub items: Vec<Item>,
}

/// A failure writing the proposal.
#[derive(Debug)]
pub enum ProposeError {
    /// Filesystem failure.
    Io(PathBuf, std::io::Error),
    /// The proposal could not be serialised.
    Serialize(String),
}

impl fmt::Display for ProposeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(p, e) => write!(f, "{}: {e}", p.display()),
            Self::Serialize(e) => write!(f, "proposal.toml: {e}"),
        }
    }
}

impl std::error::Error for ProposeError {}

/// Upper-camel component name from a file stem (`cover_plate.SLDPRT` -> `CoverPlate`).
fn upper_camel(file: &str) -> String {
    let stem = FsPath::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file);
    let s: String = stem
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut cs = w.chars();
            let first = cs.next().map(|c| c.to_ascii_uppercase());
            first.into_iter().chain(cs).collect::<String>()
        })
        .collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("C{s}")
    } else {
        s
    }
}

fn kind_name(k: LocatorKind) -> &'static str {
    match k {
        LocatorKind::Global => "global",
        LocatorKind::Dimension => "dimension",
        LocatorKind::Property => "property",
        LocatorKind::Frame => "frame",
    }
}

/// Builds a proposal from observed IR, one project per native file (adapters.md 7.1).
pub fn propose(adapter: &str, observed: &[Project]) -> Proposal {
    let mut items = Vec::new();
    for project in observed {
        for comp in &project.components {
            for m in &comp.members {
                let Some(value) = &m.value else { continue };
                let Provenance::Imported {
                    file, native_name, ..
                } = &m.provenance
                else {
                    continue;
                };
                let Some(binding) = project.bindings.iter().find(|b| b.path == m.path) else {
                    tracing::warn!(native = %native_name, "import: member has no binding; skipped");
                    continue;
                };
                for loc in &binding.locators {
                    let (status, reason) = if loc.mode == LocatorMode::Ref {
                        (
                            Status::Skip,
                            Some("driven (reference) dimension".to_string()),
                        )
                    } else {
                        (Status::Accept, None)
                    };
                    let unit_assumed = value.unit_assumed.is_some();
                    let unit = value
                        .unit_assumed
                        .clone()
                        .or_else(|| Some(loc.native_unit.clone()).filter(|u| !u.is_empty()));
                    let path = m.path.to_string();
                    let name = path.rsplit('.').next().unwrap_or(&path).to_string();
                    items.push(Item {
                        file: file.clone(),
                        locator: loc.locator.clone(),
                        kind: kind_name(loc.kind).to_string(),
                        native_name: native_name.clone(),
                        expression: m.expr.clone(),
                        value: value.authored.clone(),
                        unit,
                        unit_assumed,
                        path: name,
                        component: upper_camel(file),
                        status,
                        reason,
                    });
                }
            }
        }
    }
    items.sort_by(|a, b| (&a.file, &a.locator).cmp(&(&b.file, &b.locator)));
    tracing::info!(adapter, items = items.len(), "import: proposal built");
    Proposal {
        adapter: adapter.to_string(),
        items,
    }
}

/// Renders the `.cdt` scaffold: one `model` per file with its accepted items.
pub fn render_cdt(p: &Proposal) -> String {
    let mut out = String::from("// Import scaffold; edit proposal.toml, then run `import --accept`.\n");
    let mut files: Vec<&str> = p.items.iter().map(|i| i.file.as_str()).collect();
    files.dedup();
    for file in files {
        let items: Vec<&Item> = p
            .items
            .iter()
            .filter(|i| i.file == file && i.status != Status::Skip)
            .collect();
        if items.is_empty() {
            continue;
        }
        out.push_str(&format!("\nmodel {} {{\n", upper_camel(file)));
        for i in items {
            let unit = i.unit.as_deref().unwrap_or("");
            let lit = if unit.is_empty() || i.value.ends_with(|c: char| c.is_ascii_alphabetic()) {
                i.value.clone()
            } else {
                format!("{}{unit}", i.value)
            };
            out.push_str(&format!("  {} = {lit}\n", i.path));
        }
        out.push_str("}\n");
    }
    out
}

/// Writes `proposal.toml` and `proposal.cdt` under `root/build/import`, and nothing else.
pub fn write_proposal(root: &FsPath, p: &Proposal) -> Result<Vec<PathBuf>, ProposeError> {
    let dir = root.join(PROPOSAL_DIR);
    fs::create_dir_all(&dir).map_err(|e| ProposeError::Io(dir.clone(), e))?;
    let toml_text =
        toml::to_string(p).map_err(|e| ProposeError::Serialize(e.to_string()))?;
    let files = [
        (dir.join(PROPOSAL_TOML), toml_text),
        (dir.join(PROPOSAL_CDT), render_cdt(p)),
    ];
    let mut written = Vec::new();
    for (path, text) in files {
        fs::write(&path, text).map_err(|e| ProposeError::Io(path.clone(), e))?;
        tracing::info!(path = %path.display(), "import: proposal written");
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{Exact, Locator, Nominal, PackageId};
    use cadetta_ir::observed::{ObservedBuilder, ObservedParam};
    use cadetta_units::{Dim, Magnitude, Rational};

    fn param(name: &str, n: i64, unit_assumed: Option<&str>, mode: LocatorMode) -> ObservedParam {
        ObservedParam {
            native_name: name.into(),
            locator: Locator {
                locator: name.into(),
                kind: LocatorKind::Global,
                mode,
                native_unit: "mm".into(),
                ref_to: None,
                for_alternate: None,
                config: None,
                sign: None,
                component: None,
            },
            nominal: Nominal::Exact(Exact(Magnitude::new(Rational::new(n, 1).unwrap(), 0))),
            dim: Dim::default(),
            text: n.to_string(),
            expr: (name == "A").then(|| "\"B\" * 2".to_string()),
            unit_assumed: unit_assumed.map(str::to_owned),
            inexact_reason: None,
        }
    }

    fn observed(unit_assumed: Option<&str>) -> Project {
        let pkg = PackageId {
            name: "acme.rover".into(),
            version: "0.0.0".into(),
        };
        let mut b = ObservedBuilder::new(pkg, "solidworks", "cover_plate.SLDPRT").unwrap();
        b.push(param("B", 5, unit_assumed, LocatorMode::Value)).unwrap();
        b.push(param("A", 10, unit_assumed, LocatorMode::Value)).unwrap();
        b.push(param("D1", 3, unit_assumed, LocatorMode::Ref)).unwrap();
        b.build()
    }

    // frob:tests proposal_written_alone
    #[test]
    fn proposal_written_alone() {
        let root = std::env::temp_dir().join(format!("cdt-propose-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let p = propose("solidworks", &[observed(None)]);
        let written = write_proposal(&root, &p).unwrap();
        assert_eq!(written.len(), 2);
        let mut names: Vec<_> = fs::read_dir(root.join(PROPOSAL_DIR))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["proposal.cdt", "proposal.toml"]);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        let text = fs::read_to_string(root.join(PROPOSAL_DIR).join(PROPOSAL_TOML)).unwrap();
        let back: Proposal = toml::from_str(&text).unwrap();
        assert_eq!(back, p);
        let locs: Vec<_> = back.items.iter().map(|i| i.locator.as_str()).collect();
        assert_eq!(locs, ["A", "B", "D1"]);
        assert_eq!(back.items[0].expression.as_deref(), Some("\"B\" * 2"));
        assert_eq!(back.items[2].status, Status::Skip);
        let cdt = fs::read_to_string(root.join(PROPOSAL_DIR).join(PROPOSAL_CDT)).unwrap();
        assert!(cdt.contains("model CoverPlate {"));
        assert!(!cdt.contains("D1"));
        let _ = fs::remove_dir_all(&root);
    }

    // frob:tests assume_unit_recorded
    #[test]
    fn assume_unit_recorded() {
        let p = propose("solidworks", &[observed(Some("in"))]);
        assert!(p.items.iter().all(|i| i.unit_assumed));
        assert_eq!(p.items[0].unit.as_deref(), Some("in"));
        let text = toml::to_string(&p).unwrap();
        assert!(text.contains("unit_assumed = true"));
        let q = propose("solidworks", &[observed(None)]);
        assert!(q.items.iter().all(|i| !i.unit_assumed));
    }
}
