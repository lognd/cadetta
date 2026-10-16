//! Import proposal: `build/import/proposal.toml` and `proposal.cdt` (adapters.md 7.1, 7.3).
//!
//! Propose mode turns observed IR into one `[[item]]` per native item, sorted by
//! `file` then `locator`, plus a `.cdt` scaffold. It writes those two files and
//! nothing else, and never touches `.cdt` or bindings outside `build/import/`.
//! Component names come from [`super::naming::component_name`] (7.2).
// frob:ticket 01M4SWMCMY3QH9282ZJ07XBBB7

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path as FsPath, PathBuf};

use cadetta_ir::model::{LocatorKind, LocatorMode, Project, Provenance, Value};
use serde::{Deserialize, Serialize};

use super::geometry::{unit_scale, write_transform};
use super::naming::{MemberNames, component_name, member_name};
use crate::geometry_bind::Mat34;

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
    /// `global | dimension | property | frame | instance`.
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
    /// The part type of an `instance` item (the component the instance places).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
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

fn kind_name(k: LocatorKind) -> &'static str {
    match k {
        LocatorKind::Global => "global",
        LocatorKind::Dimension => "dimension",
        LocatorKind::Property => "property",
        LocatorKind::Frame => "frame",
        LocatorKind::Instance => "instance",
    }
}

/// Why an accepted item carries a reason: its native expression is converted, not copied.
const EXPRESSION_REASON: &str = "native expression converted; review the formula";

/// Converts a native expression to `.cdt` syntax (adapters.md 7.2).
///
/// Each quoted native name (`"B"`) becomes the proposed member path found in `names`
/// (native name to path). Returns `None` when a reference cannot be resolved or quoting is
/// unbalanced, so the caller falls back to the literal value.
pub fn convert_expression(expr: &str, names: &BTreeMap<&str, &str>) -> Option<String> {
    let mut out = String::new();
    let mut rest = expr;
    while let Some(start) = rest.find('"') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after.find('"')?;
        let path = names.get(&after[..end])?;
        out.push_str(path);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    tracing::debug!(%expr, converted = %out, "import: expression converted");
    Some(out.trim().to_owned()).filter(|s| !s.is_empty())
}

/// Builds a proposal from observed IR, one project per native file (adapters.md 7.1).
pub fn propose(adapter: &str, observed: &[Project]) -> Proposal {
    let mut items = Vec::new();
    let mut names: BTreeMap<String, MemberNames> = BTreeMap::new();
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
                let allocated = names
                    .entry(component_name(file))
                    .or_default()
                    .allocate(native_name);
                if allocated.collided {
                    tracing::warn!(native = %native_name, name = %allocated.name, "import: member name collided; suffixed");
                }
                for loc in &binding.locators {
                    let (status, reason) = if loc.mode == LocatorMode::Ref {
                        (
                            Status::Skip,
                            Some("driven (reference) dimension".to_string()),
                        )
                    } else if allocated.collided {
                        (
                            Status::Accept,
                            Some("name collided with an earlier member; suffixed".to_string()),
                        )
                    } else if m.expr.is_some() {
                        (Status::Accept, Some(EXPRESSION_REASON.to_string()))
                    } else {
                        (Status::Accept, None)
                    };
                    let unit_assumed = value.unit_assumed.is_some();
                    let unit = value
                        .unit_assumed
                        .clone()
                        .or_else(|| Some(loc.native_unit.clone()).filter(|u| !u.is_empty()));
                    items.push(Item {
                        file: file.clone(),
                        locator: loc.locator.clone(),
                        kind: kind_name(loc.kind).to_string(),
                        native_name: native_name.clone(),
                        expression: m.expr.clone(),
                        value: value.authored.clone(),
                        unit,
                        unit_assumed,
                        path: allocated.name.clone(),
                        component: component_name(file),
                        part: None,
                        status,
                        reason,
                    });
                }
            }
        }
        propose_geometry(project, &mut names, &mut items);
    }
    items.sort_by(|a, b| (&a.file, &a.locator).cmp(&(&b.file, &b.locator)));
    tracing::info!(adapter, items = items.len(), "import: proposal built");
    Proposal {
        adapter: adapter.to_string(),
        items,
    }
}

/// The native unit of a project: the first one its bindings name, else millimetres.
fn native_unit(project: &Project) -> String {
    project
        .bindings
        .iter()
        .flat_map(|b| &b.locators)
        .map(|l| l.native_unit.as_str())
        .find(|u| !u.is_empty())
        .unwrap_or("mm")
        .to_owned()
}

/// The native unit and its SI length; an unknown unit falls back to millimetres with a warning.
fn unit_and_scale(project: &Project) -> (String, f64) {
    let unit = native_unit(project);
    match unit_scale(&unit) {
        Some(s) => (unit, s),
        None => {
            tracing::warn!(%unit, "import: unknown native unit; reading transforms as mm");
            ("mm".to_owned(), 0.001)
        }
    }
}

fn read_transform(
    rows: &[Vec<Value>],
    scale: f64,
    unit: &str,
) -> Result<super::geometry::Written, String> {
    let m = Mat34::from_observed(rows, scale).map_err(|e| e.to_string())?;
    write_transform(&m, unit).map_err(|e| e.to_string())
}

/// Proposes a datum per observed frame and a flagged place per observed instance (7.2).
// frob:ticket 01M4VB64CJDA2CZ4WM4MNV70G6
fn propose_geometry(
    project: &Project,
    names: &mut BTreeMap<String, MemberNames>,
    items: &mut Vec<Item>,
) {
    let (unit, scale) = unit_and_scale(project);
    for f in &project.frames {
        let component = component_name(&f.document);
        let allocated = names
            .entry(component.clone())
            .or_default()
            .allocate(&f.name);
        let (value, status, reason) = match read_transform(&f.transform, scale, &unit) {
            _ if allocated.name == "origin" => (
                String::new(),
                Status::Skip,
                Some("the origin datum is implicit".to_owned()),
            ),
            Ok(w) => (
                w.expr,
                Status::Accept,
                w.inexact
                    .then(|| "rotation is not a multiple of 90deg; written as read".to_owned()),
            ),
            Err(e) => (String::new(), Status::Skip, Some(e)),
        };
        tracing::debug!(frame = %f.name, datum = %allocated.name, ?status, "import: frame proposed");
        items.push(Item {
            file: f.document.clone(),
            locator: f.name.clone(),
            kind: "frame".to_owned(),
            native_name: f.name.clone(),
            expression: None,
            value,
            unit: Some(unit.clone()),
            unit_assumed: false,
            path: allocated.name,
            component,
            part: None,
            status,
            reason,
        });
    }
    for i in &project.instances {
        let assembly = component_name(&i.document);
        let stem = instance_stem(&i.component);
        let allocated = names
            .entry(format!("asm:{assembly}"))
            .or_default()
            .allocate(&stem);
        let part = component_name(i.reference.as_deref().unwrap_or(&stem));
        let (value, status, reason) = match read_transform(&i.transform, scale, &unit) {
            Ok(w) => (
                w.expr,
                Status::Accept,
                Some(if w.inexact {
                    "placed at the origin with the read transform (rotation as read); re-anchor on a datum of the host"
                } else {
                    "placed at the origin with the read transform; re-anchor on a datum of the host"
                }
                .to_owned()),
            ),
            Err(e) => (String::new(), Status::Skip, Some(e)),
        };
        tracing::debug!(instance = %i.component, part = %allocated.name, ?status, "import: instance proposed");
        items.push(Item {
            file: i.document.clone(),
            locator: i.component.clone(),
            kind: "instance".to_owned(),
            native_name: i.component.clone(),
            expression: None,
            value,
            unit: Some(unit.clone()),
            unit_assumed: false,
            path: allocated.name,
            component: assembly,
            part: Some(part),
            status,
            reason,
        });
    }
}

/// An instance name without its occurrence suffix (`bracket<2>` -> `bracket`).
fn instance_stem(name: &str) -> String {
    let base = name.split('<').next().unwrap_or(name);
    let base = base.trim_end_matches(|c: char| c.is_ascii_digit());
    let base = base.trim_end_matches(['-', ':', '_']);
    let base = if base.is_empty() { name } else { base };
    member_name(base)
}

/// Renders the `.cdt` scaffold: one `model` per file with its accepted items.
pub fn render_cdt(p: &Proposal) -> String {
    let mut out =
        String::from("// Import scaffold; edit proposal.toml, then run `import --accept`.\n");
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
        out.push('\n');
        out.push_str(&render_model(&component_name(file), &items));
    }
    out
}

/// Renders the items of one native file: a `model` for members and frames, then an `assembly`
/// with a `part` and a `place` per instance (7.2).
pub fn render_model(component: &str, items: &[&Item]) -> String {
    let (instances, members): (Vec<&Item>, Vec<&Item>) =
        items.iter().partition(|i| i.kind == "instance");
    let names: BTreeMap<&str, &str> = members
        .iter()
        .filter(|i| i.kind != "frame")
        .map(|i| (i.native_name.as_str(), i.path.as_str()))
        .collect();
    let mut out = String::new();
    if !members.is_empty() || instances.is_empty() {
        out.push_str(&format!("model {component} {{\n"));
        for i in members {
            if i.kind == "frame" {
                out.push_str(&format!("  datum {} = {}\n", i.path, i.value));
                continue;
            }
            let converted = i
                .expression
                .as_deref()
                .and_then(|e| convert_expression(e, &names));
            if let Some(expr) = converted {
                out.push_str(&format!("  {} = {expr}\n", i.path));
                continue;
            }
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
    if !instances.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("assembly {component} {{\n"));
        for i in &instances {
            let part = i.part.as_deref().unwrap_or(&i.component);
            out.push_str(&format!("  part {}: {part}\n", i.path));
        }
        for i in &instances {
            out.push_str(&format!("  place {}.origin at {}\n", i.path, i.value));
        }
        out.push_str("}\n");
    }
    out
}

/// Writes `proposal.toml` and `proposal.cdt` under `root/build/import`, and nothing else.
pub fn write_proposal(root: &FsPath, p: &Proposal) -> Result<Vec<PathBuf>, ProposeError> {
    let dir = root.join(PROPOSAL_DIR);
    fs::create_dir_all(&dir).map_err(|e| ProposeError::Io(dir.clone(), e))?;
    let toml_text = toml::to_string(p).map_err(|e| ProposeError::Serialize(e.to_string()))?;
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
    use cadetta_ir::model::{
        Exact, Locator, Nominal, ObservedFrame, ObservedInstance, PackageId, Value,
    };
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
                implicit: None,
                tolerance: None,
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
        b.push(param("B", 5, unit_assumed, LocatorMode::Value))
            .unwrap();
        b.push(param("A", 10, unit_assumed, LocatorMode::Value))
            .unwrap();
        b.push(param("D1", 3, unit_assumed, LocatorMode::Ref))
            .unwrap();
        b.build()
    }

    // frob:tests convert_expression
    // frob:tests render_model
    #[test]
    fn expression_becomes_member_references() {
        let p = propose("solidworks", &[observed(None)]);
        let a = p.items.iter().find(|i| i.locator == "A").unwrap();
        assert_eq!(a.reason.as_deref(), Some(EXPRESSION_REASON));
        let cdt = render_cdt(&p);
        assert!(cdt.contains("a = b * 2"), "{cdt}");
        let names = BTreeMap::from([("B", "b")]);
        assert_eq!(
            convert_expression("\"B\" * 2", &names).as_deref(),
            Some("b * 2")
        );
        assert_eq!(convert_expression("\"Q\" * 2", &names), None);
        assert_eq!(convert_expression("\"B * 2", &names), None);
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

    fn num_value(n: i64, den: i64) -> Value {
        Value::new(
            Nominal::Exact(Exact(Magnitude::new(Rational::new(n, den).unwrap(), 0))),
            Dim::default(),
            n.to_string(),
        )
    }

    /// A 3x4 transform: integer rotation entries and a translation in millimetres (as metres).
    fn rows(rot: [[i64; 3]; 3], t_mm: [i64; 3]) -> Vec<Vec<Value>> {
        (0..3)
            .map(|r| {
                let mut row: Vec<Value> = rot[r].iter().map(|n| num_value(*n, 1)).collect();
                row.push(num_value(t_mm[r], 1000));
                row
            })
            .collect()
    }

    const IDENT: [[i64; 3]; 3] = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
    const RZ90: [[i64; 3]; 3] = [[0, -1, 0], [1, 0, 0], [0, 0, 1]];

    fn geometry_project() -> Project {
        let pkg = PackageId {
            name: "acme.rover".into(),
            version: "0.0.0".into(),
        };
        let mut b = ObservedBuilder::new(pkg, "solidworks", "drive.SLDASM").unwrap();
        b.push(param("B", 5, None, LocatorMode::Value)).unwrap();
        for (name, rot, t) in [
            ("Mount", RZ90, [14, 0, 0]),
            ("Rail__site", IDENT, [0, -3, 2]),
            ("Origin", IDENT, [0, 0, 0]),
        ] {
            b.push_frame(ObservedFrame {
                name: name.into(),
                document: "rail.SLDPRT".into(),
                transform: rows(rot, t),
            })
            .unwrap();
        }
        b.push_instance(ObservedInstance {
            component: "bracket<1>".into(),
            document: "drive.SLDASM".into(),
            reference: Some("bracket".into()),
            transform: rows(RZ90, [10, 20, 0]),
        })
        .unwrap();
        b.push_instance(ObservedInstance {
            component: "bracket<2>".into(),
            document: "drive.SLDASM".into(),
            reference: Some("bracket".into()),
            transform: rows(IDENT, [0, 0, 5]),
        })
        .unwrap();
        b.build()
    }

    // frob:ticket 01M4VB64CJDA2CZ4WM4MNV70G6
    // frob:tests frames_become_datums
    #[test]
    fn frames_become_datums() {
        let p = propose("solidworks", &[geometry_project()]);
        let frame = |loc: &str| {
            p.items
                .iter()
                .find(|i| i.kind == "frame" && i.locator == loc)
        };
        let mount = frame("Mount").unwrap();
        assert_eq!(mount.component, "Rail");
        assert_eq!(mount.path, "mount");
        assert_eq!(mount.value, "origin.offset(x = 14mm).rotate(z = 90deg)");
        assert_eq!(mount.status, Status::Accept);
        assert_eq!(mount.reason, None);
        let rail = frame("Rail__site").unwrap();
        assert_eq!(rail.path, "rail_site");
        assert_eq!(rail.value, "origin.offset(y = -3mm, z = 2mm)");
        let origin = frame("Origin").unwrap();
        assert_eq!(origin.status, Status::Skip);
        assert!(origin.reason.as_deref().unwrap().contains("implicit"));
        let cdt = render_cdt(&p);
        assert!(cdt.contains("model Rail {"));
        assert!(cdt.contains("  datum mount = origin.offset(x = 14mm).rotate(z = 90deg)\n"));
        assert!(!cdt.contains("datum origin"));
    }

    // frob:tests instances_become_flagged_places
    #[test]
    fn instances_become_flagged_places() {
        let p = propose("solidworks", &[geometry_project()]);
        let inst: Vec<&Item> = p.items.iter().filter(|i| i.kind == "instance").collect();
        assert_eq!(inst.len(), 2);
        assert_eq!(inst[0].locator, "bracket<1>");
        assert_eq!(inst[0].component, "Drive");
        assert_eq!(inst[0].path, "bracket");
        assert_eq!(inst[1].path, "bracket_2");
        assert_eq!(inst[0].part.as_deref(), Some("Bracket"));
        assert_eq!(
            inst[0].value,
            "origin.offset(x = 10mm, y = 20mm).rotate(z = 90deg)"
        );
        assert_eq!(inst[0].status, Status::Accept);
        assert!(inst[0].reason.as_deref().unwrap().contains("re-anchor"));
        let cdt = render_cdt(&p);
        assert!(cdt.contains("assembly Drive {"));
        assert!(cdt.contains("  part bracket: Bracket\n"));
        assert!(cdt.contains(
            "  place bracket.origin at origin.offset(x = 10mm, y = 20mm).rotate(z = 90deg)\n"
        ));
        let text = toml::to_string(&p).unwrap();
        let back: Proposal = toml::from_str(&text).unwrap();
        assert_eq!(back, p);
    }
}
