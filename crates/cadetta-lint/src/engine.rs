//! Lint engine: the code catalog, severity precedence, `fail_on` gating and
//! waiver/allow application over a run of diagnostics.
//!
//! Specification: `docs/spec/lints.md` sections 1 to 3 and `docs/spec/config-cli.md`.
// frob:ticket 01M4SPWQM289M7Q58CSS6F029F

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use cadetta_ir::entities::Waiver;
use cadetta_ir::model::Digest;
use cadetta_obligations::selector::Glob;

use crate::diag::{self, Diagnostic, Severity, Waived};

/// One row of the lint catalog (lints.md 3): the single source for `cadetta explain`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeInfo {
    /// Stable code, one family letter and three digits.
    pub code: &'static str,
    /// Kebab-case rule name.
    pub name: &'static str,
    /// Catalog default severity (the sync/emit value for Q004 and R008).
    pub default: Severity,
    /// Fixed codes report a broken program: never configurable, never waivable.
    pub fixed: bool,
    /// One-sentence meaning.
    pub meaning: &'static str,
}

/// The catalog, in code order (lints.md 3); retired codes are not listed.
pub const CATALOG: &[CodeInfo] = &[
    CodeInfo {
        code: "E001",
        name: "syntax-error",
        default: Severity::Error,
        fixed: true,
        meaning: "The source does not lex or parse (language.md 2, 3); lexical errors (quantities.md 2: leading or trailing `.`, `5\"abc\"`, bad compound literals) are E001 with a lexical message",
    },
    CodeInfo {
        code: "E002",
        name: "unresolved-name",
        default: Severity::Error,
        fixed: true,
        meaning: "A name, path or `use` target resolves to nothing",
    },
    CodeInfo {
        code: "E003",
        name: "duplicate-definition",
        default: Severity::Error,
        fixed: true,
        meaning: "Two definitions have one canonical path; both spans are given",
    },
    CodeInfo {
        code: "E004",
        name: "ambiguous-import",
        default: Severity::Error,
        fixed: true,
        meaning: "Two globs or uses bring the same name, where the name is used",
    },
    CodeInfo {
        code: "E005",
        name: "missing-package",
        default: Severity::Error,
        fixed: true,
        meaning: "A package's `src` is missing or holds no `.cdt` file, or a workspace member or path dependency has no `cadetta.toml` (packages.md 1)",
    },
    CodeInfo {
        code: "E006",
        name: "argument-mismatch",
        default: Severity::Error,
        fixed: true,
        meaning: "Arguments to a component, `part`, function or fit do not match the parameters (unknown, missing, wrong kind)",
    },
    CodeInfo {
        code: "E007",
        name: "lookup-miss",
        default: Severity::Error,
        fixed: true,
        meaning: "A table lookup key matched no row",
    },
    CodeInfo {
        code: "E008",
        name: "table-shape-mismatch",
        default: Severity::Error,
        fixed: true,
        meaning: "A table row or column does not match the table's `key` and `cols`",
    },
    CodeInfo {
        code: "E009",
        name: "non-exhaustive-match",
        default: Severity::Error,
        fixed: true,
        meaning: "A `match` leaves some case unhandled",
    },
    CodeInfo {
        code: "E010",
        name: "role-required",
        default: Severity::Error,
        fixed: true,
        meaning: "`mmc` or `lmc` read on a tolerance with no role (hole or boss) to say which side is material",
    },
    CodeInfo {
        code: "E011",
        name: "parameter-dependent-type-read",
        default: Severity::Error,
        fixed: true,
        meaning: "`Type.member` read where the member depends on component parameters; a parameterized component exists only as instances (language.md 3.1)",
    },
    CodeInfo {
        code: "E012",
        name: "version-unsatisfied",
        default: Severity::Error,
        fixed: true,
        meaning: "A resolved dependency does not meet the `version` requirement (packages.md 1)",
    },
    CodeInfo {
        code: "E013",
        name: "version-conflict",
        default: Severity::Error,
        fixed: true,
        meaning: "Two requirements on one package cannot be met by a single version",
    },
    CodeInfo {
        code: "E014",
        name: "dependency-changed",
        default: Severity::Error,
        fixed: true,
        meaning: "A dependency's content differs from the digest pinned in the lock; run `cadetta update`",
    },
    CodeInfo {
        code: "E015",
        name: "bad-module-name",
        default: Severity::Error,
        fixed: true,
        meaning: "A file or directory under `src` is not a valid module name",
    },
    CodeInfo {
        code: "E016",
        name: "private-item",
        default: Severity::Error,
        fixed: true,
        meaning: "An item is used outside its visibility, or a `priv` member is read from another package (packages.md 3)",
    },
    CodeInfo {
        code: "E017",
        name: "private-in-public",
        default: Severity::Error,
        fixed: true,
        meaning: "A `pub` item's signature names a less visible item, or a member implementing an interface member is `priv`",
    },
    CodeInfo {
        code: "Q001",
        name: "unitless-quantity",
        default: Severity::Error,
        fixed: true,
        meaning: "Bare number where a quantity is required (quantities.md 2)",
    },
    CodeInfo {
        code: "Q002",
        name: "dimension-mismatch",
        default: Severity::Error,
        fixed: true,
        meaning: "`+ -` or comparison across dimensions",
    },
    CodeInfo {
        code: "Q003",
        name: "kind-mismatch",
        default: Severity::Error,
        fixed: true,
        meaning: "Implicit conversion between named kinds",
    },
    CodeInfo {
        code: "Q004",
        name: "inexact-emit",
        default: Severity::Error,
        fixed: false,
        meaning: "Value not representable at the target's declared precision",
    },
    CodeInfo {
        code: "Q006",
        name: "inexact-value",
        default: Severity::Info,
        fixed: false,
        meaning: "Reported once at the root inexact operation, with the count of dependents",
    },
    CodeInfo {
        code: "Q007",
        name: "quantity-literal-split",
        default: Severity::Error,
        fixed: true,
        meaning: "Whitespace between number and unit",
    },
    CodeInfo {
        code: "Q008",
        name: "angle-dropped",
        default: Severity::Warning,
        fixed: false,
        meaning: "Angle exponent mismatch (quantities.md 4.3): the missing-2*pi class",
    },
    CodeInfo {
        code: "Q009",
        name: "non-integer-count",
        default: Severity::Error,
        fixed: true,
        meaning: "A Count that is not an integer",
    },
    CodeInfo {
        code: "Q010",
        name: "ambiguous-rounding",
        default: Severity::Error,
        fixed: true,
        meaning: "Rounding an inexact value that sits on a tie or step boundary",
    },
    CodeInfo {
        code: "Q011",
        name: "root-of-dimension",
        default: Severity::Error,
        fixed: true,
        meaning: "`sqrt` or `cbrt` of a dimension whose exponents do not divide",
    },
    CodeInfo {
        code: "N001",
        name: "non-canonical-case",
        default: Severity::Warning,
        fixed: false,
        meaning: "Name breaks the case canon: package `lower.dotted`, module `snake_case`, type `PascalCase`, rest `snake_case` (language.md 4)",
    },
    CodeInfo {
        code: "N002",
        name: "redundant-prefix",
        default: Severity::Warning,
        fixed: false,
        meaning: "Member repeats its component's name (`Axle.axle_length`)",
    },
    CodeInfo {
        code: "N003",
        name: "unit-in-name",
        default: Severity::Warning,
        fixed: false,
        meaning: "Name encodes a unit (`diameter_mm`)",
    },
    CodeInfo {
        code: "N004",
        name: "renamed-reference",
        default: Severity::Warning,
        fixed: false,
        meaning: "Reference uses a `was` path",
    },
    CodeInfo {
        code: "N005",
        name: "reserved-separator",
        default: Severity::Error,
        fixed: true,
        meaning: "Name contains `__` (reserved for mangling)",
    },
    CodeInfo {
        code: "N006",
        name: "type-read-beside-instance",
        default: Severity::Warning,
        fixed: false,
        meaning: "`Type.member` in an assembly that has a `part` of that type; suggests the instance (language.md 3.1)",
    },
    CodeInfo {
        code: "D001",
        name: "restated-value",
        default: Severity::Warning,
        fixed: false,
        meaning: "An authored literal in a `model` (or observed IR) equals another parameter's value of the same kind, in any component including its own",
    },
    CodeInfo {
        code: "D002",
        name: "derivable-authored",
        default: Severity::Warning,
        fixed: false,
        meaning: "A literal equals a value the design already offers by name: a fit member or its accessors, a table cell reachable from the component, or a solver output",
    },
    CodeInfo {
        code: "D003",
        name: "reference-cycle",
        default: Severity::Error,
        fixed: true,
        meaning: "Parameters depend on each other cyclically",
    },
    CodeInfo {
        code: "M001",
        name: "unexplained-precision",
        default: Severity::Warning,
        fixed: false,
        meaning: "In a `model`: a literal with more significant decimals than `[lint] precision` for its kind and no tolerance, fit, table lookup or allow",
    },
    CodeInfo {
        code: "M002",
        name: "magic-factor",
        default: Severity::Info,
        fixed: false,
        meaning: "Bare factor other than small integers, simple fractions and powers of ten",
    },
    CodeInfo {
        code: "M003",
        name: "untoleranced-fit-side",
        default: Severity::Warning,
        fixed: false,
        meaning: "A parameter used as a side of a `fit` has no tolerance",
    },
    CodeInfo {
        code: "F001",
        name: "unknown-fit-class",
        default: Severity::Error,
        fixed: true,
        meaning: "Class not in the ISO 286 tables",
    },
    CodeInfo {
        code: "F002",
        name: "size-out-of-range",
        default: Severity::Error,
        fixed: true,
        meaning: "Nominal outside the table's size range",
    },
    CodeInfo {
        code: "F003",
        name: "fit-tolerance-conflict",
        default: Severity::Error,
        fixed: false,
        meaning: "A side's own tolerance numerically differs from the class it is declared to play",
    },
    CodeInfo {
        code: "F004",
        name: "fit-range-violated",
        default: Severity::Error,
        fixed: false,
        meaning: "A fit's clearance lies entirely outside its declared required range; a partial overlap is C002",
    },
    CodeInfo {
        code: "I001",
        name: "missing-interface-member",
        default: Severity::Error,
        fixed: true,
        meaning: "An implementer lacks an interface member",
    },
    CodeInfo {
        code: "I002",
        name: "alternate-not-viable",
        default: Severity::Warning,
        fixed: false,
        meaning: "An approved alternate violates a check, fit, bound or require under variant evaluation (names it)",
    },
    CodeInfo {
        code: "I003",
        name: "alternate-undecided",
        default: Severity::Info,
        fixed: false,
        meaning: "An approved alternate is viable but leaves a check undecided that the primary decides",
    },
    CodeInfo {
        code: "I004",
        name: "root-not-evaluated",
        default: Severity::Info,
        fixed: false,
        meaning: "A root of the package being checked has parameters without defaults and no instances, so its members are never evaluated",
    },
    CodeInfo {
        code: "L001",
        name: "end-of-life-part",
        default: Severity::Warning,
        fixed: false,
        meaning: "A primary is `eol` or `obsolete` and has no viable alternate",
    },
    CodeInfo {
        code: "L002",
        name: "not-recommended-part",
        default: Severity::Info,
        fixed: false,
        meaning: "A primary is `nrnd`",
    },
    CodeInfo {
        code: "T001",
        name: "not-in-table",
        default: Severity::Warning,
        fixed: false,
        meaning: "An `ots` literal citing a table matches none of its rows",
    },
    CodeInfo {
        code: "P001",
        name: "missing-process-role",
        default: Severity::Warning,
        fixed: false,
        meaning: "Under a process, a feature-like parameter has no rule role",
    },
    CodeInfo {
        code: "S001",
        name: "ots-without-citation",
        default: Severity::Warning,
        fixed: false,
        meaning: "`ots` value with no citation on it or its component",
    },
    CodeInfo {
        code: "S002",
        name: "citation-unreachable",
        default: Severity::Info,
        fixed: false,
        meaning: "A local-file citation path does not exist",
    },
    CodeInfo {
        code: "S003",
        name: "solver-stale",
        default: Severity::Warning,
        fixed: false,
        meaning: "A solver result was computed against inputs that have changed",
    },
    CodeInfo {
        code: "S004",
        name: "citation-changed",
        default: Severity::Warning,
        fixed: false,
        meaning: "A citation's digest (fields or cited file) differs from the acknowledged one",
    },
    CodeInfo {
        code: "S005",
        name: "solver-failed",
        default: Severity::Error,
        fixed: false,
        meaning: "The last solve failed; the previous result is in use",
    },
    CodeInfo {
        code: "S006",
        name: "solver-unsolved-variant",
        default: Severity::Warning,
        fixed: false,
        meaning: "A variant changes a solver's inputs and no result exists for them; dependent verdicts are undecided (solvers.md 4.1)",
    },
    CodeInfo {
        code: "S007",
        name: "solver-input-not-quantity",
        default: Severity::Error,
        fixed: true,
        meaning: "A solver `in` expression is not a quantity-valued expression, or an `out` has no kind (solvers.md 1)",
    },
    CodeInfo {
        // frob:ticket 01M4VB63M63PQVRKN0HDXQP567
        code: "S008",
        name: "citation-incomplete",
        default: Severity::Warning,
        fixed: false,
        meaning: "A `cite` lacks a field its type needs, has a `url` with neither `file` nor `accessed`, or an `ots` value is cited only by an inline note",
    },
    CodeInfo {
        code: "C001",
        name: "check-violated",
        default: Severity::Error,
        fixed: false,
        meaning: "False for every value in tolerance (names the alternate when relevant)",
    },
    CodeInfo {
        code: "C002",
        name: "check-undecided",
        default: Severity::Warning,
        fixed: false,
        meaning: "Holds for some values and not others; says if the evaluation was conservative",
    },
    CodeInfo {
        code: "C003",
        name: "toleranced-equality",
        default: Severity::Warning,
        fixed: false,
        meaning: "`==` on toleranced operands; suggests accessors",
    },
    CodeInfo {
        code: "C004",
        name: "derived-feature-in-check",
        default: Severity::Info,
        fixed: false,
        meaning: "A check reads an arithmetic parameter with toleranced inputs and no tolerance of its own, so it treats a computed value as exact (quantities.md 5.3); suggests `let`",
    },
    CodeInfo {
        code: "O001",
        name: "obligation-stale",
        default: Severity::Warning,
        fixed: false,
        meaning: "An attestation's digest set changed (error if its role is `required_for = \"check\"`)",
    },
    CodeInfo {
        code: "O002",
        name: "obligation-open",
        default: Severity::Warning,
        fixed: false,
        meaning: "A required attestation was never given",
    },
    CodeInfo {
        code: "O003",
        name: "requirement-unattested",
        default: Severity::Error,
        fixed: false,
        meaning: "A `require` that holds lacks the sign-off its role requires for release",
    },
    CodeInfo {
        code: "O004",
        name: "sync-stale",
        default: Severity::Info,
        fixed: false,
        meaning: "The \"CAD file reflects design\" obligation is stale",
    },
    CodeInfo {
        code: "G001",
        name: "datum-mismatch",
        default: Severity::Error,
        fixed: true,
        meaning: "Arithmetic or comparison between positions in different datums without `in`; also array + point",
    },
    CodeInfo {
        code: "G002",
        name: "placement-cycle",
        default: Severity::Error,
        fixed: true,
        meaning: "A placement chain returns to a part already placed; the closing connection is a check",
    },
    CodeInfo {
        code: "G003",
        name: "unplaced-part",
        default: Severity::Error,
        fixed: true,
        meaning: "A position is converted out of a part that has no placement, or re-typed where the model lacks the part or its placement",
    },
    CodeInfo {
        code: "G004",
        name: "pattern-count-mismatch",
        default: Severity::Error,
        fixed: false,
        meaning: "`aligned` or a joint's `at`/`onto` pair patterns of different counts",
    },
    CodeInfo {
        code: "G005",
        name: "bolted-float-unlocated",
        default: Severity::Warning,
        fixed: false,
        meaning: "A check tighter than a joint's float reads through a placement carrying that float (geometry.md 5.2)",
    },
    CodeInfo {
        code: "G006",
        name: "double-placement",
        default: Severity::Error,
        fixed: true,
        meaning: "A part or instance is placed more than once",
    },
    CodeInfo {
        code: "G007",
        name: "unplaced-instance",
        default: Severity::Warning,
        fixed: false,
        meaning: "A placed array leaves an instance without a placement",
    },
    CodeInfo {
        code: "G008",
        name: "rotation-unconstrained",
        default: Severity::Warning,
        fixed: false,
        meaning: "A non-locating joint with a one-point pattern (or float >= spread) leaves the part free to turn about the screw (geometry.md 5.2)",
    },
    CodeInfo {
        code: "R001",
        name: "drift",
        default: Severity::Error,
        fixed: false,
        meaning: "Observed value differs from the value last emitted for that path (the emit point, not the nominal; the base is the lock `synced` record)",
    },
    CodeInfo {
        code: "R002",
        name: "unbound-native",
        default: Severity::Warning,
        fixed: false,
        meaning: "Native parameter has no binding",
    },
    CodeInfo {
        code: "R003",
        name: "missing-native",
        default: Severity::Error,
        fixed: false,
        meaning: "Bound path not found in the native file",
    },
    CodeInfo {
        code: "R004",
        name: "regen-failed",
        default: Severity::Error,
        fixed: false,
        meaning: "The CAD tool failed to rebuild after a push; affected documents are restored from their snapshot",
    },
    CodeInfo {
        code: "R005",
        name: "orphan-binding",
        default: Severity::Warning,
        fixed: false,
        meaning: "A binding's path matches no IR path and no `was` path",
    },
    CodeInfo {
        code: "R006",
        name: "native-structure-replaced",
        default: Severity::Warning,
        fixed: false,
        meaning: "A push replaces a native expression or driving equation on a bound locator (cadetta owns the dependency structure); lists the old expression",
    },
    CodeInfo {
        code: "R007",
        name: "binding-datum-mismatch",
        default: Severity::Warning,
        fixed: false,
        meaning: "A bound coordinate's native sketch is not on its datum's plane, or its dimension is not measured from the sketch origin (adapters that can see it; geometry.md 6)",
    },
    CodeInfo {
        code: "R008",
        name: "adapter-unavailable",
        default: Severity::Error,
        fixed: false,
        meaning: "Adapter not installed, wrong platform, or version mismatch (exit 2 on sync)",
    },
    CodeInfo {
        code: "R009",
        name: "file-busy",
        default: Severity::Error,
        fixed: false,
        meaning: "Document dirty, locked or read-only; refused before any push",
    },
    CodeInfo {
        code: "R010",
        name: "push-rejected",
        default: Severity::Error,
        fixed: false,
        meaning: "Push to a driven or suppressed dimension, an equation conflict or a unit conflict",
    },
    CodeInfo {
        code: "R011",
        name: "component-mismatch",
        default: Severity::Warning,
        fixed: false,
        meaning: "The file instantiated in an assembly is not the file bound to the slot's primary (for example after a manual swap)",
    },
    CodeInfo {
        code: "R012",
        name: "binding-divergent",
        default: Severity::Error,
        fixed: false,
        meaning: "A `[*]` binding over instances whose emitted values differ",
    },
    CodeInfo {
        code: "R013",
        name: "name-collision",
        default: Severity::Error,
        fixed: true,
        meaning: "Two emitted paths would create the same native name (adapters.md, emitters/sw-equations.md)",
    },
    CodeInfo {
        code: "R014",
        name: "text-not-expressible",
        default: Severity::Error,
        fixed: true,
        meaning: "An emission is required for a bound `Text` parameter and the adapter's format cannot express `Text` (`sw-equations`, emitters/sw-equations.md 1); a `Text` parameter that is not emitted is never reported",
    },
    CodeInfo {
        code: "R015",
        name: "locator-unsupported",
        default: Severity::Error,
        fixed: true,
        meaning: "An emission is required for a `dimension`, `property` or `frame` locator, or a `mode = \"ref\"` binding, on a format with only `global` value rows (`json`, `csv`); a locator that is not emitted is never reported",
    },
    CodeInfo {
        code: "R016",
        name: "unbound-row",
        default: Severity::Error,
        fixed: true,
        meaning: "A `read` of a `csv` file meets a row whose `name` matches no binding of that adapter (an error, unlike the native-discovery warning R002); no observed IR is produced",
    },
    CodeInfo {
        code: "U001",
        name: "unused-parameter",
        default: Severity::Info,
        fixed: false,
        meaning: "A `model` parameter is read by nothing and bound to nothing",
    },
    CodeInfo {
        code: "U002",
        name: "unused-private-item",
        default: Severity::Warning,
        fixed: false,
        meaning: "A private item is used nowhere in its package",
    },
    CodeInfo {
        code: "W001",
        name: "waiver-expired",
        default: Severity::Error,
        fixed: true,
        meaning: "A waiver's `expires` date has passed; it no longer suppresses (2.1)",
    },
    CodeInfo {
        code: "W002",
        name: "waiver-stale",
        default: Severity::Warning,
        fixed: true,
        meaning: "A target's digest changed since the waiver was reviewed; the suppressed diagnostic reappears",
    },
    CodeInfo {
        code: "W003",
        name: "waiver-not-allowed",
        default: Severity::Error,
        fixed: true,
        meaning: "Waiver or `allow` of a fixed code or of a diagnostic raised by a `require`",
    },
    CodeInfo {
        code: "W004",
        name: "waiver-unused",
        default: Severity::Warning,
        fixed: true,
        meaning: "A waiver or `allow` suppresses nothing",
    },
    CodeInfo {
        code: "W005",
        name: "waiver-expiring",
        default: Severity::Info,
        fixed: false,
        meaning: "A waiver or an `eol` date falls within `[lint] expiry_warn_days` (default 30)",
    },
];

/// Codes whose severity `required_for` floors at error (lints.md 1, rule 2).
pub const FLOOR_CODES: &[&str] = &["O001", "O002", "O004", "S003"];

/// Look a code up in the catalog (`cadetta explain CODE`).
pub fn explain(code: &str) -> Option<&'static CodeInfo> {
    CATALOG.iter().find(|c| c.code == code)
}

/// Errors from engine configuration and construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineError {
    /// A code is not in the catalog.
    UnknownCode(String),
    /// A `[lint] severity` value is not `error`, `warning` or `info`.
    BadSeverity(String),
    /// A `fail_on` value is not `error`, `warning`, `info` or `never`.
    BadFailOn(String),
    /// A `required_for` floor was requested for a code outside [`FLOOR_CODES`].
    NotFloorable(String),
    /// An `allow` has no (or a blank) reason string.
    AllowNeedsBecause(String),
    /// A waiver glob does not compile.
    BadGlob(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCode(c) => write!(f, "unknown lint code `{c}`"),
            Self::BadSeverity(s) => write!(
                f,
                "invalid severity `{s}` (expected error, warning or info)"
            ),
            Self::BadFailOn(s) => write!(
                f,
                "invalid fail_on `{s}` (expected error, warning, info or never)"
            ),
            Self::NotFloorable(c) => write!(f, "`{c}` has no required_for floor"),
            Self::AllowNeedsBecause(c) => write!(f, "allow({c}) requires a because string"),
            Self::BadGlob(g) => write!(f, "invalid waiver glob `{g}`"),
        }
    }
}

impl std::error::Error for EngineError {}

/// Rank of a severity for ordering (higher is more severe).
fn rank(s: Severity) -> u8 {
    match s {
        Severity::Error => 3,
        Severity::Warning => 2,
        Severity::Info => 1,
        Severity::Hint => 0,
    }
}

/// The lowest severity that makes a command exit 1 (`[lint] fail_on`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailOn {
    /// Fail on errors (default).
    Error,
    /// Fail on warnings and above.
    Warning,
    /// Fail on info and above.
    Info,
    /// Never fail; diagnostics are informational.
    Never,
}

impl FailOn {
    /// Parse the config value.
    pub fn parse(s: &str) -> Result<Self, EngineError> {
        match s {
            "error" => Ok(Self::Error),
            "warning" => Ok(Self::Warning),
            "info" => Ok(Self::Info),
            "never" => Ok(Self::Never),
            other => Err(EngineError::BadFailOn(other.to_owned())),
        }
    }

    /// Whether a diagnostic of severity `s` fails the run.
    pub fn fails(self, s: Severity) -> bool {
        let floor = match self {
            Self::Error => Severity::Error,
            Self::Warning => Severity::Warning,
            Self::Info => Severity::Info,
            Self::Never => return false,
        };
        rank(s) >= rank(floor)
    }
}

/// Which command is running; Q004 and R008 default differently when writing to CAD.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// `check`: no target to write (Q004 warning, R008 info).
    Check,
    /// `sync`/emit: Q004 and R008 are errors (the catalog value).
    Sync,
}

/// The `[lint]` configuration plus the active `required_for` floors.
#[derive(Clone, Debug)]
pub struct LintConfig {
    /// Gating threshold.
    pub fail_on: FailOn,
    /// Running command.
    pub mode: Mode,
    overrides: BTreeMap<String, Severity>,
    floors: BTreeSet<String>,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            fail_on: FailOn::Error,
            mode: Mode::Check,
            overrides: BTreeMap::new(),
            floors: BTreeSet::new(),
        }
    }
}

impl LintConfig {
    /// A default configuration: `fail_on = error`, check mode, no overrides.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one `[lint] severity CODE = "sev"` entry; a fixed code's override is ignored.
    pub fn set_severity(&mut self, code: &str, sev: &str) -> Result<(), EngineError> {
        let info = explain(code).ok_or_else(|| EngineError::UnknownCode(code.to_owned()))?;
        let sev = match sev {
            "error" => Severity::Error,
            "warning" => Severity::Warning,
            "info" => Severity::Info,
            other => return Err(EngineError::BadSeverity(other.to_owned())),
        };
        if info.fixed {
            tracing::warn!(%code, "lint: severity override of a fixed code ignored");
            return Ok(());
        }
        tracing::debug!(%code, ?sev, "lint: severity override set");
        self.overrides.insert(code.to_owned(), sev);
        Ok(())
    }

    /// Mark `code`'s `required_for` gate as active for this run: its severity is at least error.
    pub fn require_floor(&mut self, code: &str) -> Result<(), EngineError> {
        if !FLOOR_CODES.contains(&code) {
            return Err(EngineError::NotFloorable(code.to_owned()));
        }
        tracing::debug!(%code, "lint: required_for floor active");
        self.floors.insert(code.to_owned());
        Ok(())
    }

    /// Whether `[lint] severity` set `code` explicitly (an override beats a rule-set severity).
    pub fn is_overridden(&self, code: &str) -> bool {
        self.overrides.contains_key(code)
    }

    /// Whether the `required_for` floor is active for `code`.
    pub fn has_floor(&self, code: &str) -> bool {
        self.floors.contains(code)
    }

    /// The effective severity of `code` after lints.md 1 rules 1 to 3, or `None` if unknown.
    pub fn severity(&self, code: &str) -> Option<Severity> {
        let info = explain(code)?;
        if info.fixed {
            return Some(info.default);
        }
        let mut sev = self
            .overrides
            .get(code)
            .copied()
            .unwrap_or(match (self.mode, code) {
                (Mode::Check, "Q004") => Severity::Warning,
                (Mode::Check, "R008") => Severity::Info,
                _ => info.default,
            });
        if self.floors.contains(code) && rank(sev) < rank(Severity::Error) {
            sev = Severity::Error;
        }
        Some(sev)
    }
}

/// `#[allow(CODE, "because")]` on one member (lints.md 2.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allow {
    /// The suppressed code.
    pub code: String,
    /// Canonical path of the member.
    pub path: String,
    /// The required reason.
    pub because: String,
}

impl Allow {
    /// Build an allow; a missing or blank reason is rejected.
    pub fn new(code: &str, path: &str, because: &str) -> Result<Self, EngineError> {
        if because.trim().is_empty() {
            tracing::warn!(%code, %path, "lint: allow without because rejected");
            return Err(EngineError::AllowNeedsBecause(code.to_owned()));
        }
        Ok(Self {
            code: code.to_owned(),
            path: path.to_owned(),
            because: because.to_owned(),
        })
    }

    /// Stable identifier recorded in `waived.waiver`.
    pub fn id(&self) -> String {
        format!("allow:{}@{}", self.code, self.path)
    }
}

/// Stable identifier of a waiver, recorded in `waived.waiver`.
pub fn waiver_id(w: &Waiver) -> String {
    format!("waiver:{}:{}", w.glob, w.code)
}

/// Bind the closure digests of the waiver's targets (lints.md 2.1); returns the target count.
pub fn bind_targets(
    waiver: &mut Waiver,
    closures: &BTreeMap<String, Digest>,
) -> Result<usize, EngineError> {
    let glob = Glob::new(&waiver.glob).map_err(|_| EngineError::BadGlob(waiver.glob.clone()))?;
    waiver.targets_closure = closures
        .iter()
        .filter(|(p, _)| glob.matches(p))
        .map(|(p, d)| (p.clone(), d.clone()))
        .collect();
    tracing::debug!(id = %waiver_id(waiver), targets = waiver.targets_closure.len(), "lint: waiver bound");
    Ok(waiver.targets_closure.len())
}

/// Bind a source waiver to the digest the lock recorded when it was reviewed (lints.md 2.1).
///
/// The lock stores one combined digest for all targets. When the combined digest of the current
/// target closures differs from `recorded`, every target is bound to `recorded` so
/// [`waiver_state`] reports it stale (W002); an unchanged or unreviewed waiver is left open.
/// Returns whether the waiver was marked changed.
// frob:ticket 01M4TRKDEST57137A1Y203JFR7
pub fn bind_reviewed(
    waiver: &mut Waiver,
    closures: &BTreeMap<String, Digest>,
    recorded: Option<&str>,
) -> Result<bool, EngineError> {
    let Some(recorded) = recorded else {
        return Ok(false);
    };
    let glob = Glob::new(&waiver.glob).map_err(|_| EngineError::BadGlob(waiver.glob.clone()))?;
    let targets: Vec<(&String, &Digest)> =
        closures.iter().filter(|(p, _)| glob.matches(p)).collect();
    if targets.is_empty() {
        return Ok(false);
    }
    let current = cadetta_obligations::staleness::covered_digest(
        &targets
            .iter()
            .map(|(_, d)| (*d).clone())
            .collect::<Vec<_>>(),
    );
    if current.0 == recorded {
        return Ok(false);
    }
    waiver.targets_closure = targets
        .into_iter()
        .map(|(p, _)| (p.clone(), Digest(recorded.to_owned())))
        .collect();
    tracing::debug!(id = %waiver_id(waiver), "lint: waiver targets changed since review");
    Ok(true)
}

/// Whether a waiver currently suppresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaiverState {
    /// Suppresses (open or reviewed with matching digests).
    Active,
    /// `expires` has passed at `as_of` (W001).
    Expired,
    /// A recorded target digest differs from the current closure (W002).
    Stale,
}

/// Classify a waiver; `as_of` is ISO `YYYY-MM-DD`, expired when strictly after `expires`.
pub fn waiver_state(
    w: &Waiver,
    closures: &BTreeMap<String, Digest>,
    as_of: Option<&str>,
) -> WaiverState {
    if let (Some(exp), Some(now)) = (w.expires.as_deref(), as_of)
        && now > exp
    {
        return WaiverState::Expired;
    }
    if w.targets_closure
        .iter()
        .any(|(p, d)| closures.get(p) != Some(d))
    {
        return WaiverState::Stale;
    }
    WaiverState::Active
}

/// Everything that can suppress a run.
#[derive(Clone, Debug, Default)]
pub struct Suppressions {
    /// `waive` declarations.
    pub waivers: Vec<Waiver>,
    /// `#[allow]` attributes.
    pub allows: Vec<Allow>,
    /// Paths of `require`s: diagnostics there are never suppressed (lints.md 2.3).
    pub require_paths: BTreeSet<String>,
    /// Current closure digest of every path.
    pub closures: BTreeMap<String, Digest>,
    /// The `--as-of` date, if any.
    pub as_of: Option<String>,
}

/// The gated run: sorted diagnostics plus what the waiver rules need.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    /// Diagnostics with effective severity, `waived` set where suppressed, sorted.
    pub diagnostics: Vec<Diagnostic>,
    /// Active waivers and allows that matched nothing (W004).
    pub unused: Vec<String>,
    /// Waivers past `expires` (W001).
    pub expired: Vec<String>,
    /// Waivers with a changed target digest (W002).
    pub stale: Vec<String>,
}

impl Outcome {
    /// Whether any unsuppressed diagnostic reaches `fail_on`.
    pub fn fails(&self, fail_on: FailOn) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.waived.is_none() && fail_on.fails(d.severity))
    }
}

fn alt_matches(w: &Waiver, d: &Diagnostic) -> bool {
    match &w.for_alt {
        None => true,
        Some(v) => d.alt.as_deref() == Some(v.to_string().as_str()),
    }
}

/// Severity for a diagnostic: a rule-set downgrade (below the configured one, not `Hint`)
/// survives unless the user overrode the code or a `required_for` floor is active.
fn resolve_severity(
    emitted: Severity,
    configured: Severity,
    overridden: bool,
    floored: bool,
) -> Severity {
    if !overridden && !floored && emitted != Severity::Hint && rank(emitted) < rank(configured) {
        tracing::debug!(
            ?emitted,
            ?configured,
            "lint: rule-set severity downgrade kept"
        );
        emitted
    } else {
        configured
    }
}

/// True when `glob` matches `path` or an ancestor item path of it (lints.md 2.1).
fn covers(glob: &Glob, path: &str) -> bool {
    let mut cur = path;
    loop {
        if glob.matches(cur) {
            return true;
        }
        // Ancestors stop at the item: never strip into the package part.
        let Some(item_at) = cur.find("::").map(|i| i + 2) else {
            return false;
        };
        match cur[item_at..].rfind('.') {
            Some(i) => cur = &cur[..item_at + i],
            None => return false,
        }
    }
}

/// Apply severity precedence, then waivers and allows, to a run (lints.md 1 and 2).
pub fn apply(mut diags: Vec<Diagnostic>, cfg: &LintConfig, sup: &Suppressions) -> Outcome {
    let mut out = Outcome::default();
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut active: Vec<(usize, Option<Glob>)> = Vec::new();
    for (i, w) in sup.waivers.iter().enumerate() {
        match waiver_state(w, &sup.closures, sup.as_of.as_deref()) {
            WaiverState::Expired => out.expired.push(waiver_id(w)),
            WaiverState::Stale => out.stale.push(waiver_id(w)),
            WaiverState::Active => active.push((i, Glob::new(&w.glob).ok())),
        }
    }
    for d in &mut diags {
        // frob:ticket 01M4V13T1DW08P87XP4J4802RH
        let date_passed_l001 = d.code == "L001" && d.severity == Severity::Error;
        if date_passed_l001 {
            tracing::debug!("lint: L001 date-passed error escalation kept");
        } else if let Some(s) = cfg.severity(&d.code) {
            d.severity = resolve_severity(
                d.severity,
                s,
                cfg.is_overridden(&d.code),
                cfg.has_floor(&d.code),
            );
        }
        let fixed = explain(&d.code).is_some_and(|c| c.fixed);
        let protected = d
            .path
            .as_deref()
            .is_some_and(|p| sup.require_paths.contains(p));
        if fixed || protected {
            continue;
        }
        let Some(path) = d.path.clone() else { continue };
        let by_waiver = active.iter().find_map(|(i, g)| {
            let w = &sup.waivers[*i];
            (w.code == d.code && alt_matches(w, d) && g.as_ref().is_some_and(|g| covers(g, &path)))
                .then(|| (waiver_id(w), w.because.clone()))
        });
        let hit = by_waiver.or_else(|| {
            sup.allows
                .iter()
                .find(|a| a.code == d.code && a.path == path)
                .map(|a| (a.id(), a.because.clone()))
        });
        if let Some((id, because)) = hit {
            tracing::debug!(code = %d.code, %path, %id, "lint: diagnostic suppressed");
            used.insert(id.clone());
            d.waived = Some(Waived {
                waiver: id,
                because,
            });
        }
    }
    for (i, _) in &active {
        let id = waiver_id(&sup.waivers[*i]);
        if !used.contains(&id) {
            out.unused.push(id);
        }
    }
    for a in &sup.allows {
        if !used.contains(&a.id()) {
            out.unused.push(a.id());
        }
    }
    diag::sort(&mut diags);
    out.diagnostics = diags;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::variant_name::VariantName;

    fn d(code: &str, path: &str) -> Diagnostic {
        let mut x = Diagnostic::new(code, Severity::Hint, "m");
        x.path = Some(path.into());
        x
    }

    fn dg(s: &str) -> Digest {
        Digest(s.into())
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::apply kind=unit
    #[test]
    fn l001_date_passed_error_survives_warning_default() {
        let c = LintConfig::new();
        assert_eq!(c.severity("L001"), Some(Severity::Warning));
        let mut passed = d("L001", "a::B.c");
        passed.severity = Severity::Error;
        let mut before = d("L001", "a::B.d");
        before.severity = Severity::Warning;
        let out = apply(vec![passed, before], &c, &Suppressions::default());
        let sev = |p: &str| {
            out.diagnostics
                .iter()
                .find(|x| x.path.as_deref() == Some(p))
                .unwrap()
                .severity
        };
        assert_eq!(sev("a::B.c"), Severity::Error);
        assert_eq!(sev("a::B.d"), Severity::Warning);
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::apply kind=unit
    #[test]
    fn rule_set_downgrade_stays_info_unless_overridden() {
        let mut x = d("D001", "a::B.c");
        x.severity = Severity::Info;
        let mut c = LintConfig::new();
        assert_eq!(c.severity("D001"), Some(Severity::Warning));
        let out = apply(vec![x.clone()], &c, &Suppressions::default());
        assert_eq!(out.diagnostics[0].severity, Severity::Info);
        c.set_severity("D001", "error").unwrap();
        let out = apply(vec![x], &c, &Suppressions::default());
        assert_eq!(out.diagnostics[0].severity, Severity::Error);
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::LintConfig.set_severity kind=unit
    #[test]
    fn severity_override_applies() {
        let mut c = LintConfig::new();
        assert_eq!(c.severity("N002"), Some(Severity::Warning));
        c.set_severity("N002", "info").unwrap();
        let out = apply(vec![d("N002", "a::B.c")], &c, &Suppressions::default());
        assert_eq!(out.diagnostics[0].severity, Severity::Info);
        assert_eq!(
            c.set_severity("ZZ99", "info"),
            Err(EngineError::UnknownCode("ZZ99".into()))
        );
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::LintConfig.severity kind=unit
    #[test]
    fn fixed_override_ignored() {
        let mut c = LintConfig::new();
        c.set_severity("E001", "info").unwrap();
        assert_eq!(c.severity("E001"), Some(Severity::Error));
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::apply kind=unit
    #[test]
    fn waiver_suppresses_path() {
        let sup = Suppressions {
            waivers: vec![Waiver::new("M001", "acme.rover::Axle.chamfer", "ok")],
            ..Default::default()
        };
        let out = apply(
            vec![
                d("M001", "acme.rover::Axle.chamfer"),
                d("M001", "acme.rover::Axle.other"),
            ],
            &LintConfig::new(),
            &sup,
        );
        assert!(out.diagnostics[0].waived.is_some());
        assert!(out.diagnostics[1].waived.is_none());
        assert!(out.unused.is_empty());
        assert!(!out.fails(FailOn::Info) || out.diagnostics[1].severity != Severity::Hint);
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::apply kind=unit
    #[test]
    fn waiver_on_item_path_covers_members() {
        let sup = Suppressions {
            waivers: vec![Waiver::new("M001", "acme.rover::Axle", "ok")],
            ..Default::default()
        };
        let out = apply(
            vec![
                d("M001", "acme.rover::Axle.a.b"),
                d("M001", "acme.rover::Axle2.a"),
            ],
            &LintConfig::new(),
            &sup,
        );
        assert!(out.diagnostics[0].waived.is_some());
        assert!(out.diagnostics[1].waived.is_none());
        assert!(out.unused.is_empty());
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::Allow.new kind=unit
    #[test]
    fn allow_needs_because_and_has_no_expiry() {
        assert!(Allow::new("M001", "a::B.c", "").is_err());
        assert!(Allow::new("M001", "a::B.c", "  ").is_err());
        let a = Allow::new("M001", "a::B.c", "vendor drawing 1234").unwrap();
        let sup = Suppressions {
            allows: vec![a],
            as_of: Some("2999-01-01".into()),
            ..Default::default()
        };
        let out = apply(vec![d("M001", "a::B.c")], &LintConfig::new(), &sup);
        let w = out.diagnostics[0].waived.as_ref().unwrap();
        assert_eq!(w.because, "vendor drawing 1234");
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::apply kind=unit
    #[test]
    fn waiver_scoped_to_alt() {
        let mut w = Waiver::new("D001", "a::B.*", "ok");
        w.for_alt = Some(VariantName::parse("a::B/wheel=a::Colson2Series").unwrap());
        let sup = Suppressions {
            waivers: vec![w],
            ..Default::default()
        };
        let mut on = d("D001", "a::B.x");
        on.alt = Some("a::B/wheel=a::Colson2Series".into());
        let mut other = d("D001", "a::B.x");
        other.alt = Some("a::B/wheel=a::Other".into());
        let base = d("D001", "a::B.x");
        let out = apply(vec![on, other, base], &LintConfig::new(), &sup);
        let waived: Vec<bool> = out.diagnostics.iter().map(|x| x.waived.is_some()).collect();
        assert_eq!(waived.iter().filter(|b| **b).count(), 1);
        let hit = out.diagnostics.iter().find(|x| x.waived.is_some()).unwrap();
        assert_eq!(hit.alt.as_deref(), Some("a::B/wheel=a::Colson2Series"));
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::bind_targets kind=unit
    #[test]
    fn waiver_binds_target_digests() {
        let closures: BTreeMap<String, Digest> = [
            ("a::B.x".to_owned(), dg("11")),
            ("a::B.y".to_owned(), dg("22")),
            ("a::C.z".to_owned(), dg("33")),
        ]
        .into();
        let mut w = Waiver::new("D001", "a::B.*", "ok");
        assert_eq!(bind_targets(&mut w, &closures), Ok(2));
        assert_eq!(w.targets_closure.get("a::B.x"), Some(&dg("11")));
        assert!(!w.targets_closure.contains_key("a::C.z"));
        let mut cur = closures.clone();
        assert_eq!(waiver_state(&w, &cur, None), WaiverState::Active);
        cur.insert("a::B.x".into(), dg("99"));
        assert_eq!(waiver_state(&w, &cur, None), WaiverState::Stale);
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::bind_reviewed kind=unit
    #[test]
    fn reviewed_waiver_is_stale_only_when_the_combined_digest_changed() {
        let closures: BTreeMap<String, Digest> = [
            ("a::B.x".to_owned(), dg("11")),
            ("a::B.y".to_owned(), dg("22")),
        ]
        .into();
        let recorded = cadetta_obligations::staleness::covered_digest(&[dg("11"), dg("22")]);
        let mut w = Waiver::new("D001", "a::B.*", "ok");
        assert_eq!(bind_reviewed(&mut w, &closures, None), Ok(false));
        assert_eq!(
            bind_reviewed(&mut w, &closures, Some(&recorded.0)),
            Ok(false)
        );
        assert_eq!(waiver_state(&w, &closures, None), WaiverState::Active);
        let mut changed = closures.clone();
        changed.insert("a::B.y".into(), dg("99"));
        assert_eq!(bind_reviewed(&mut w, &changed, Some(&recorded.0)), Ok(true));
        assert_eq!(waiver_state(&w, &changed, None), WaiverState::Stale);
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::explain kind=unit
    #[test]
    fn catalog_is_single_table() {
        let e = explain("N002").unwrap();
        assert_eq!(e.name, "redundant-prefix");
        assert_eq!(e.default, Severity::Warning);
        assert!(!e.fixed);
        assert!(!e.meaning.is_empty());
        assert!(explain("E001").unwrap().fixed);
        assert!(explain("Q005").is_none());
        let mut codes: Vec<&str> = CATALOG.iter().map(|c| c.code).collect();
        let n = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), n);
        for c in FLOOR_CODES {
            assert!(explain(c).is_some());
        }
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::LintConfig.severity kind=unit
    #[test]
    fn precedence_floor_and_fail_on() {
        let mut c = LintConfig::new();
        c.set_severity("O002", "info").unwrap();
        assert_eq!(c.severity("O002"), Some(Severity::Info));
        c.require_floor("O002").unwrap();
        assert_eq!(c.severity("O002"), Some(Severity::Error));
        assert!(c.require_floor("N002").is_err());
        // waive then suppresses a floored, non-fixed diagnostic.
        let sup = Suppressions {
            waivers: vec![Waiver::new("O002", "a::B.x", "ok")],
            ..Default::default()
        };
        let out = apply(vec![d("O002", "a::B.x")], &c, &sup);
        assert_eq!(out.diagnostics[0].severity, Severity::Error);
        assert!(!out.fails(FailOn::Error));
        // fixed codes and require paths are never suppressed.
        let mut sup = Suppressions {
            waivers: vec![
                Waiver::new("E002", "a::B.x", "ok"),
                Waiver::new("C001", "a::R.req", "ok"),
            ],
            ..Default::default()
        };
        sup.require_paths.insert("a::R.req".into());
        let out = apply(
            vec![d("E002", "a::B.x"), d("C001", "a::R.req")],
            &LintConfig::new(),
            &sup,
        );
        assert!(out.diagnostics.iter().all(|x| x.waived.is_none()));
        assert!(out.fails(FailOn::Error));
        assert!(!out.fails(FailOn::Never));
        assert!(FailOn::Warning.fails(Severity::Warning));
        assert!(!FailOn::Error.fails(Severity::Warning));
        assert_eq!(FailOn::parse("x"), Err(EngineError::BadFailOn("x".into())));
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::waiver_state kind=unit
    #[test]
    fn expired_waiver_stops_suppressing() {
        let mut w = Waiver::new("M001", "a::B.c", "ok");
        w.expires = Some("2027-01-01".into());
        let sup = Suppressions {
            waivers: vec![w],
            as_of: Some("2027-01-02".into()),
            ..Default::default()
        };
        let out = apply(vec![d("M001", "a::B.c")], &LintConfig::new(), &sup);
        assert!(out.diagnostics[0].waived.is_none());
        assert_eq!(out.expired.len(), 1);
    }
}
