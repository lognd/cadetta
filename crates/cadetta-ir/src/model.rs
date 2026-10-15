//! The `cadetta.ir/1` data model: the one shape every lint, query and adapter speaks.
//!
//! Specification: `docs/spec/ir.md` 1, 3. These are plain data types; JSON encoding lives in
//! the serialization module and digests in the digest module (later tickets). Magnitudes reuse
//! `cadetta-units` (`Magnitude` is the exact `r * pi^k`; `Dim` is the dimension vector), so the
//! IR never holds a float except `Inexact::f64`, which names its root operation.
//!
//! Decisions made here (written into ir.md 3): `Project.digest` and node digests are
//! `Option<Digest>` until the digest ticket fills them; `Value::nominal` is always the designed
//! value and there is no `designed` field, `drawn` exists only under a process rule; the
//! entities `Part`, `Fit` and `Solver` are defined here with the listed fields and the
//! dependent entity tickets extend them; `Project` keeps `modules` and `dependencies` sorted by
//! construction.
// frob:ticket 01M4SPWQ3VSX2CA3TZY4KQ2DZW

use std::collections::BTreeMap;

use cadetta_units::{Dim, Magnitude};
use tracing::debug;

use crate::entities::{Interface, Table, Waiver};
use crate::path::Path;

/// Which producer built an IR (ir.md 1); the schema is the same for both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flavour {
    /// Compiled from `.cdt` source: everything.
    Design,
    /// Read by an adapter from CAD files or inspection data.
    Observed,
}

/// A BLAKE3 digest as lowercase hex (ir.md 4).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Digest(pub String);

/// A source location kept for diagnostics; excluded from digests (ir.md 4).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpanRef {
    /// The file the node came from.
    pub file: String,
    /// Start byte offset.
    pub start: u32,
    /// End byte offset (exclusive).
    pub end: u32,
}

/// An exact magnitude `r * pi^pi` in SI base units (ir.md 3 `Exact`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Exact(pub Magnitude);

/// A float result of an inexact operation, naming the root operation (ir.md 3 `Inexact`).
#[derive(Debug, Clone, PartialEq)]
pub struct Inexact {
    /// The binary64 value; the only float in the IR.
    pub f64: f64,
    /// The member that performed the root inexact operation.
    pub root: Path,
}

/// A nominal: exact unless an inexact operation produced it.
#[derive(Debug, Clone, PartialEq)]
pub enum Nominal {
    /// An exact rational times a power of pi.
    Exact(Exact),
    /// A float with its root.
    Inexact(Inexact),
}

/// One tolerance source variable with its signed bounds (ir.md 3 `Affine`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AffineTerm {
    /// The variable's path.
    pub var: Path,
    /// Lowest deviation.
    pub lo: Exact,
    /// Highest deviation.
    pub hi: Exact,
}

/// An affine tolerance form: a list of variable terms (ir.md 3 `Affine`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Affine {
    /// The terms, one per source variable.
    pub terms: Vec<AffineTerm>,
}

/// The comparison of a bound member (ir.md 3 `Bound`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundOp {
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `in`
    In,
}

/// A bound member `name <= x`, `>= x` or `in [lo, hi]` (ir.md 3 `Bound`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bound {
    /// The comparison.
    pub op: BoundOp,
    /// Lower limit, when the op has one.
    pub lo: Option<Exact>,
    /// Upper limit, when the op has one.
    pub hi: Option<Exact>,
}

/// An evaluated quantity (ir.md 3 `Value`). `nominal` is always the designed value.
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    /// The designed value: what fits, checks and reads see.
    pub nominal: Nominal,
    /// The dimension vector (`.to(unit)` is computable from the IR alone).
    pub dim: Dim,
    /// Integer angle exponent.
    pub angle: i32,
    /// The tolerance form, if toleranced.
    pub tol: Option<Affine>,
    /// True when the tolerance is a conservative enclosure.
    pub conservative: bool,
    /// The compensated value that is emitted; present only when a process rule applied.
    pub drawn: Option<Exact>,
    /// False for observed values the native format could not give exactly.
    pub exact: bool,
    /// Why the value is inexact, for observed values.
    pub reason: Option<String>,
    /// A unit supplied by `--assume-unit` (adapters.md 3).
    pub unit_assumed: Option<String>,
    /// The value as authored, source text.
    pub authored: String,
}

impl Value {
    /// A fresh exact value with no tolerance and no process compensation.
    pub fn new(nominal: Nominal, dim: Dim, authored: impl Into<String>) -> Self {
        let exact = matches!(nominal, Nominal::Exact(_));
        debug!(exact, "ir: value built");
        Self {
            nominal,
            angle: dim.angle,
            dim,
            tol: None,
            conservative: false,
            drawn: None,
            exact,
            reason: None,
            unit_assumed: None,
            authored: authored.into(),
        }
    }
}

/// How a member came to exist (ir.md 3 `Provenance`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Provenance {
    /// Written in source.
    Authored,
    /// Computed from other members.
    Derived,
    /// Read from a CAD tool.
    Imported {
        /// The adapter that read it.
        adapter: String,
        /// The native file.
        file: String,
        /// The native parameter name.
        native_name: String,
    },
}

/// The class of a component (ir.md 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentClass {
    /// An off-the-shelf part.
    Ots,
    /// A modelled part.
    Model,
    /// An assembly.
    Assembly,
    /// A manufacturing process.
    Process,
}

/// What a member declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberKind {
    /// A parameter.
    Param,
    /// A `let` (analysis value).
    Let,
    /// A bound member (`<=`, `>=`, `in`).
    Bound,
    /// A datum offset or rotation component.
    Datum,
    /// A placement op argument.
    Place,
}

/// The declared type of a `cite` item (language.md 3.9, ir.md 3 `Citation`).
// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CitationType {
    /// A standard.
    Standard,
    /// A manufacturer datasheet.
    Datasheet,
    /// A drawing.
    Drawing,
    /// A catalog page.
    Catalog,
    /// A book.
    Book,
    /// A journal article.
    Article,
    /// A web page.
    Web,
    /// A quotation.
    Quote,
    /// Any other document.
    Document,
}

impl CitationType {
    /// Every type, in declaration order.
    pub const ALL: [CitationType; 9] = [
        Self::Standard,
        Self::Datasheet,
        Self::Drawing,
        Self::Catalog,
        Self::Book,
        Self::Article,
        Self::Web,
        Self::Quote,
        Self::Document,
    ];

    /// The source and wire name (`standard`, `datasheet`, ...).
    pub fn name(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Datasheet => "datasheet",
            Self::Drawing => "drawing",
            Self::Catalog => "catalog",
            Self::Book => "book",
            Self::Article => "article",
            Self::Web => "web",
            Self::Quote => "quote",
            Self::Document => "document",
        }
    }

    /// The type named `name`, if any.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }
}

/// One field value of a `cite` item: text, or a list for `author`.
// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CiteField {
    /// A single string.
    Text(String),
    /// A list of strings (`author` only).
    List(Vec<String>),
}

/// A `cite` item reached by the design (ir.md 3 `Citation`).
// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Citation {
    /// Canonical item path, `PKG::module::name`.
    pub path: String,
    /// The declared type.
    pub ty: CitationType,
    /// Field values by field name.
    pub fields: BTreeMap<String, CiteField>,
    /// Digest of the file named by the `file` field, when it was readable.
    pub file_digest: Option<Digest>,
    /// Digest of type, fields in name order and the file digest (`absent` when missing).
    pub digest: Digest,
}

impl Citation {
    /// Builds a citation and computes its digest.
    pub fn new(
        path: impl Into<String>,
        ty: CitationType,
        fields: BTreeMap<String, CiteField>,
        file_digest: Option<Digest>,
    ) -> Self {
        let digest = crate::digest::citation_digest(ty, &fields, file_digest.as_ref());
        Self {
            path: path.into(),
            ty,
            fields,
            file_digest,
            digest,
        }
    }

    /// The text of field `name`, when it is a single string.
    pub fn text(&self, name: &str) -> Option<&str> {
        match self.fields.get(name) {
            Some(CiteField::Text(t)) => Some(t),
            _ => None,
        }
    }

    /// The package-relative file the citation names, if any.
    pub fn file(&self) -> Option<&str> {
        self.text("file")
    }
}

/// A built-in citation form (ir.md 3 `CiteRef.builtin`).
// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CiteBuiltin {
    /// An inline `@"text"` note.
    Note,
    /// `@measured`.
    Measured,
    /// `@assumed`.
    Assumed,
    /// `@estimate`.
    Estimate,
}

impl CiteBuiltin {
    /// The wire name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Measured => "measured",
            Self::Assumed => "assumed",
            Self::Estimate => "estimate",
        }
    }
}

/// A use-site citation: a cite item path or a built-in form, with a locator (ir.md 3 `CiteRef`).
// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CiteRef {
    /// The cited `cite` item's path, when the citation names one.
    pub cite: Option<String>,
    /// The built-in form, when it names none.
    pub builtin: Option<CiteBuiltin>,
    /// A note's text.
    pub text: Option<String>,
    /// The string after `:`.
    pub locator: Option<String>,
}

impl CiteRef {
    /// A reference to a `cite` item.
    pub fn item(path: impl Into<String>, locator: Option<String>) -> Self {
        Self {
            cite: Some(path.into()),
            builtin: None,
            text: None,
            locator,
        }
    }

    /// A built-in reference (`measured`, `assumed`, `estimate`).
    pub fn builtin(kind: CiteBuiltin, locator: Option<String>) -> Self {
        Self {
            cite: None,
            builtin: Some(kind),
            text: None,
            locator,
        }
    }

    /// An inline note.
    pub fn note(text: impl Into<String>, locator: Option<String>) -> Self {
        Self {
            cite: None,
            builtin: Some(CiteBuiltin::Note),
            text: Some(text.into()),
            locator,
        }
    }

    /// The name the citation was written with: the item's last path segment, the built-in's
    /// name, or the note text.
    pub fn display_name(&self) -> String {
        match (&self.cite, &self.text, self.builtin) {
            (Some(c), _, _) => c.rsplit("::").next().unwrap_or(c).to_owned(),
            (None, Some(t), _) => t.clone(),
            (None, None, Some(b)) => b.name().to_owned(),
            (None, None, None) => String::new(),
        }
    }
}

/// A parameter, `let`, bound or generated member (ir.md 3 `Member`).
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    /// Canonical path.
    pub path: Path,
    /// What it declares.
    pub kind: MemberKind,
    /// True for a `let`: never emitted or bound.
    pub analysis: bool,
    /// True for a private member.
    pub private: bool,
    /// The evaluated value.
    pub value: Option<Value>,
    /// The bound, for bound members.
    pub bound: Option<Bound>,
    /// The normalized expression text.
    pub expr: Option<String>,
    /// Canonical paths this member reads; acyclic (D003).
    pub deps: Vec<Path>,
    /// The tolerance role (`hole`, `shaft`, ...).
    pub role: Option<String>,
    /// The citation for the value.
    pub citation: Option<CiteRef>,
    /// Previous names (`was`).
    pub was: Vec<String>,
    /// Attributes, semantic and presentational.
    pub attrs: BTreeMap<String, String>,
    /// How the member came to exist.
    pub provenance: Provenance,
    /// Doc comment; excluded from digests.
    pub doc: Option<String>,
    /// Source location; excluded from digests.
    pub span: Option<SpanRef>,
    /// Values that differ from the primary, keyed `"<slot>=<alternate>"`.
    pub variants: Option<BTreeMap<String, Value>>,
    /// Own digest.
    pub digest: Option<Digest>,
    /// Merkle closure digest.
    pub closure: Option<Digest>,
}

impl Member {
    /// A bare authored member; callers fill value, deps and the rest.
    pub fn new(path: Path, kind: MemberKind) -> Self {
        Self {
            path,
            analysis: kind == MemberKind::Let,
            kind,
            private: false,
            value: None,
            bound: None,
            expr: None,
            deps: Vec::new(),
            role: None,
            citation: None,
            was: Vec::new(),
            attrs: BTreeMap::new(),
            provenance: Provenance::Authored,
            doc: None,
            span: None,
            variants: None,
            digest: None,
            closure: None,
        }
    }
}

/// A part slot in a component (ir.md 3 `Part`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Canonical path of the slot.
    pub path: Path,
    /// The part's type (a component or interface path text).
    pub type_name: String,
    /// Instance count, when declared as a repeated part.
    pub count: Option<u32>,
    /// The primary component path.
    pub primary: String,
    /// Approved alternates, component path text.
    pub alternates: Vec<String>,
    /// Argument name to expression text.
    pub args: BTreeMap<String, String>,
}

/// A fit between a hole and a shaft member (ir.md 3 `Fit`).
#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    /// Canonical path.
    pub path: Path,
    /// The hole member.
    pub hole: Path,
    /// The shaft member.
    pub shaft: Path,
    /// The clearance (negative for interference).
    pub clearance: Value,
    /// The fit type (clearance, transition, interference), text.
    pub fit_type: String,
    /// The required clearance bound, if declared.
    pub required: Option<Bound>,
}

/// A component definition (ir.md 3 `Component`).
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    /// Canonical path.
    pub path: Path,
    /// Its class.
    pub class: ComponentClass,
    /// The parameter signature, for parameterized components.
    pub params_sig: Option<String>,
    /// Interfaces it implements, path text.
    pub implements: Vec<String>,
    /// Component-level citation.
    pub citation: Option<CiteRef>,
    /// Attributes.
    pub attrs: BTreeMap<String, String>,
    /// Its members.
    pub members: Vec<Member>,
    /// Its part slots.
    pub parts: Vec<Part>,
    /// Its fits.
    pub fits: Vec<Fit>,
    /// Its datums (geometry.md 2).
    pub datums: Vec<crate::geometry::Datum>,
    /// Its named points (geometry.md 3).
    pub points: Vec<crate::geometry::Point>,
    /// Its placements, one per array instance (geometry.md 4).
    pub placements: Vec<crate::geometry::Placement>,
    /// Source location.
    pub span: Option<SpanRef>,
    /// Own digest.
    pub digest: Option<Digest>,
}

/// Per-alternate outcome of a check (ir.md 3 `Check`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerdictState {
    /// Holds over the whole tolerance range.
    Pass,
    /// Fails somewhere in the range.
    Fail,
    /// Cannot be decided exactly.
    Unknown,
}

/// The verdicts of a check: the primary and one per alternate (ir.md 3 `Check`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Verdicts {
    /// The verdict on the primary evaluation.
    pub primary: Option<VerdictState>,
    /// Verdicts keyed `"<slot>=<alternate>"`.
    pub per_alternate: BTreeMap<String, VerdictState>,
}

/// A `check` assertion (ir.md 3 `Check`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// Canonical path.
    pub path: Path,
    /// Normalized expression text.
    pub expr: String,
    /// Primary and per-alternate verdicts.
    pub verdicts: Verdicts,
    /// True when a verdict rests on a conservative enclosure.
    pub conservative: bool,
    /// Alternates (`"<slot>=<alternate>"`) for which a waiver applies.
    pub waived_for: Vec<String>,
}

/// A solver declaration with its result state (ir.md 3 `Solver`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solver {
    /// Canonical path.
    pub path: Path,
    /// The script reference.
    pub script: String,
    /// Input name to member path text.
    pub inputs: BTreeMap<String, String>,
    /// Output name to member path text.
    pub outputs: BTreeMap<String, String>,
    /// Canonical dependency paths.
    pub deps: Vec<Path>,
    /// Digest of the recorded result.
    pub result_digest: Option<Digest>,
    /// True when inputs changed since the result was recorded.
    pub stale: bool,
}

/// What a native locator addresses (ir.md 3, adapters.md 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocatorKind {
    /// A global variable or equation.
    Global,
    /// A sketch or feature dimension.
    Dimension,
    /// A custom property.
    Property,
    /// A reference frame.
    Frame,
    /// A placed component instance (the placement node `Assembly/part.place`).
    // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
    Instance,
}

/// Which native fact implied a locator no bindings file lists (ir.md 3, adapters.md 3.5).
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Implicit {
    /// A global named by an in-scope path's created name.
    Global,
    /// A dimension whose equation is exactly a bare reference to such a global.
    Link,
    /// A frame named by a datum's created name.
    Frame,
    /// An instance whose reference property names a part instance path.
    Reference,
}

/// Whether a native entry holds the value or links to another (ir.md 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocatorMode {
    /// The entry holds the value.
    Value,
    /// The entry references another native name (`ref_to`).
    Ref,
}

/// The sign applied to a coordinate (`sign = -1` negates it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sign {
    /// Positive.
    Plus,
    /// Negative.
    Minus,
}

/// One native address of a bound member (ir.md 3 `Locator`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Locator {
    /// The opaque native locator text.
    pub locator: String,
    /// What it addresses.
    pub kind: LocatorKind,
    /// Value or reference mode.
    pub mode: LocatorMode,
    /// The unit the native tool stores.
    pub native_unit: String,
    /// The native name a `ref`-mode link points at.
    pub ref_to: Option<String>,
    /// The alternate a slot binding selects.
    pub for_alternate: Option<String>,
    /// The native configuration.
    pub config: Option<String>,
    /// Coordinate sign.
    pub sign: Option<Sign>,
    /// The in-assembly instance (`wheel<2>`).
    pub component: Option<String>,
    /// Set when the locator was discovered natively, not listed in a bindings file.
    // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
    pub implicit: Option<Implicit>,
    /// Whether a `dimension` locator's tolerance is written (adapters.md 3.6); `None` is the adapter default.
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    pub tolerance: Option<TolerancePolicy>,
}

/// Whether a dimension locator's design tolerance is written to the tool (adapters.md 3.6).
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TolerancePolicy {
    /// Write the design tolerance with the value.
    Drive,
    /// Leave the tool's tolerance alone.
    Ignore,
}

/// A frame seen in a native document (ir.md 3 observed `frames`).
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedFrame {
    /// The native frame name.
    pub name: String,
    /// The document holding it.
    pub document: String,
    /// The 3x4 transform (rotation columns then translation).
    pub transform: Vec<Vec<Value>>,
}

/// A component instance seen in a native document (ir.md 3 observed `instances`).
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedInstance {
    /// The native instance name.
    pub component: String,
    /// The document holding it.
    pub document: String,
    /// The reference property text, when set.
    pub reference: Option<String>,
    /// The 3x4 transform.
    pub transform: Vec<Vec<Value>>,
}

/// A member's native bindings for one adapter (ir.md 3 `Binding`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The bound member.
    pub path: Path,
    /// The adapter name.
    pub adapter: String,
    /// The native addresses.
    pub locators: Vec<Locator>,
}

/// The package a project was built from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    /// The dotted package name.
    pub name: String,
    /// The package version.
    pub version: String,
}

/// A resolved dependency (packages.md 1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dependency {
    /// The dependency's package name.
    pub name: String,
    /// The resolved version.
    pub version: String,
    /// Where it came from.
    pub source: String,
    /// Content digest.
    pub digest: String,
}

/// The whole IR: one schema for design and observed flavours (ir.md 1, 3).
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    /// The schema tag, `cadetta.ir/1`.
    pub schema: String,
    /// Design or observed.
    pub flavour: Flavour,
    /// The package.
    pub package: PackageId,
    /// Every module path of the package, sorted; `src/lib.cdt` items sit at the root.
    pub modules: Vec<String>,
    /// Resolved dependencies, sorted by name.
    pub dependencies: Vec<Dependency>,
    /// Components.
    pub components: Vec<Component>,
    /// Checks.
    pub checks: Vec<Check>,
    /// Solvers.
    pub solvers: Vec<Solver>,
    /// Bindings.
    pub bindings: Vec<Binding>,
    /// Citations.
    pub citations: Vec<Citation>,
    /// Interfaces (ir.md 3; omitted from JSON when empty).
    // frob:ticket 01M4YKQSGPM7JTKPRTZXSHCDWP
    pub interfaces: Vec<Interface>,
    /// Lookup tables (ir.md 3; omitted from JSON when empty).
    pub tables: Vec<Table>,
    /// Waivers (ir.md 3; omitted from JSON when empty).
    pub waivers: Vec<Waiver>,
    /// Observed frames (observed flavour only).
    // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
    pub frames: Vec<ObservedFrame>,
    /// Observed instances (observed flavour only).
    pub instances: Vec<ObservedInstance>,
    /// Own digest.
    pub digest: Option<Digest>,
}

/// The schema tag every project carries.
pub const SCHEMA: &str = "cadetta.ir/1";

impl Project {
    /// An empty project of `flavour`; both flavours use this one type.
    pub fn new(flavour: Flavour, package: PackageId) -> Self {
        debug!(?flavour, package = %package.name, "ir: project built");
        Self {
            schema: SCHEMA.to_owned(),
            flavour,
            package,
            modules: Vec::new(),
            dependencies: Vec::new(),
            components: Vec::new(),
            checks: Vec::new(),
            solvers: Vec::new(),
            bindings: Vec::new(),
            citations: Vec::new(),
            interfaces: Vec::new(),
            tables: Vec::new(),
            waivers: Vec::new(),
            frames: Vec::new(),
            instances: Vec::new(),
            digest: None,
        }
    }

    /// Set the module list, sorted and de-duplicated.
    pub fn set_modules(&mut self, mut modules: Vec<String>) {
        modules.sort();
        modules.dedup();
        self.modules = modules;
    }

    /// Set the dependency list, sorted by name.
    pub fn set_dependencies(&mut self, mut deps: Vec<Dependency>) {
        deps.sort_by(|a, b| a.name.cmp(&b.name));
        self.dependencies = deps;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathContext;
    use cadetta_units::Rational;

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn exact(n: i64) -> Exact {
        Exact(Magnitude::new(Rational::new(n, 1).unwrap(), 0))
    }

    fn pkg() -> PackageId {
        PackageId {
            name: "acme.rover".into(),
            version: "0.1.0".into(),
        }
    }

    fn value() -> Value {
        Value::new(Nominal::Exact(exact(3)), Dim::default(), "3")
    }

    // frob:tests ir_project_flavours
    #[test]
    fn ir_project_flavours() {
        let d = Project::new(Flavour::Design, pkg());
        let o = Project::new(Flavour::Observed, pkg());
        assert_eq!(d.flavour, Flavour::Design);
        assert_eq!(o.flavour, Flavour::Observed);
        assert_eq!(d.schema, o.schema);
    }

    // frob:tests ir_entities_exist
    #[test]
    fn ir_entities_exist() {
        let m = Member::new(p("acme.rover::Axle.d"), MemberKind::Param);
        let comp = Component {
            path: p("acme.rover::Axle.d"),
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: BTreeMap::new(),
            members: vec![m],
            parts: vec![Part {
                path: p("acme.rover::Rover/wheel.x"),
                type_name: "Wheel".into(),
                count: Some(2),
                primary: "A".into(),
                alternates: vec!["B".into()],
                args: BTreeMap::new(),
            }],
            fits: vec![Fit {
                path: p("acme.rover::Axle.seat"),
                hole: p("acme.rover::Axle.h"),
                shaft: p("acme.rover::Axle.s"),
                clearance: value(),
                fit_type: "clearance".into(),
                required: Some(Bound {
                    op: BoundOp::In,
                    lo: Some(exact(0)),
                    hi: Some(exact(1)),
                }),
            }],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        };
        assert_eq!(comp.members.len(), 1);
        let s = Solver {
            path: p("acme.rover::Axle.sol"),
            script: "s.py".into(),
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            deps: vec![],
            result_digest: None,
            stale: false,
        };
        assert!(!s.stale);
        let inexact = Nominal::Inexact(Inexact {
            f64: 1.5,
            root: p("acme.rover::Axle.d"),
        });
        assert!(!Value::new(inexact, Dim::default(), "sqrt(2)").exact);
        let a = Affine {
            terms: vec![AffineTerm {
                var: p("acme.rover::Axle.d"),
                lo: exact(-1),
                hi: exact(1),
            }],
        };
        assert_eq!(a.terms.len(), 1);
        let c = Citation::new(
            "acme.rover::refs::n",
            CitationType::Standard,
            BTreeMap::new(),
            None,
        );
        assert_eq!(c.ty, CitationType::Standard);
        let pr = Provenance::Imported {
            adapter: "freecad".into(),
            file: "a.FCStd".into(),
            native_name: "d".into(),
        };
        assert_ne!(pr, Provenance::Authored);
    }

    // frob:tests ir_value_fields
    #[test]
    fn ir_value_fields() {
        let mut v = value();
        v.tol = Some(Affine::default());
        assert!(v.exact && !v.conservative && v.authored == "3");
        assert!(matches!(v.nominal, Nominal::Exact(_)));
        assert_eq!(v.angle, 0);
    }

    // frob:tests ir_member_fields
    #[test]
    fn ir_member_fields() {
        let mut m = Member::new(p("acme.rover::Axle.d"), MemberKind::Let);
        assert!(m.analysis);
        m.deps.push(p("acme.rover::Axle.e"));
        m.variants = Some(BTreeMap::from([(
            "acme.rover::Rover/wheel=acme.rover::Colson2Series".to_owned(),
            value(),
        )]));
        m.digest = Some(Digest("00".into()));
        m.closure = Some(Digest("11".into()));
        assert_eq!(m.variants.as_ref().unwrap().len(), 1);
        assert!(m.digest.is_some() && m.closure.is_some() && m.deps.len() == 1);
    }

    // frob:tests ir_binding_locators
    #[test]
    fn ir_binding_locators() {
        let l = |kind, mode| Locator {
            locator: "D1@Sketch1".into(),
            kind,
            mode,
            native_unit: "mm".into(),
            ref_to: Some("x".into()),
            for_alternate: Some("Colson".into()),
            config: Some("Default".into()),
            sign: Some(Sign::Minus),
            component: Some("wheel<2>".into()),
            implicit: None,
            tolerance: None,
        };
        let b = Binding {
            path: p("acme.rover::Axle.d"),
            adapter: "solidworks".into(),
            locators: vec![
                l(LocatorKind::Global, LocatorMode::Value),
                l(LocatorKind::Dimension, LocatorMode::Ref),
                l(LocatorKind::Property, LocatorMode::Value),
                l(LocatorKind::Frame, LocatorMode::Value),
            ],
        };
        assert_eq!(b.locators.len(), 4);
        assert_eq!(b.locators[1].mode, LocatorMode::Ref);
        assert_eq!(b.locators[3].native_unit, "mm");
    }

    // frob:tests ir_check_verdicts
    #[test]
    fn ir_check_verdicts() {
        let mut v = Verdicts {
            primary: Some(VerdictState::Pass),
            ..Verdicts::default()
        };
        v.per_alternate
            .insert("Rover/wheel=Colson".into(), VerdictState::Fail);
        let c = Check {
            path: p("acme.rover::Axle.c"),
            expr: "a < b".into(),
            verdicts: v,
            conservative: false,
            waived_for: vec!["Rover/wheel=Colson".into()],
        };
        assert_eq!(c.verdicts.per_alternate.len(), 1);
        assert_eq!(c.waived_for.len(), 1);
    }

    // frob:tests ir_value_drawn_only_under_process
    #[test]
    fn ir_value_drawn_only_under_process() {
        let mut v = value();
        assert!(v.drawn.is_none());
        v.drawn = Some(exact(4));
        assert_eq!(v.nominal, Nominal::Exact(exact(3)));
    }

    // frob:tests ir_locator_fields
    #[test]
    fn ir_locator_fields() {
        let l = Locator {
            locator: "x".into(),
            kind: LocatorKind::Frame,
            mode: LocatorMode::Ref,
            native_unit: "in".into(),
            ref_to: None,
            for_alternate: None,
            config: None,
            sign: Some(Sign::Plus),
            component: None,
            implicit: None,
            tolerance: None,
        };
        assert_eq!(l.kind, LocatorKind::Frame);
        assert_eq!(l.sign, Some(Sign::Plus));
    }

    // frob:tests ir_project_package_modules
    #[test]
    fn ir_project_package_modules() {
        let mut pr = Project::new(Flavour::Design, pkg());
        pr.set_modules(vec!["drive::motor".into(), "".into(), "axle".into()]);
        assert_eq!(pr.modules, vec!["", "axle", "drive::motor"]);
        let dep = |n: &str| Dependency {
            name: n.into(),
            version: "1".into(),
            source: "path".into(),
            digest: "d".into(),
        };
        pr.set_dependencies(vec![dep("zeta"), dep("alpha")]);
        assert_eq!(pr.dependencies[0].name, "alpha");
        assert_eq!(pr.package.version, "0.1.0");
    }
}
