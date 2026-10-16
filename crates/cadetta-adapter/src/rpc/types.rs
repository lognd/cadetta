//! Wire types of the adapter protocol (adapters.md 5); the JSON Schema in
//! `docs/schemas/adapter-protocol.json` is generated from these.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The protocol identifier sent in `initialize`.
pub const PROTOCOL: &str = "cadetta.adapter/1";

/// Diagnostic code for an adapter whose version does not match `cadetta.toml` (R008).
pub const R008: &str = "R008";

/// A `cadetta.diag/1` record carried verbatim; the schema is `diag.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Value")]
pub struct DiagRecord(pub Value);

impl TryFrom<Value> for DiagRecord {
    type Error = String;

    fn try_from(v: Value) -> Result<Self, String> {
        let tag = v.get("schema").and_then(Value::as_str);
        if tag != Some("cadetta.diag/1") {
            return Err(format!(
                "diagnostic schema tag is {tag:?}, want cadetta.diag/1"
            ));
        }
        for key in ["code", "severity", "message"] {
            if !v.get(key).is_some_and(Value::is_string) {
                return Err(format!("diagnostic lacks string field `{key}`"));
            }
        }
        Ok(Self(v))
    }
}

impl JsonSchema for DiagRecord {
    fn schema_name() -> Cow<'static, str> {
        "DiagRecord".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({ "$ref": "diag.json" })
    }
}

/// An adapter capability name as declared at `initialize`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityName {
    /// IR to file.
    Emit,
    /// File to observed IR.
    Read,
    /// IR to and from a live tool.
    Drive,
}

/// An optional adapter feature declared at `initialize` (adapters.md 5).
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    /// Reads frame transforms.
    Frames,
    /// Reads component transforms and reference properties.
    Instances,
    /// Creates and drives coordinate systems.
    CreateFrames,
    /// Reads and writes dimension tolerances (adapters.md 3.6).
    Tolerances,
    /// Renames globals in place.
    RenameGlobals,
}

/// A dimension tolerance on the wire, as native-unit literals (adapters.md 3.6).
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "form", rename_all = "snake_case", deny_unknown_fields)]
pub enum DimTolerance {
    /// `[+-t]`: the value plus or minus `t`.
    Symmetric {
        /// Half-width `t`.
        t: String,
    },
    /// Asymmetric deviations from the value.
    Bilateral {
        /// Upper deviation (signed).
        upper: String,
        /// Lower deviation (signed).
        lower: String,
    },
    /// Absolute limits (`.min`, `.max`); also the form when the emit point is not nominal.
    Limits {
        /// Lower limit.
        min: String,
        /// Upper limit.
        max: String,
    },
    /// An ISO 286 class the tool names itself.
    Fit {
        /// The class, e.g. `H7`.
        class: String,
    },
}

impl std::fmt::Display for DimTolerance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Symmetric { t } => write!(f, "+-{t}"),
            Self::Bilateral { upper, lower } => write!(f, "+{upper}/{lower}"),
            Self::Limits { min, max } => write!(f, "[{min}, {max}]"),
            Self::Fit { class } => f.write_str(class),
        }
    }
}

/// Name and version of the calling host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClientInfo {
    /// Client name, `cadetta`.
    pub name: String,
    /// Client version.
    pub version: String,
}

/// Params of `initialize`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitializeParams {
    /// Protocol identifier, [`PROTOCOL`].
    pub protocol: String,
    /// The calling host.
    pub client: ClientInfo,
}

/// Result of `initialize`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitializeResult {
    /// Adapter name.
    pub name: String,
    /// Adapter version; must equal `[[adapter]] version` (R008).
    pub version: String,
    /// Declared capabilities.
    pub capabilities: Vec<CapabilityName>,
    /// The tool's native length unit, as a unit symbol.
    pub native_units: String,
    /// Fractional decimal digits of the native number format.
    pub precision: u32,
    /// Platforms the adapter runs on (`linux`, `macos`, `windows`).
    pub platforms: Vec<String>,
    /// Optional features (adapters.md 5); absent means none.
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    #[serde(default)]
    pub features: Vec<Feature>,
}

/// A file path with the digest of its bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileDigest {
    /// Path of the file.
    pub path: String,
    /// `blake3:` digest of the file bytes.
    pub digest: String,
}

/// Params of `emit`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmitParams {
    /// The design IR (`cadetta.ir/1`).
    pub design: Value,
    /// The bindings table.
    pub bindings: Value,
    /// The policy decided upstream.
    pub policy: Value,
}

/// Result of `emit`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmitResult {
    /// Files written.
    pub files: Vec<FileDigest>,
    /// Diagnostics.
    pub diagnostics: Vec<DiagRecord>,
}

/// Params of `open`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenParams {
    /// Files to open.
    pub files: Vec<String>,
}

/// Dirty and read-only state of one opened file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileState {
    /// Path of the file.
    pub path: String,
    /// Unsaved changes exist (R009).
    pub dirty: bool,
    /// The file is read-only (R009).
    pub readonly: bool,
}

/// An open session: an opaque handle and per-file state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Session {
    /// Opaque session handle.
    pub handle: String,
    /// Per-file state.
    pub files: Vec<FileState>,
}

/// Result of `open`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenResult {
    /// The new session.
    pub session: Session,
    /// Diagnostics.
    pub diagnostics: Vec<DiagRecord>,
}

/// What `read` reads: file paths, or an open session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReadSource {
    /// Read these files; the adapter opens and closes its own session.
    Files(Vec<String>),
    /// Read from this session handle.
    Session(String),
}

/// Params of `read`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadParams {
    /// Where to read from.
    pub source: ReadSource,
}

/// The tolerance read back from one dimension locator (adapters.md 3.6).
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservedTolerance {
    /// The dimension locator.
    pub locator: String,
    /// Its tolerance; `None` when the dimension is untoleranced.
    pub tolerance: Option<DimTolerance>,
}

/// Where one bound sketch dimension sits, read for R007 (adapters.md 3.4).
// frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservedSketch {
    /// The document holding the sketch.
    pub file: String,
    /// The dimension locator.
    pub locator: String,
    /// True when the sketch lies on its datum's plane.
    pub on_datum_plane: bool,
    /// What the dimension is measured from; `sketch origin` is the only clean answer.
    pub measured_from: String,
}

/// The file an assembly actually holds for one component instance, read for R011 (adapters.md 3.3).
// frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservedComponent {
    /// The assembly document.
    pub file: String,
    /// The `instance` locator of the slot's binding.
    pub locator: String,
    /// The component file the instance uses.
    pub component_file: String,
}

/// The value each instance matched by one `[*]` binding holds, read for R012 (adapters.md 3).
// frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservedInstanceValues {
    /// The document holding the instances.
    pub file: String,
    /// The locator the `[*]` binding names.
    pub locator: String,
    /// One value text per instance, in instance order.
    pub values: Vec<String>,
}

/// Result of `read`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadResult {
    /// The observed IR.
    pub observed: Value,
    /// Dimension tolerances, only from adapters reporting `tolerances`.
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tolerances: Vec<ObservedTolerance>,
    /// Sketch-plane and origin facts for R007, only from adapters that can see them.
    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sketches: Vec<ObservedSketch>,
    /// Component files held by assembly slots, for R011.
    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<ObservedComponent>,
    /// Per-instance values of `[*]` bindings, for R012.
    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instance_values: Vec<ObservedInstanceValues>,
    /// Diagnostics.
    pub diagnostics: Vec<DiagRecord>,
}

/// How `push` writes a dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PushMode {
    /// Create or keep the link to `ref_to`.
    Ref,
    /// Set the number.
    Value,
    /// Set a frame's 3x4 transform (`value` is three rows of four numbers), creating the
    /// coordinate system named by the locator when it is missing (adapters.md 3.4).
    // frob:ticket 01M4SWMCYW919R5JKSV2T1ZYBN
    Frame,
}

/// One changed parameter in a `push`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangedParam {
    /// Canonical parameter path.
    pub path: String,
    /// The new value.
    pub value: Value,
    /// Where it lives in the native file.
    pub locator: String,
    /// How to write it.
    pub mode: PushMode,
    /// Link target when `mode` is `ref`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_to: Option<String>,
    /// The design tolerance to write with the value (adapters.md 3.6); only to `tolerances` adapters.
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<DimTolerance>,
}

/// Params of `push`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PushParams {
    /// Session handle.
    pub session: String,
    /// File the changes apply to.
    pub file: String,
    /// Changed parameters.
    pub params: Vec<ChangedParam>,
}

/// Outcome of one pushed locator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PushStatus {
    /// Written.
    Pushed,
    /// Already equal; nothing written.
    Skipped,
    /// Refused by the adapter.
    Rejected,
    /// Attempted and failed.
    Failed,
}

/// Per-locator result of a `push`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocatorStatus {
    /// The locator.
    pub locator: String,
    /// What happened.
    pub status: PushStatus,
    /// Value before the push, if known.
    pub old: Option<Value>,
    /// Value after the push, if known.
    pub new: Option<Value>,
    /// Tolerance read back after the push, for adapters reporting `tolerances`.
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<DimTolerance>,
}

/// Result of `push`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PushResult {
    /// One status per locator, never only a summary.
    pub statuses: Vec<LocatorStatus>,
    /// Diagnostics.
    pub diagnostics: Vec<DiagRecord>,
}

/// Params of `regen`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegenParams {
    /// Session handle.
    pub session: String,
    /// Targets to regenerate.
    pub targets: Vec<String>,
}

/// Outcome of regenerating one target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RegenState {
    /// Rebuilt cleanly.
    Ok,
    /// Rebuild failed (R004).
    Failed,
}

/// Per-target result of a `regen`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetStatus {
    /// The target.
    pub target: String,
    /// What happened.
    pub status: RegenState,
}

/// Result of `regen`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegenResult {
    /// One status per target.
    pub targets: Vec<TargetStatus>,
    /// Rebuild diagnostics.
    pub diagnostics: Vec<DiagRecord>,
}

/// Params of `save` and `close`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionParams {
    /// Session handle.
    pub session: String,
}

/// Result of `save`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveResult {
    /// Files saved.
    pub files: Vec<FileDigest>,
    /// Diagnostics.
    pub diagnostics: Vec<DiagRecord>,
}

/// Every protocol message shape, for schema generation only.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Messages {
    /// Params of `initialize`.
    pub initialize_params: InitializeParams,
    /// Result of `initialize`.
    pub initialize_result: InitializeResult,
    /// Params of `emit`.
    pub emit_params: EmitParams,
    /// Result of `emit`.
    pub emit_result: EmitResult,
    /// Params of `open`.
    pub open_params: OpenParams,
    /// Result of `open`.
    pub open_result: OpenResult,
    /// Params of `read`.
    pub read_params: ReadParams,
    /// Result of `read`.
    pub read_result: ReadResult,
    /// Params of `push`.
    pub push_params: PushParams,
    /// Result of `push`.
    pub push_result: PushResult,
    /// Params of `regen`.
    pub regen_params: RegenParams,
    /// Result of `regen`.
    pub regen_result: RegenResult,
    /// Params of `save` and `close`.
    pub session_params: SessionParams,
    /// Result of `save`.
    pub save_result: SaveResult,
}

/// The JSON Schema of [`Messages`], pretty-printed with a trailing newline.
pub fn schema_json() -> String {
    let mut text = serde_json::to_string_pretty(&schema_for!(Messages))
        .expect("a schema is always serializable");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:ticket 01M4SPWRS0CDJT4GYC92AB1HCD
    // frob:tests crates/cadetta-adapter/src/rpc/types.rs::schema_json kind=unit
    #[test]
    fn schema_file_matches_types() {
        let on_disk = include_str!("../../../../docs/schemas/adapter-protocol.json");
        assert_eq!(
            on_disk,
            schema_json(),
            "docs/schemas/adapter-protocol.json is stale; replace it with schema_json()"
        );
    }

    #[test]
    fn diag_record_rejects_foreign_shapes() {
        assert!(serde_json::from_value::<DiagRecord>(serde_json::json!({"code": "x"})).is_err());
    }
}
