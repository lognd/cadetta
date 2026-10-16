//! frob:ticket 01M4SPWT3FP4SN4RTCHE631Y52
//! The source-to-diagnostics pipeline shared by the CLI verbs: parse, resolve, evaluate,
//! kind-check, lower to IR and run the IR lints (config-cli.md 3, ir.md 1).
//!
//! [`analyze`] is the one entry point; `check` renders its diagnostics, and later verbs
//! (`why`, `impact`, `ir`, `ack`, `obligations`) reuse the lowered packages it returns.
//! Resolver, evaluator and dependency diagnostics carry the module their span lives in
//! (`ModuleKey`), so [`SourceIndex::file_for`] maps them to the exact file; a diagnostic
//! without a module is dropped with a warning (no span guessing).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::cmd::solve::{current_digests, eval_inputs};
use cadetta_ir::entities::Waiver;
use cadetta_ir::model::{
    Citation, CiteBuiltin, ComponentClass, MemberKind as IrMemberKind, Project as IrProject,
};
use cadetta_lint::diag::{Diagnostic, Location, Position, Severity};
use cadetta_lint::engine::{LintConfig, Suppressions};
use cadetta_lint::rules::d::{Key, NamedValue};
use cadetta_lint::rules::e::{self as e_rules, EFinding};
use cadetta_lint::rules::lifecycle::{self, LifecycleFinding, MissingRole, TableMiss};
use cadetta_lint::rules::s::{self, CitationFacts, CitationKind, OtsValueFacts};
use cadetta_lint::rules::{c, d, f, interface, m, n, q, u002, unused};
use cadetta_sema::cite::CiteIndex;
use cadetta_sema::deps::{DepGraph, Renames, Severity as SemaSeverity};
use cadetta_sema::eval::{Evaluator, Selection};
use cadetta_sema::fitderive;
use cadetta_sema::kindcheck::{self, EvalEnv, KindChecker};
use cadetta_sema::lower::{LowerInput, SourceText, lower};
use cadetta_sema::modules::module_path_of;
use cadetta_sema::process::Processes;
use cadetta_sema::renames::Moves;
use cadetta_sema::resolve::{
    CanonPath, Ctx, EnumDecl as SemaEnum, MemberSeg, ModuleKey, ModuleSrc, PackageSrc,
    ResolveError, Resolver,
};
use cadetta_sema::visibility::VisibilityChecker;
use cadetta_solve::decl::lower_solver;
use cadetta_solve::lints as solver_lints;
use cadetta_solve::protocol::Request;
use cadetta_solve::results::{CurrentDigests, Lookup, ResultFile, input_digest, out_path};
use cadetta_solve::variants::{VariantReading, read_variant};
use cadetta_syntax::ast::{File, ItemKind, MemberKind};
use cadetta_syntax::lexer::Span;
use cadetta_syntax::parser::parse;
use cadetta_syntax::span::SourceFile;
use cadetta_units::{KindTable, UnitTable, Verdict};

use crate::config::Lint;
use crate::lint_facts::{self, ModuleView};
use crate::project::{Module, PackageDir, Project};

/// Environment variable naming a directory that holds the `std` package.
pub const STD_ENV: &str = "CADETTA_STD";

/// One indexed source file of the analysed project.
#[derive(Debug)]
pub struct IndexedFile {
    /// The name diagnostics report (relative to the project root when possible).
    pub name: String,
    /// The package the file belongs to.
    pub package: String,
    /// The module path inside the package.
    pub module: Vec<String>,
    /// Whether the package is `std` (its findings are not the project's).
    pub is_std: bool,
    /// The text with a line index.
    pub source: SourceFile,
}

/// Every source file seen by one analysis, for resolving spans to locations.
#[derive(Debug, Default)]
pub struct SourceIndex {
    files: Vec<IndexedFile>,
}

impl SourceIndex {
    /// The indexed files in load order.
    pub fn files(&self) -> &[IndexedFile] {
        &self.files
    }

    /// The text of line `line` (1-based) of the file called `name`.
    pub fn line(&self, name: &str, line: u32) -> Option<String> {
        let f = self.files.iter().find(|f| f.name == name)?;
        f.source
            .text()
            .lines()
            .nth(line.checked_sub(1)? as usize)
            .map(str::to_owned)
    }

    /// The file of module `module` in `package`.
    fn module(&self, package: &str, module: &[String]) -> Option<&IndexedFile> {
        self.files
            .iter()
            .find(|f| f.package == package && f.module == module)
    }

    /// A source location for a byte span of `file`.
    fn location(file: &IndexedFile, span: Span) -> Location {
        let a = file.source.line_col(span.start);
        let b = file.source.line_col(span.end);
        Location::Source {
            file: file.name.clone(),
            start: Position {
                line: a.line as u32,
                col: a.col as u32,
            },
            end: Position {
                line: b.line as u32,
                col: b.col as u32,
            },
        }
    }

    /// Resolve an IR span reference (`file` is an indexed name, offsets are bytes).
    pub fn locate_ref(&self, file: &str, start: u32, end: u32) -> Location {
        match self.files.iter().find(|f| f.name == file) {
            Some(f) => Self::location(
                f,
                Span {
                    start: start as usize,
                    end: end as usize,
                },
            ),
            None => {
                tracing::warn!(file, "pipeline: span in an unindexed file");
                Location::Source {
                    file: file.to_owned(),
                    start: Position { line: 1, col: 1 },
                    end: Position { line: 1, col: 1 },
                }
            }
        }
    }

    /// The project file of a diagnostic's `module` (None for std or an unknown module).
    // frob:ticket 01M4Y66FVYYDJH0FJ16F1T08H3
    fn file_for(&self, key: &ModuleKey) -> Option<&IndexedFile> {
        let file = self.module(&key.package, &key.module).filter(|f| !f.is_std);
        if file.is_none() {
            tracing::debug!(
                package = %key.package,
                module = ?key.module,
                "pipeline: diagnostic module is std or has no file; dropped"
            );
        }
        file
    }
}

/// One package lowered to IR.
#[derive(Debug)]
pub struct LoweredPackage {
    /// The package name.
    pub name: String,
    /// Its IR.
    pub ir: IrProject,
    /// Non-`let` `Text` parameters by canonical path, with their text (the IR keeps no value).
    pub text_params: std::collections::BTreeMap<String, String>,
}

/// Everything one analysis produced.
#[derive(Debug, Default)]
pub struct Analysis {
    /// Raw diagnostics (severity as the producing stage assigned it; run them through
    /// `cadetta_lint::engine::apply` for the effective severity).
    pub diagnostics: Vec<Diagnostic>,
    /// The in-scope packages lowered to IR; empty when parsing failed.
    pub packages: Vec<LoweredPackage>,
    /// The loaded dependency packages outside the scope, lowered but not linted.
    pub dependencies: Vec<LoweredPackage>,
    /// Source files, for rendering excerpts.
    pub sources: SourceIndex,
    /// True when a syntax error stopped the pipeline before evaluation.
    pub halted: bool,
    /// The `waive` declarations of the in-scope packages, for the lint engine.
    pub suppressions: Suppressions,
    /// `(entity path, date)` of every in-scope `#[lifecycle(eol, ..)]`, for W005.
    pub eols: Vec<(String, String)>,
}

/// The `eol` lifecycle dates of the `ots` components of one package, keyed by canonical path.
// frob:ticket 01M4YM8XJ9MWEBA1NTN09X7PNP
fn eol_dates(package: &str, modules: &[Parsed]) -> Vec<(String, String)> {
    use cadetta_variants::lifecycle as vl;
    let mut out = Vec::new();
    for m in modules {
        for item in m.file.items() {
            let ItemKind::Component(c) = &item.kind else {
                continue;
            };
            let Some(lc) = vl::find(&item.attrs).and_then(Result::ok) else {
                continue;
            };
            if lc.status == vl::Status::Eol {
                let path = CanonPath::item(package, &m.path, &c.name.name).to_string();
                tracing::debug!(%path, date = %lc.date, "pipeline: eol date collected");
                out.push((path, lc.date.to_string()));
            }
        }
    }
    out
}

/// The lint-facing view of every module of `p`.
fn module_views(p: &ParsedPackage) -> Vec<ModuleView<'_>> {
    p.modules
        .iter()
        .map(|m| ModuleView {
            path: &m.path,
            file: &m.file,
            text: &m.text,
            display: &m.display,
        })
        .collect()
}

/// Lower the `waive` items of one parsed module to engine waivers (lints.md 2.1).
///
/// A glob whose literal prefix names a path moved by a `was` clause is rewritten to the current
/// path through `moves`, so a waiver written before a rename still matches.
// frob:ticket 01M4YM8XJ9MWEBA1NTN09X7PNP
fn module_waivers(file: &File, moves: &Moves, out: &mut Vec<Waiver>) {
    for item in file.items() {
        let ItemKind::Waive(w) = &item.kind else {
            continue;
        };
        // frob:ticket 01M4ZJAYRJMG47610S0R0ANADA
        match cadetta_sema::lower::lower_waiver(w) {
            Ok(mut waiver) => {
                tracing::debug!(code = %w.code.name, glob = %w.glob.text, "pipeline: waiver collected");
                waiver.glob = moves.carry_glob(&w.glob.text);
                out.push(waiver);
            }
            Err(error) => {
                tracing::warn!(code = %w.code.name, %error, "pipeline: waiver with a bad `for alt`; skipped");
            }
        }
    }
}

/// Recorded primary-result state of every solver site, for `ir.solvers` (solvers.md 4).
///
/// A fresh or stale result carries a digest of its recorded outputs; a missing one carries none.
// frob:ticket 01M4ZJAYRJMG47610S0R0ANADA
fn solver_states(
    sites: &[SolverSite<'_>],
) -> BTreeMap<CanonPath, cadetta_sema::lower::SolverState> {
    sites
        .iter()
        .map(|s| {
            let (result, stale) = match s.results.lookup(&s.key, None, &s.current) {
                Lookup::Fresh(r) => (Some(r), false),
                Lookup::Stale(r) => (Some(r), true),
                Lookup::Missing => (None, false),
            };
            let result_digest = result.map(|r| {
                let text: String = r
                    .outputs
                    .iter()
                    .map(|(k, v)| format!("{k}={v}\n"))
                    .collect();
                cadetta_ir::digest::digest_bytes(text.as_bytes())
            });
            tracing::debug!(solver = %s.path, stale, recorded = result_digest.is_some(), "pipeline: solver state");
            (
                s.path.clone(),
                cadetta_sema::lower::SolverState {
                    result_digest,
                    stale,
                },
            )
        })
        .collect()
}

/// S001/S002/S008 inputs of one lowered package: `ots` values, and the package's own `cite`
/// items with the filesystem answer taken against the package directory.
// frob:ticket 01M4VB63M63PQVRKN0HDXQP567
pub(crate) fn s_facts(
    ir: &IrProject,
    package: &str,
    dir: Option<&Path>,
) -> (Vec<OtsValueFacts>, Vec<CitationFacts>, Vec<Citation>) {
    let prefix = format!("{package}::");
    let own: Vec<Citation> = ir
        .citations
        .iter()
        .filter(|c| c.path.starts_with(&prefix))
        .cloned()
        .collect();
    let cites = own
        .iter()
        .filter_map(|c| {
            let target = c.file()?.to_owned();
            let exists = dir.is_none_or(|d| d.join(&target).exists());
            Some(CitationFacts {
                path: c.path.clone(),
                kind: CitationKind::LocalFile,
                target,
                exists,
                span: None,
            })
        })
        .collect();
    let mut values = Vec::new();
    for c in &ir.components {
        if c.class != ComponentClass::Ots {
            continue;
        }
        for m in &c.members {
            // Datum offsets of an `ots` are catalogue values too (geometry.md 2).
            // frob:ticket 01M4V43BDMB7CKR8W79S5DC5W0
            let is_value = matches!(m.kind, IrMemberKind::Param | IrMemberKind::Datum);
            if is_value && m.value.is_some() {
                // A member citation replaces the component's, never merges (language.md 3.9).
                let applied = m.citation.as_ref().or(c.citation.as_ref());
                values.push(OtsValueFacts {
                    path: m.path.to_string(),
                    component: c.path.to_string(),
                    component_span: c.span.clone(),
                    cited: m.citation.is_some(),
                    component_cited: c.citation.is_some(),
                    note_only: applied.is_some_and(|r| r.builtin == Some(CiteBuiltin::Note)),
                    span: m.span.clone(),
                });
            }
        }
    }
    (values, cites, own)
}

/// A parsed module with everything the later stages borrow.
struct Parsed {
    path: Vec<String>,
    file: File,
    /// Always empty: the parser reads `enum` items, so the resolver indexes them itself.
    enums: Vec<SemaEnum>,
    text: String,
    display: String,
}

/// A package's raw files, whether from a validated [`PackageDir`] or the `std` tree.
struct RawPackage {
    name: String,
    version: String,
    deps: Vec<String>,
    /// The package's `[defaults]` table (empty for `std`).
    defaults: BTreeMap<String, String>,
    dir: PathBuf,
    modules: Vec<Module>,
}

impl RawPackage {
    fn of(pkg: &PackageDir) -> Self {
        Self {
            name: package_name(pkg),
            version: pkg
                .config
                .package
                .as_ref()
                .and_then(|p| p.version.clone())
                .unwrap_or_else(|| "0.0.0".to_owned()),
            deps: pkg.config.dependencies.keys().cloned().collect(),
            defaults: pkg.config.defaults.clone(),
            dir: pkg.dir.clone(),
            modules: pkg.modules.clone(),
        }
    }
}

fn collect_cdt(base: &Path, dir: &Path, out: &mut Vec<Module>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_cdt(base, &path, out)?;
        } else if path.extension().is_some_and(|e| e == "cdt") {
            let rel_path = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let source = std::fs::read_to_string(&path)?;
            out.push(Module {
                rel_path,
                file: path,
                source,
            });
        }
    }
    Ok(())
}

/// Read the `std` package tree directly: its name is reserved, so config validation
/// (which rejects `std` for user packages) is bypassed.
fn load_std(dir: &Path) -> std::io::Result<RawPackage> {
    let mut modules = Vec::new();
    collect_cdt(&dir.join("src"), &dir.join("src"), &mut modules)?;
    modules.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(RawPackage {
        name: "std".to_owned(),
        version: "0.1.0".to_owned(),
        deps: Vec::new(),
        defaults: BTreeMap::new(),
        dir: dir.to_owned(),
        modules,
    })
}

struct ParsedPackage {
    name: String,
    version: String,
    deps: Vec<String>,
    defaults: BTreeMap<String, String>,
    dir: PathBuf,
    is_std: bool,
    modules: Vec<Parsed>,
}

/// The `std` package: `$CADETTA_STD` when it names a std directory, else the copy
/// embedded in the binary.
fn load_std_package() -> Option<RawPackage> {
    let from_env = std::env::var_os(STD_ENV)
        .map(PathBuf::from)
        .filter(|d| d.join("cadetta.toml").is_file());
    if let Some(dir) = from_env {
        tracing::debug!(dir = %dir.display(), "pipeline: std from CADETTA_STD");
        return match load_std(&dir) {
            Ok(p) => Some(p),
            Err(error) => {
                tracing::warn!(%error, "pipeline: std package could not be loaded");
                None
            }
        };
    }
    tracing::debug!("pipeline: std from the embedded copy");
    Some(embedded_std())
}

/// The `std` package built from the sources compiled into the binary.
fn embedded_std() -> RawPackage {
    let dir = PathBuf::from("<embedded>/std");
    let mut modules: Vec<Module> = crate::std_embed::STD_FILES
        .iter()
        .map(|(rel, text)| Module {
            rel_path: (*rel).to_owned(),
            file: dir.join("src").join(rel),
            source: (*text).to_owned(),
        })
        .collect();
    modules.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    RawPackage {
        name: "std".to_owned(),
        version: "0.1.0".to_owned(),
        deps: Vec::new(),
        defaults: BTreeMap::new(),
        dir,
        modules,
    }
}

fn package_name(pkg: &PackageDir) -> String {
    pkg.config
        .package
        .as_ref()
        .and_then(|p| p.name.clone())
        .unwrap_or_else(|| "unnamed".to_owned())
}

fn display_name(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Lifts a located resolver error through `e::lift_resolution` (E002-E011), keeping the
/// resolver's own message text and adding the related spans (E003, E004) as notes.
// frob:ticket 01M4YMYMB1NKFEWKHDYGGJ41H3
fn lift_resolve_error(
    err: &ResolveError,
    base: Option<Diagnostic>,
    index: &SourceIndex,
) -> Option<Diagnostic> {
    let base = base?;
    let base_path = base.path.clone();
    let mut finding = EFinding::new(
        err.code,
        base.path.as_deref().unwrap_or_default(),
        &err.message,
        base.location.clone(),
    );
    if let Some(file) = err.module.as_deref().and_then(|k| index.file_for(k)) {
        for span in &err.related {
            finding = finding.with_related(SourceIndex::location(file, *span));
        }
    }
    let lifted = e_rules::lift_resolution(&[finding], &LintConfig::new()).pop();
    Some(match lifted {
        Some(mut d) => {
            // The resolver names the exact offender; the catalog template would only generalise it.
            d.message = err.message.clone();
            // frob:ticket 01M51NC9FMMRMKC3NVMJKF14FE
            // Keep the base path (None stays None) so the twin dedup still matches.
            d.path = base_path;
            d
        }
        None => {
            tracing::debug!(
                code = err.code,
                "pipeline: not an E001-E011 code; kept as located"
            );
            base
        }
    })
}

fn diag_at(
    code: &str,
    severity: Severity,
    message: impl Into<String>,
    path: Option<String>,
    location: Option<Location>,
) -> Diagnostic {
    let mut d = Diagnostic::new(code, severity, message);
    d.path = path;
    d.location = location;
    d
}

/// M001 diagnostic for a `[lint] precision` key that names no built-in or package kind.
// frob:ticket 01M5137DWGRARW5K0EMRB93QEK
fn unknown_precision_diag(package: &str, kind: &str) -> Diagnostic {
    diag_at(
        "M001",
        Severity::Warning,
        format!(
            "`[lint] precision` names unknown quantity kind `{kind}`; it is ignored (package `{package}`)"
        ),
        Some("cadetta.toml".to_owned()),
        None,
    )
}

fn sema_severity(s: SemaSeverity) -> Severity {
    match s {
        SemaSeverity::Error => Severity::Error,
        SemaSeverity::Warning => Severity::Warning,
    }
}

/// True when an evaluator quantity report (`EVAL`/`Q001`..`Q003`) overlaps a kind-checker
/// diagnostic in the same module, so the one underlying fault is reported once.
// frob:ticket 01M4V1HDJS2MDQF8GRK4QX9B1C
fn shadowed_by_kind_diag(
    d: &cadetta_sema::eval::EvalDiagnostic,
    kind_spans: &[(ModuleKey, usize, usize)],
) -> bool {
    if !matches!(d.code, "EVAL" | "Q001" | "Q002" | "Q003") {
        return false;
    }
    kind_spans
        .iter()
        .any(|(k, s, e)| d.module.as_ref() == Some(k) && d.span.start < *e && *s < d.span.end)
}

/// The member of a component whose name-to-value region covers `span`, as a canonical path.
fn member_path_at(pkg: &str, module: &[String], file: &File, span: Span) -> Option<String> {
    for item in file.items() {
        let ItemKind::Component(c) = &item.kind else {
            continue;
        };
        for m in &c.members {
            let (name, value) = match &m.kind {
                MemberKind::Param(p) => (&p.name, &p.value),
                MemberKind::Let(l) => (&l.name, &l.value),
                MemberKind::Check(c) => (&c.name, &c.cond),
                MemberKind::Require(r) => (&r.name, &r.cond),
                _ => continue,
            };
            if name.span.start <= span.start && span.end <= value.span.end {
                let mut p = CanonPath::item(pkg, module, &c.name.name);
                p.members.push(MemberSeg::Name(name.name.clone()));
                return Some(p.to_string());
            }
        }
    }
    None
}

/// Convert a lexer fix-it (byte span and replacement) to a lint fix over `file`.
// frob:ticket 01M4TRKDAC6FR44KEW64H7PTZX
fn syntax_fix(file: &IndexedFile, fix: &cadetta_syntax::lexer::Fix) -> cadetta_lint::diag::Fix {
    let Location::Source { start, end, .. } = SourceIndex::location(file, fix.span) else {
        unreachable!("SourceIndex::location always yields a source location");
    };
    cadetta_lint::diag::Fix {
        title: if fix.replacement.is_empty() {
            "delete the text".to_owned()
        } else {
            format!("replace with `{}`", fix.replacement)
        },
        edits: vec![cadetta_lint::diag::Edit {
            file: file.name.clone(),
            range: cadetta_lint::diag::Range { start, end },
            text: fix.replacement.clone(),
        }],
    }
}

fn parse_package(
    pkg: &RawPackage,
    root: &Path,
    is_std: bool,
    out: &mut Vec<Diagnostic>,
    halted: &mut bool,
    index: &mut SourceIndex,
) -> ParsedPackage {
    let name = pkg.name.clone();
    let version = pkg.version.clone();
    let mut modules = Vec::new();
    for module in &pkg.modules {
        let path = match module_path_of(&module.rel_path) {
            Ok(p) => p,
            Err(error) => {
                tracing::error!(file = %module.file.display(), %error, "pipeline: bad module path");
                out.push(diag_at(
                    error.code,
                    Severity::Error,
                    error.message.clone(),
                    None,
                    None,
                ));
                *halted = true;
                continue;
            }
        };
        let shown = display_name(root, &module.file);
        let source = SourceFile::new(shown.clone(), module.source.clone());
        let parsed = parse(&module.source);
        let here = IndexedFile {
            name: shown.clone(),
            package: name.clone(),
            module: path.clone(),
            is_std,
            source: source.clone(),
        };
        let lcfg = LintConfig::new();
        let mut qfindings = Vec::new();
        for d in &parsed.diags {
            tracing::debug!(file = %shown, code = d.code.code(), "pipeline: syntax diagnostic");
            let location = SourceIndex::location(&here, d.span);
            if d.code.code() == "E001" {
                *halted = true;
                out.push(diag_at(
                    d.code.code(),
                    Severity::Error,
                    d.message.clone(),
                    None,
                    Some(location),
                ));
                continue;
            }
            // Lexer-found Q findings (Q007) go through the Q lift for catalog severity and fix-it.
            qfindings.push(q::QFinding {
                code: d.code.code().to_owned(),
                message: d.message.clone(),
                path: None,
                location: Some(location),
                fix: d.fix.as_ref().map(|f| syntax_fix(&here, f)),
            });
        }
        out.extend(q::lift(&qfindings, &lcfg));
        index.files.push(IndexedFile {
            name: shown.clone(),
            package: name.clone(),
            module: path.clone(),
            is_std,
            source,
        });
        modules.push(Parsed {
            path,
            file: parsed.file,
            enums: Vec::new(),
            text: module.source.clone(),
            display: shown,
        });
    }
    ParsedPackage {
        name,
        version,
        deps: pkg.deps.clone(),
        defaults: pkg.defaults.clone(),
        dir: pkg.dir.clone(),
        is_std,
        modules,
    }
}

/// Compile the packages of `project` selected by `scope` and gather every diagnostic.
///
/// Stages: parse (a lexical error halts), resolve, evaluate, kind-check, dependency
/// graph, lower the in-scope packages to IR, then the IR lints and check verdicts.
pub fn analyze(project: &Project, scope: &[usize]) -> Analysis {
    let config_as_of = project
        .config
        .package
        .as_ref()
        .and_then(|p| p.as_of.as_deref());
    let as_of = cadetta_obligations::as_of::AsOf::resolve(
        None,
        config_as_of,
        cadetta_obligations::as_of::Date::today(),
    )
    .map_or_else(
        |_| cadetta_obligations::as_of::Date::today().to_string(),
        |a| a.date.to_string(),
    );
    analyze_at(project, scope, &as_of)
}

/// Analyse with an explicit `--as-of` date (`YYYY-MM-DD`), which L001 reads.
// frob:ticket 01M4TRKDAC6FR44KEW64H7PTZX
pub fn analyze_at(project: &Project, scope: &[usize], as_of: &str) -> Analysis {
    let mut analysis = Analysis::default();
    let mut halted = false;
    let mut parsed: Vec<ParsedPackage> = Vec::new();
    let mut loaded_std: Option<RawPackage> = None;
    let has_std_member = project.packages.iter().any(|p| package_name(p) == "std");
    if !has_std_member {
        loaded_std = load_std_package();
    }
    for pkg in &project.packages {
        parsed.push(parse_package(
            &RawPackage::of(pkg),
            &project.root,
            false,
            &mut analysis.diagnostics,
            &mut halted,
            &mut analysis.sources,
        ));
    }
    for dep in &project.dependencies {
        tracing::debug!(name = %dep.resolved.name, "pipeline: parsing dependency package");
        parsed.push(parse_package(
            &RawPackage::of(&dep.package),
            &project.root,
            false,
            &mut analysis.diagnostics,
            &mut halted,
            &mut analysis.sources,
        ));
    }
    if let Some(std_pkg) = &loaded_std {
        let std_root = std_pkg.dir.parent().unwrap_or(&std_pkg.dir).to_owned();
        let mut scratch = Vec::new();
        let mut std_halted = false;
        let p = parse_package(
            std_pkg,
            &std_root,
            true,
            &mut scratch,
            &mut std_halted,
            &mut analysis.sources,
        );
        if std_halted {
            tracing::warn!("pipeline: std has syntax errors; continuing without it");
        } else {
            let mut p = p;
            p.is_std = true;
            parsed.push(p);
        }
    }
    if halted {
        tracing::info!("pipeline: syntax errors; skipping evaluation");
        analysis.halted = true;
        return analysis;
    }
    let scope_names: BTreeSet<String> = scope
        .iter()
        .map(|&i| package_name(&project.packages[i]))
        .collect();
    evaluate_stages(
        &parsed,
        &scope_names,
        &project.config.lint,
        as_of,
        &mut analysis,
    );
    analysis
}

/// Every quantity-valued table cell of the parsed packages, by package name, named
/// `Table[key].column` for D002 (the IR carries no cells, lints.md D).
fn table_cells(
    evaluator: &mut Evaluator<'_, '_>,
    parsed: &[ParsedPackage],
) -> BTreeMap<String, Vec<NamedValue>> {
    let mut out: BTreeMap<String, Vec<NamedValue>> = BTreeMap::new();
    for p in parsed {
        for m in &p.modules {
            let ctx = Ctx {
                package: p.name.clone(),
                module: m.path.clone(),
                component: None,
                locals: Vec::new(),
            };
            for item in m.file.items() {
                let ItemKind::Table(t) = &item.kind else {
                    continue;
                };
                for row in &t.rows {
                    let key_text = m
                        .text
                        .get(row.key.span.start..row.key.span.end)
                        .unwrap_or("?");
                    for (col, expr) in t.cols.iter().zip(row.values.iter().flatten()) {
                        let Ok(q) = evaluator.eval_input(ctx.clone(), expr) else {
                            tracing::trace!(table = %t.name.name, "pipeline: table cell is not a quantity");
                            continue;
                        };
                        out.entry(p.name.clone()).or_default().push(NamedValue {
                            name: format!("{}[{key_text}].{}", t.name.name, col.name.name),
                            key: Key::new(q.dim(), &q.nominal),
                        });
                    }
                }
            }
        }
    }
    tracing::debug!(
        cells = out.values().map(Vec::len).sum::<usize>(),
        "pipeline: table cells collected for D002"
    );
    out
}

/// What variant evaluation needs: the resolver, units and sources, plus a per-swap cache.
struct VariantEnv<'a, 'p> {
    resolver: &'a Resolver<'p>,
    units: &'a UnitTable,
    sources: &'a [PackageSrc<'a>],
    solvers: &'a [SolverSite<'a>],
    cache: std::cell::RefCell<BTreeMap<String, std::rc::Rc<cadetta_sema::eval::Evaluation>>>,
    unsolved: std::cell::RefCell<Vec<UnsolvedVariant>>,
}

impl<'a, 'p> VariantEnv<'a, 'p> {
    /// Builds the environment with an empty cache.
    fn new(
        resolver: &'a Resolver<'p>,
        units: &'a UnitTable,
        sources: &'a [PackageSrc<'a>],
        solvers: &'a [SolverSite<'a>],
    ) -> Self {
        Self {
            resolver,
            units,
            sources,
            solvers,
            cache: std::cell::RefCell::new(BTreeMap::new()),
            unsolved: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// The recorded outputs each solver reads under `selection` (solvers.md 4.1).
    ///
    /// A variant whose inputs equal the primary's reads the primary's result; one with changed
    /// inputs reads its own, and with none its outputs stay unknown and S006 is recorded for
    /// in-scope solvers. Solvers whose inputs do not evaluate under the swap are skipped.
    // frob:ticket 01M50M66PAWQ2N9P3HRB645GTM
    fn variant_outputs(
        &self,
        selection: &Selection,
    ) -> BTreeMap<CanonPath, BTreeMap<String, String>> {
        let variant = selection.key();
        let mut out = BTreeMap::new();
        for site in self.solvers {
            let mut probe = Evaluator::new(self.resolver, self.units, self.sources)
                .with_selection(selection.clone());
            let Ok(inputs) = eval_inputs(site.ast, &site.ctx, &mut probe, self.units) else {
                tracing::debug!(solver = %site.key, %variant, "pipeline: variant inputs unevaluable; skipped");
                continue;
            };
            let digest = input_digest(&Request::new(site.key.clone(), inputs));
            match read_variant(&site.results, &site.key, &variant, &site.current, &digest) {
                VariantReading::Outputs(r) => {
                    tracing::debug!(solver = %site.key, %variant, "pipeline: variant outputs recorded");
                    out.insert(site.path.clone(), r.outputs.clone());
                }
                VariantReading::Unknown => {
                    if site.in_scope {
                        self.unsolved.borrow_mut().push(UnsolvedVariant {
                            key: site.module.clone(),
                            span: site.ast.name.span,
                            diag: solver_lints::unsolved_variant(&site.key, &variant),
                        });
                    }
                }
            }
        }
        out
    }

    /// The design evaluated with `alt` swapped into `owner`'s `slot`, cached per swap.
    ///
    /// Returns the swap key with the evaluation, or `None` when the alternate does not resolve.
    // frob:ticket 01M4Z3FGQHHAPQ3N8VCDEY1G2X
    fn evaluation_for(
        &self,
        owner: &CanonPath,
        slot: &str,
        package: &str,
        module: &[String],
        alt: &cadetta_syntax::ast::Ctor,
    ) -> Option<(String, std::rc::Rc<cadetta_sema::eval::Evaluation>)> {
        let Some((alt_path, _)) = self.resolver.ctor_component(package, module, alt) else {
            tracing::debug!(slot, "pipeline: alternate does not resolve; not evaluated");
            return None;
        };
        let selection = Selection {
            owner: owner.clone(),
            slot: slot.to_owned(),
            alt: alt_path,
        };
        let key = selection.key();
        if let Some(cached) = self.cache.borrow().get(&key) {
            return Some((key, cached.clone()));
        }
        let outputs = self.variant_outputs(&selection);
        let evaluator = Evaluator::new(self.resolver, self.units, self.sources)
            .with_selection(selection)
            .with_solver_outputs(outputs);
        let evaluation = std::rc::Rc::new(evaluator.evaluate());
        let violated: Vec<String> = evaluation
            .checks
            .iter()
            .filter(|r| r.verdict == Verdict::Violates)
            .map(|r| r.path.to_string())
            .collect();
        tracing::info!(variant = %key, viable = violated.is_empty(), ?violated, "pipeline: alternate viability evaluated");
        self.cache
            .borrow_mut()
            .insert(key.clone(), evaluation.clone());
        Some((key, evaluation))
    }

    /// The S006 findings gathered while evaluating swaps, deduplicated and drained.
    fn take_unsolved(&self) -> Vec<UnsolvedVariant> {
        let mut found = std::mem::take(&mut *self.unsolved.borrow_mut());
        found.sort_by(|a, b| (&a.diag.path, &a.diag.alt).cmp(&(&b.diag.path, &b.diag.alt)));
        found.dedup_by(|a, b| a.diag.path == b.diag.path && a.diag.alt == b.diag.alt);
        found
    }

    /// True when no check or require violates with `alt` swapped into `owner`'s `slot`.
    fn alternate_holds(
        &self,
        owner: &CanonPath,
        slot: &str,
        package: &str,
        module: &[String],
        alt: &cadetta_syntax::ast::Ctor,
    ) -> bool {
        self.evaluation_for(owner, slot, package, module, alt)
            .is_none_or(|(_, e)| e.checks.iter().all(|r| r.verdict != Verdict::Violates))
    }
}

/// Evaluations of every one-alternate-at-a-time swap in the in-scope packages, by variant key.
///
/// Feeds `Check.verdicts.per_alternate` so I002/I003 see each alternate (language.md 3.2).
// frob:ticket 01M4Z3FGQHHAPQ3N8VCDEY1G2X
fn alternate_evaluations(
    parsed: &[ParsedPackage],
    scope: &BTreeSet<String>,
    env: &VariantEnv<'_, '_>,
) -> BTreeMap<String, std::rc::Rc<cadetta_sema::eval::Evaluation>> {
    let mut out = BTreeMap::new();
    for p in parsed
        .iter()
        .filter(|p| !p.is_std && scope.contains(&p.name))
    {
        for m in &p.modules {
            for item in m.file.items() {
                let ItemKind::Component(c) = &item.kind else {
                    continue;
                };
                let owner = CanonPath::item(&p.name, &m.path, &c.name.name);
                for mem in &c.members {
                    let MemberKind::Part(part) = &mem.kind else {
                        continue;
                    };
                    for alt in &part.alts {
                        if let Some((key, e)) =
                            env.evaluation_for(&owner, &part.name.name, &p.name, &m.path, alt)
                        {
                            out.insert(key, e);
                        }
                    }
                }
            }
        }
    }
    tracing::debug!(
        variants = out.len(),
        "pipeline: alternate evaluations collected"
    );
    out
}

/// L001/L002: lifecycle facts for the primary of every part slot in scope.
///
/// An alternate counts as viable when it names a known `ots` component that is not itself
/// end-of-life and the design evaluated under that alternate's slot swap violates no check or
/// require (language.md 3.2); undecided results keep it viable. Variants are evaluated only for
/// primaries whose lifecycle can raise a fact, and once per distinct swap.
// frob:ticket 01M4YMYMD920CQR97CW6QJCHN0
fn lifecycle_findings(
    parsed: &[ParsedPackage],
    scope: &BTreeSet<String>,
    as_of: &str,
    index: &SourceIndex,
    env: &VariantEnv<'_, '_>,
) -> Vec<LifecycleFinding> {
    use cadetta_variants::lifecycle as vl;
    let Some(as_of) = vl::LifecycleDate::parse(as_of) else {
        tracing::warn!(as_of, "pipeline: unparsable as-of date; L001/L002 skipped");
        return Vec::new();
    };
    // ots component name -> (package, its lifecycle)
    let mut ots: BTreeMap<&str, Vec<(&str, Option<vl::Lifecycle>)>> = BTreeMap::new();
    for p in parsed {
        for m in &p.modules {
            for item in m.file.items() {
                if let ItemKind::Component(c) = &item.kind
                    && c.class == cadetta_syntax::ast::ComponentClass::Ots
                {
                    let lc = vl::find(&item.attrs).and_then(Result::ok);
                    ots.entry(c.name.name.as_str())
                        .or_default()
                        .push((p.name.as_str(), lc));
                }
            }
        }
    }
    let lookup = |pkg: &str, ctor: &cadetta_syntax::ast::Ctor| {
        let name = ctor.path.segments.last()?.name.as_str();
        let all = ots.get(name)?;
        all.iter().find(|(p, _)| *p == pkg).or_else(|| all.first())
    };
    let mut out = Vec::new();
    for p in parsed
        .iter()
        .filter(|p| !p.is_std && scope.contains(&p.name))
    {
        for m in &p.modules {
            for item in m.file.items() {
                let ItemKind::Component(c) = &item.kind else {
                    continue;
                };
                for mem in &c.members {
                    let MemberKind::Part(part) = &mem.kind else {
                        continue;
                    };
                    let Some(primary) = &part.primary else {
                        continue;
                    };
                    let Some((_, lc)) = lookup(&p.name, primary) else {
                        continue;
                    };
                    // Only a non-active primary can raise L001/L002; skip the swaps otherwise.
                    let needs_variants =
                        lc.as_ref().is_some_and(|l| l.status != vl::Status::Active);
                    let owner = CanonPath::item(&p.name, &m.path, &c.name.name);
                    let alternates: Vec<vl::Alternate> = part
                        .alts
                        .iter()
                        .map(|a| {
                            let name = a
                                .path
                                .segments
                                .last()
                                .map_or_else(String::new, |s| s.name.clone());
                            let found = lookup(&p.name, a);
                            let lifecycle = found.and_then(|(_, l)| *l);
                            let mut viable = found.is_some()
                                && !lifecycle
                                    .as_ref()
                                    .is_some_and(|l| l.status.is_end_of_life());
                            if viable && needs_variants {
                                viable = env.alternate_holds(
                                    &owner,
                                    &part.name.name,
                                    &p.name,
                                    &m.path,
                                    a,
                                );
                            }
                            vl::Alternate {
                                name,
                                lifecycle,
                                viable,
                            }
                        })
                        .collect();
                    let pname = primary
                        .path
                        .segments
                        .last()
                        .map_or_else(String::new, |s| s.name.clone());
                    let Some(fact) = vl::analyse(&pname, lc.as_ref(), &alternates, &as_of) else {
                        continue;
                    };
                    let mut path = CanonPath::item(&p.name, &m.path, &c.name.name);
                    path.members.push(MemberSeg::Name(part.name.name.clone()));
                    out.push(LifecycleFinding {
                        code: fact.code.to_owned(),
                        path: path.to_string(),
                        status: fact.status.as_str().to_owned(),
                        date: fact.date.to_string(),
                        date_passed: fact.date_passed,
                        candidates: fact.candidates,
                        location: index
                            .module(&p.name, &m.path)
                            .map(|f| SourceIndex::location(f, primary.span)),
                    });
                }
            }
        }
    }
    out
}

/// T001: `ots` literals that equal no key and no cell of the table(s) sharing their citation.
///
/// Only non-parameterized `ots` components are checked, a literal is compared only against
/// tables holding a value of its dimension, and citations match by written path.
// frob:ticket 01M4TRKDAC6FR44KEW64H7PTZX
fn table_misses(
    evaluator: &mut Evaluator<'_, '_>,
    parsed: &[ParsedPackage],
    scope: &BTreeSet<String>,
    index: &SourceIndex,
) -> Vec<TableMiss> {
    let cite_text = |c: &cadetta_syntax::ast::Citation| match &c.name {
        cadetta_syntax::ast::CiteName::Named(p) => Some(
            p.segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join("::"),
        ),
        cadetta_syntax::ast::CiteName::Literal(_) => None,
    };
    // citation text -> (table name, every key and cell value)
    let mut tables: BTreeMap<String, Vec<(String, Vec<Key>)>> = BTreeMap::new();
    for p in parsed {
        for m in &p.modules {
            let ctx = Ctx {
                package: p.name.clone(),
                module: m.path.clone(),
                component: None,
                locals: Vec::new(),
            };
            for item in m.file.items() {
                let ItemKind::Table(t) = &item.kind else {
                    continue;
                };
                let Some(cite) = t.citation.as_ref().and_then(cite_text) else {
                    continue;
                };
                let mut keys = Vec::new();
                for row in &t.rows {
                    for expr in std::iter::once(&row.key).chain(row.values.iter().flatten()) {
                        if let Ok(q) = evaluator.eval_input(ctx.clone(), expr) {
                            keys.push(Key::new(q.dim(), &q.nominal));
                        }
                    }
                }
                tables
                    .entry(cite)
                    .or_default()
                    .push((t.name.name.clone(), keys));
            }
        }
    }
    let mut out = Vec::new();
    for p in parsed
        .iter()
        .filter(|p| !p.is_std && scope.contains(&p.name))
    {
        for m in &p.modules {
            let ctx = Ctx {
                package: p.name.clone(),
                module: m.path.clone(),
                component: None,
                locals: Vec::new(),
            };
            for item in m.file.items() {
                let ItemKind::Component(c) = &item.kind else {
                    continue;
                };
                if c.class != cadetta_syntax::ast::ComponentClass::Ots || !c.params.is_empty() {
                    continue;
                }
                for mem in &c.members {
                    let MemberKind::Param(par) = &mem.kind else {
                        continue;
                    };
                    let Some(cite) = par
                        .citation
                        .as_ref()
                        .or(c.citation.as_ref())
                        .and_then(cite_text)
                    else {
                        continue;
                    };
                    let Some(cands) = tables.get(&cite) else {
                        continue;
                    };
                    let Ok(q) = evaluator.eval_input(ctx.clone(), &par.value) else {
                        continue;
                    };
                    let lit = Key::new(q.dim(), &q.nominal);
                    let like: Vec<_> = cands
                        .iter()
                        .filter(|(_, ks)| ks.iter().any(|k| k.same_dim(&lit)))
                        .collect();
                    if like.is_empty() || like.iter().any(|(_, ks)| ks.contains(&lit)) {
                        continue;
                    }
                    let mut path = CanonPath::item(&p.name, &m.path, &c.name.name);
                    path.members.push(MemberSeg::Name(par.name.name.clone()));
                    tracing::debug!(path = %path, table = %like[0].0, "pipeline: T001 literal matches no row");
                    out.push(TableMiss {
                        path: path.to_string(),
                        table: like[0].0.clone(),
                        column: par.name.name.clone(),
                        location: index
                            .module(&p.name, &m.path)
                            .map(|f| SourceIndex::location(f, par.value.span)),
                    });
                }
            }
        }
    }
    out
}

/// The committed `.out.json` outputs of every solver member, by solver path (solvers.md 3, 4).
// frob:ticket 01M4YRH3G6F8EGQ5X5CK4JKH27
fn recorded_solver_outputs(
    parsed: &[ParsedPackage],
) -> BTreeMap<CanonPath, BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for pkg in parsed {
        for m in &pkg.modules {
            for c in m.file.components() {
                for member in &c.members {
                    let MemberKind::Solver(sv) = &member.kind else {
                        continue;
                    };
                    let mut path = CanonPath::item(&pkg.name, &m.path, &c.name.name);
                    path.members.push(MemberSeg::Name(sv.name.name.clone()));
                    let file = out_path(&pkg.dir.join(&sv.script));
                    let results = match ResultFile::load(&file) {
                        Ok(r) => r,
                        Err(error) => {
                            tracing::warn!(solver = %path, %error, "pipeline: solver result file unreadable");
                            continue;
                        }
                    };
                    let key = path.to_string();
                    let Some(r) = results
                        .results
                        .iter()
                        .find(|r| r.solver == key && r.variant.is_none())
                    else {
                        tracing::debug!(solver = %path, "pipeline: no recorded result for solver");
                        continue;
                    };
                    tracing::debug!(solver = %path, outputs = r.outputs.len(), "pipeline: solver outputs loaded");
                    out.insert(path, r.outputs.clone());
                }
            }
        }
    }
    out
}

/// A solver member with its recorded results and current primary digests, for staleness and
/// variant readings (solvers.md 4, 4.1).
struct SolverSite<'a> {
    path: CanonPath,
    key: String,
    ast: &'a cadetta_syntax::ast::Solver,
    ctx: Ctx,
    module: ModuleKey,
    in_scope: bool,
    results: ResultFile,
    current: CurrentDigests,
}

/// The solver members whose result file, declaration, inputs, script and deps all load.
///
/// A solver whose inputs, script or declaration cannot be evaluated is reported by the stages
/// that own those faults and is left out here.
// frob:ticket 01M4V1HYS6PKK0F82XMJET4TC0
// frob:ticket 01M50M66PAWQ2N9P3HRB645GTM
fn solver_sites<'a>(
    parsed: &'a [ParsedPackage],
    scope: &BTreeSet<String>,
    evaluator: &mut Evaluator<'_, '_>,
    units: &UnitTable,
) -> Vec<SolverSite<'a>> {
    let mut out = Vec::new();
    for pkg in parsed.iter().filter(|p| !p.is_std) {
        for m in &pkg.modules {
            for c in m.file.components() {
                for member in &c.members {
                    let MemberKind::Solver(sv) = &member.kind else {
                        continue;
                    };
                    let mut path = CanonPath::item(&pkg.name, &m.path, &c.name.name);
                    path.members.push(MemberSeg::Name(sv.name.name.clone()));
                    let key = path.to_string();
                    let Ok(results) = ResultFile::load(&out_path(&pkg.dir.join(&sv.script))) else {
                        continue;
                    };
                    let Ok(decl) = lower_solver(sv, &key) else {
                        continue;
                    };
                    let ctx = Ctx {
                        package: pkg.name.clone(),
                        module: m.path.clone(),
                        component: Some(c.name.name.clone()),
                        locals: Vec::new(),
                    };
                    let Ok(inputs) = eval_inputs(sv, &ctx, evaluator, units) else {
                        tracing::debug!(solver = %key, "pipeline: solver inputs unevaluable; staleness unjudged");
                        continue;
                    };
                    let request = Request::new(key.clone(), inputs);
                    let Ok(current) = current_digests(&request, &pkg.dir, &decl) else {
                        tracing::debug!(solver = %key, "pipeline: solver script or dep unreadable; staleness unjudged");
                        continue;
                    };
                    out.push(SolverSite {
                        path,
                        key,
                        ast: sv,
                        ctx,
                        module: ModuleKey::new(&pkg.name, &m.path),
                        in_scope: scope.contains(&pkg.name),
                        results,
                        current,
                    });
                }
            }
        }
    }
    out
}

/// A solver whose recorded primary result is stale (S003), before it is located in a file.
struct StaleSolver {
    key: ModuleKey,
    span: Span,
    diag: Diagnostic,
}

/// S003: every in-scope solver member whose recorded primary result no longer matches the
/// current input, script or dep digests (solvers.md 4); its outputs still evaluate.
///
/// A solver with no recorded result is unsolved, not stale.
// frob:ticket 01M4V1HYS6PKK0F82XMJET4TC0
fn stale_solvers(sites: &[SolverSite<'_>]) -> Vec<StaleSolver> {
    sites
        .iter()
        .filter(|s| s.in_scope)
        .filter_map(|s| {
            let lookup = s.results.lookup(&s.key, None, &s.current);
            solver_lints::lint_lookup(&s.key, None, &lookup, &s.current).map(|diag| StaleSolver {
                key: s.module.clone(),
                span: s.ast.name.span,
                diag,
            })
        })
        .collect()
}

/// An in-scope solver with no result of its own under a variant that changes its inputs (S006).
struct UnsolvedVariant {
    key: ModuleKey,
    span: Span,
    diag: Diagnostic,
}

fn evaluate_stages(
    parsed: &[ParsedPackage],
    scope: &BTreeSet<String>,
    lint: &Lint,
    as_of: &str,
    analysis: &mut Analysis,
) {
    let sources: Vec<PackageSrc> = parsed
        .iter()
        .map(|p| PackageSrc {
            name: p.name.clone(),
            deps: p.deps.clone(),
            defaults: p.defaults.clone(),
            modules: p
                .modules
                .iter()
                .map(|m| ModuleSrc {
                    path: m.path.clone(),
                    file: &m.file,
                    enums: &m.enums,
                })
                .collect(),
        })
        .collect();
    let units = UnitTable::builtin();
    // frob:ticket 01M4YMKH7NQW964KS1C2GJX813
    let mut kinds = KindTable::builtin();
    kindcheck::register_user_kinds(
        &mut kinds,
        parsed
            .iter()
            .flat_map(|p| p.modules.iter().map(|m| &m.file)),
    );
    let (resolver, resolve_errors) = Resolver::new(sources.clone());
    // frob:ticket 01M4YRH3G6F8EGQ5X5CK4JKH27
    let mut evaluator = Evaluator::new(&resolver, &units, &sources)
        .with_solver_outputs(recorded_solver_outputs(parsed));
    let procs = Processes::compile(&mut evaluator, &sources);
    evaluator.set_processes(procs.clone());
    let fit_ranges = fitderive::eval_ranges(&mut evaluator, &sources);
    let cells = table_cells(&mut evaluator, parsed);
    let misses = table_misses(&mut evaluator, parsed, scope, &analysis.sources);
    let sites = solver_sites(parsed, scope, &mut evaluator, &units);
    let stale = stale_solvers(&sites);
    let evaluation = evaluator.evaluate();
    let (derived_fits, fit_findings) =
        fitderive::derive_fits_with_findings(&resolver, &evaluation, &fit_ranges, &sources);
    let outcome = procs.apply(&evaluation);
    let renames = Renames::collect(&resolver, &sources);
    let moves = Moves::new(&renames);
    let graph = DepGraph::build(&resolver, &renames, &sources);
    tracing::info!(
        packages = parsed.len(),
        values = evaluation.values.len(),
        checks = evaluation.checks.len(),
        "pipeline: evaluated"
    );

    let index = &analysis.sources;
    let in_scope: Vec<&ParsedPackage> = parsed
        .iter()
        .filter(|p| !p.is_std && scope.contains(&p.name))
        .collect();
    let located =
        |code: &str, sev: Severity, message: String, span: Span, module: Option<&ModuleKey>| {
            let Some(key) = module else {
                tracing::warn!(
                    code,
                    start = span.start,
                    "pipeline: diagnostic has no module; dropped"
                );
                return None;
            };
            let member_in = |f: &IndexedFile| {
                in_scope.iter().find(|p| p.name == f.package).and_then(|p| {
                    let m = p.modules.iter().find(|m| m.path == f.module)?;
                    member_path_at(&p.name, &m.path, &m.file, span)
                })
            };
            let file = index.file_for(key);
            let path = file.and_then(member_in);
            if file.is_none() {
                tracing::debug!(
                    code,
                    start = span.start,
                    "pipeline: diagnostic outside every project file (std); dropped"
                );
            }
            file.map(|f| {
                diag_at(
                    code,
                    sev,
                    message,
                    path,
                    Some(SourceIndex::location(f, span)),
                )
            })
        };

    let mut out: Vec<Diagnostic> = Vec::new();
    let lift_cfg = LintConfig::new();
    // frob:ticket 01M4TRKDAC6FR44KEW64H7PTZX
    out.extend(lifecycle::lift_missing_role(
        &procs
            .facts
            .iter()
            .filter(|f| scope.contains(&f.path.package))
            .map(|f| MissingRole {
                path: f.path.to_string(),
                process: Some(f.process.clone()),
                location: index
                    .module(&f.path.package, &f.path.modules)
                    .map(|file| SourceIndex::location(file, f.span)),
            })
            .collect::<Vec<_>>(),
        &lift_cfg,
    ));
    out.extend(lifecycle::lift_not_in_table(&misses, &lift_cfg));
    let variant_env = VariantEnv::new(&resolver, &units, &sources, &sites);
    out.extend(lifecycle::lift_lifecycle(
        &lifecycle_findings(parsed, scope, as_of, &analysis.sources, &variant_env),
        &lift_cfg,
    ));
    out.extend(f::lift(
        &fit_findings
            .iter()
            .filter(|x| scope.contains(&x.path.package))
            .map(|x| f::FFinding {
                code: x.code.to_owned(),
                message: x.message.clone(),
                path: Some(x.path.to_string()),
                location: index
                    .module(&x.path.package, &x.path.modules)
                    .map(|file| SourceIndex::location(file, x.span)),
            })
            .collect::<Vec<_>>(),
        &lift_cfg,
    ));
    // Kind diagnostics are exact: they come from one module at a time.
    let mut kind_keys: BTreeSet<(String, usize, usize)> = BTreeSet::new();
    // Kind-checker diagnostic spans per module; they shadow overlapping evaluator reports.
    // frob:ticket 01M4V1HDJS2MDQF8GRK4QX9B1C
    let mut kind_spans: Vec<(ModuleKey, usize, usize)> = Vec::new();
    // frob:ticket 01M4V2J34YBSMM32SK26K58FR1
    // Interface member kinds, so a wrong kind on an implementer's member is I001 or Q003.
    let iface_kinds: Vec<_> = parsed
        .iter()
        .flat_map(|q| lint_facts::interfaces(&q.name, &module_views(q)))
        .collect();
    for p in &in_scope {
        for m in &p.modules {
            // One checker per component, each over an env that types cross-component names
            // from the evaluated values (~CH5HR2K).
            let mut found = Vec::new();
            for c in m.file.items().filter_map(|i| match &i.kind {
                ItemKind::Component(c) => Some(c),
                _ => None,
            }) {
                let env = EvalEnv::new(&resolver, &evaluation, &p.name, &m.path, &c.name.name);
                let mut checker = KindChecker::new(&kinds, &units, &env);
                checker.check_component(c);
                let required: Vec<(String, String)> = c
                    .implements
                    .iter()
                    .filter_map(|r| r.segments.last())
                    .flat_map(|last| iface_kinds.iter().filter(move |i| i.path.item == last.name))
                    .flat_map(|i| {
                        i.members
                            .iter()
                            .map(|f| (f.name.clone(), f.type_name.clone()))
                    })
                    .collect();
                checker.check_interface_members(c, &required);
                found.extend(checker.into_diagnostics());
            }
            let file = index.module(&p.name, &m.path);
            for d in found {
                kind_keys.insert((d.code.to_owned(), d.span.start, d.span.end));
                kind_spans.push((ModuleKey::new(&p.name, &m.path), d.span.start, d.span.end));
                let path = member_path_at(&p.name, &m.path, &m.file, d.span);
                out.push(diag_at(
                    d.code,
                    sema_severity(d.severity),
                    d.message,
                    path,
                    file.map(|f| SourceIndex::location(f, d.span)),
                ));
            }
        }
    }
    // frob:ticket 01M4V13RVF8WWS2VMYE8WN9V3R
    // Eager E002 use targets, beside the lazy resolver errors.
    out.extend(resolve_errors.iter().filter_map(|e| {
        let base = located(
            e.code,
            Severity::Error,
            e.message.clone(),
            e.span,
            e.module.as_deref(),
        );
        lift_resolve_error(e, base, index)
    }));
    let eager_diags: Vec<Diagnostic> = resolver
        .check_use_targets()
        .iter()
        .filter_map(|e| {
            let base = located(
                e.code,
                Severity::Error,
                e.message.clone(),
                e.span,
                e.module.as_deref(),
            );
            lift_resolve_error(e, base, index)
        })
        .collect();
    // frob:ticket 01M4YR00X3TTZGSBAKQZDNTT6T
    // Privacy: E016 on `use` targets and E017 on public signatures (packages.md 3).
    {
        let checker = VisibilityChecker::new(&resolver, &sources);
        for e in checker
            .check_uses()
            .iter()
            .chain(checker.check_signatures().iter())
        {
            if e.module
                .as_ref()
                .is_some_and(|k| scope.contains(&k.package))
            {
                out.extend(located(
                    e.code,
                    Severity::Error,
                    e.message.clone(),
                    e.span,
                    e.module.as_deref(),
                ));
            }
        }
    }
    // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
    {
        let cites = CiteIndex::new(&sources);
        for p in parsed
            .iter()
            .filter(|p| !p.is_std && scope.contains(&p.name))
        {
            for (src, m) in sources
                .iter()
                .find(|s| s.name == p.name)
                .into_iter()
                .flat_map(|s| s.modules.iter())
                .map(|m| (&p.name, m))
            {
                let key = ModuleKey::new(src, &m.path);
                for d in cites.check_module(src, m) {
                    out.extend(located(
                        d.code,
                        Severity::Error,
                        d.message,
                        d.span,
                        Some(&key),
                    ));
                }
            }
        }
    }
    // frob:ticket 01M4V2K8RG8K1P774YVYBFVYDC
    // Solver declarations: S007, Q001 and E003 (solvers.md 1).
    for p in &in_scope {
        for m in &p.modules {
            let key = ModuleKey::new(&p.name, &m.path);
            for d in cadetta_sema::solverdecl::check_solver_decls(&m.file) {
                out.extend(located(
                    d.code,
                    Severity::Error,
                    d.message,
                    d.span,
                    Some(&key),
                ));
            }
        }
    }
    // frob:ticket 01M4V1HYS6PKK0F82XMJET4TC0
    for s in stale {
        let StaleSolver { key, span, diag } = s;
        // The message itself names what changed (solvers.md 4), not only the notes.
        let message = match diag
            .notes
            .iter()
            .map(|n| n.message.as_str())
            .collect::<Vec<_>>()
        {
            n if n.is_empty() => diag.message,
            n => format!("{} ({})", diag.message, n.join("; ")),
        };
        if let Some(mut d) = located("S003", Severity::Warning, message, span, Some(&key)) {
            d.notes = diag.notes;
            d.path = diag.path.or(d.path);
            out.push(d);
        }
    }
    for d in &evaluation.diagnostics {
        // C001/C002 are reported once, from the structured check results below.
        // Datum-bound findings (~S5DC5W0) have no structured check result and pass through.
        if (d.code == "C001" || d.code == "C002")
            && evaluation.checks.iter().any(|r| r.span == d.span)
        {
            continue;
        }
        if kind_keys.contains(&(d.code.to_owned(), d.span.start, d.span.end)) {
            continue;
        }
        if shadowed_by_kind_diag(d, &kind_spans) {
            tracing::debug!(
                code = d.code,
                "dropping evaluator report shadowed by a kind diagnostic"
            );
            continue;
        }
        out.extend(located(
            d.code,
            sema_severity(d.severity),
            d.message.clone(),
            d.span,
            d.module.as_ref(),
        ));
    }
    for d in graph.diagnostics() {
        out.extend(located(
            d.code,
            sema_severity(d.severity),
            d.message.clone(),
            d.span,
            d.module.as_ref(),
        ));
    }
    // frob:ticket 01M51ND0V1FXBW8EAHVEQX7ZDY
    for d in &outcome.diagnostics {
        tracing::debug!(code = d.code, "pipeline: process rule problem surfaced");
        out.extend(located(
            d.code,
            sema_severity(d.severity),
            d.message.clone(),
            d.span,
            d.module.as_ref(),
        ));
    }
    for r in &evaluation.checks {
        if r.obligation && scope.contains(&r.path.package) {
            analysis
                .suppressions
                .require_paths
                .insert(r.path.to_string());
        }
        let (code, sev) = match r.verdict {
            Verdict::Holds => continue,
            Verdict::Violates => ("C001", Severity::Error),
            Verdict::Undecided => ("C002", Severity::Warning),
        };
        if !scope.contains(&r.path.package) {
            continue;
        }
        let file = index.module(&r.path.package, &r.path.modules);
        let what = if r.obligation { "requirement" } else { "check" };
        let message = r.message.clone().unwrap_or_else(|| {
            let verdict = if sev == Severity::Error {
                "violated"
            } else {
                "undecided"
            };
            format!("{what} `{}` is {verdict}", r.path)
        });
        out.push(diag_at(
            code,
            sev,
            message,
            Some(r.path.to_string()),
            file.map(|f| SourceIndex::location(f, r.span)),
        ));
    }

    // frob:ticket 01M4Z3FGQHHAPQ3N8VCDEY1G2X
    let alternates = alternate_evaluations(parsed, scope, &variant_env);
    // frob:ticket 01M50M66PAWQ2N9P3HRB645GTM
    for u in variant_env.take_unsolved() {
        let UnsolvedVariant { key, span, diag } = u;
        if let Some(mut d) = located("S006", Severity::Warning, diag.message, span, Some(&key)) {
            d.alt = diag.alt;
            d.path = diag.path.or(d.path);
            out.push(d);
        }
    }
    let variant_refs: BTreeMap<String, &cadetta_sema::eval::Evaluation> =
        alternates.iter().map(|(k, e)| (k.clone(), &**e)).collect();
    // Lower each in-scope package and run the IR lints over it.
    let lower_one = |p: &ParsedPackage| {
        let texts: BTreeMap<Vec<String>, SourceText> = p
            .modules
            .iter()
            .map(|m| {
                (
                    m.path.clone(),
                    SourceText {
                        file: &m.display,
                        text: &m.text,
                    },
                )
            })
            .collect();
        lower(&LowerInput {
            name: &p.name,
            version: &p.version,
            packages: &sources,
            sources: &texts,
            package_dir: Some(&p.dir),
            evaluation: &evaluation,
            compensation: Some(&outcome),
            deps: &graph,
            variants: variant_refs.clone(),
            fits: &derived_fits,
            solver_states: solver_states(&sites),
        })
    };
    // frob:ticket 01M4YTGQ9BHMKS38GTGX2PFH03
    // Out-of-scope packages are lowered unlinted so an in-scope IR can digest its cross-package reads.
    analysis.dependencies = parsed
        .iter()
        .filter(|p| !p.is_std && !scope.contains(&p.name))
        .map(|p| LoweredPackage {
            name: p.name.clone(),
            ir: lower_one(p),
            text_params: Default::default(),
        })
        .collect();
    let mut lowered = Vec::new();
    for p in &in_scope {
        let ir = lower_one(p);
        let locate = |s: &cadetta_ir::model::SpanRef| index.locate_ref(&s.file, s.start, s.end);
        out.extend(unused::check(&ir, locate));
        out.extend(interface::check_alternates(&ir));
        // ~A8J3XJW: I001, I004 and U002 read facts the IR lacks (interfaces, visibility, use).
        let own_views = module_views(p);
        let all_interfaces: Vec<_> = parsed
            .iter()
            .flat_map(|q| lint_facts::interfaces(&q.name, &module_views(q)))
            .collect();
        out.extend(interface::check_members(
            &ir.components,
            &all_interfaces,
            locate,
        ));
        out.extend(c::check_interface_bounds(
            &ir.components,
            &all_interfaces,
            locate,
        ));
        // ~G2DFAFQ: U002 and I004 read one fact table built from name resolution.
        let facts = lint_facts::item_facts(&p.name, &own_views, &ir, &resolver);
        out.extend(u002::check(&facts, locate));
        out.extend(u002::check_i004(&facts, locate));
        analysis.suppressions.allows.extend(lint_facts::allows(&ir));
        analysis
            .suppressions
            .allows
            .extend(lint_facts::statement_allows(&p.name, &own_views));
        // IR-only rule families run with the catalog severities; `check` applies `[lint]`
        // overrides afterwards through the engine.
        let lcfg = LintConfig::new();
        // `[lint]` settings are validated when the configuration loads, so these cannot fail.
        let dcfg = lint
            .d_config(
                std::iter::once(&p.name)
                    .chain(&p.deps)
                    .flat_map(|n| cells.get(n).into_iter().flatten().cloned())
                    .collect(),
            )
            .expect("[lint] settings were validated at load");
        // Package-declared kinds are known only now, so `precision` resolves here.
        let mcfg = lint.m_config_lenient(&kinds);
        // A key naming no kind is a typo the lenient resolution would hide: report each one.
        for kind in lint.unknown_precision_kinds(&kinds) {
            tracing::warn!(package = %p.name, %kind, "pipeline: unknown [lint] precision kind");
            out.push(unknown_precision_diag(&p.name, &kind));
        }
        // C001-C003 come from the structured check results; C004 reads the lowered IR.
        out.extend(c::check_derived(&ir, locate));
        out.extend(c::check_fit_ranges(&ir, locate));
        out.extend(d::check_d001(&ir, &lcfg, &dcfg, locate));
        out.extend(d::check_d002(&ir, &lcfg, &dcfg, locate));
        out.extend(f::check_f004(&ir, &lcfg, locate));
        out.extend(m::check_m001(&ir, &lcfg, &mcfg, locate));
        out.extend(m::check_m002(&ir, &lcfg, locate));
        out.extend(m::check_m003(&ir, &lcfg, locate));
        out.extend(n::check(&ir, &lcfg, locate));
        out.extend(q::check_inexact(&ir, &lcfg, locate));
        let (ots_values, cites, own_cites) = s_facts(&ir, &p.name, Some(&p.dir));
        out.extend(s::check_s001(&ots_values, locate));
        out.extend(s::check_s002(&cites, locate));
        out.extend(s::check_s008(&own_cites, &ots_values, locate));
        for m in &p.modules {
            module_waivers(&m.file, &moves, &mut analysis.suppressions.waivers);
        }
        analysis.eols.extend(eol_dates(&p.name, &p.modules));
        let text_params = evaluation
            .values
            .iter()
            .filter(|(k, e)| k.package == p.name && !e.analysis)
            .filter_map(|(k, e)| match &e.value {
                cadetta_sema::eval::Val::Text(t) => Some((k.to_string(), t.clone())),
                _ => None,
            })
            .collect();
        lowered.push(LoweredPackage {
            name: p.name.clone(),
            ir,
            text_params,
        });
    }
    tracing::info!(
        diagnostics = out.len(),
        packages = lowered.len(),
        "pipeline: analysis complete"
    );
    // The eager pass repeats what a use site already reported: one fault, one diagnostic.
    for d in eager_diags {
        let twin = |o: &Diagnostic| {
            o.code == d.code
                && o.message == d.message
                && o.path == d.path
                && o.location == d.location
        };
        if !out.iter().any(twin) {
            out.push(d);
        }
    }
    analysis.diagnostics.extend(out);
    analysis.packages = lowered;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A same-sized file of `pkg::module`.
    fn indexed(name: &str, pkg: &str, module: &str, is_std: bool) -> IndexedFile {
        IndexedFile {
            name: name.to_owned(),
            package: pkg.to_owned(),
            module: vec![module.to_owned()],
            is_std,
            source: SourceFile::new(name, "model A { x = 1 }\n"),
        }
    }

    fn index() -> SourceIndex {
        SourceIndex {
            files: vec![
                indexed("src/alpha.cdt", "p", "alpha", false),
                indexed("src/frame.cdt", "p", "frame", false),
                indexed("std/src/s.cdt", "std", "s", true),
            ],
        }
    }

    // frob:tests file_for_picks_the_diagnostics_own_module
    #[test]
    fn file_for_picks_the_diagnostics_own_module() {
        let ix = index();
        // The module key alone decides the file.
        let frame = ModuleKey::new("p", &["frame".to_owned()]);
        assert_eq!(
            ix.file_for(&frame).map(|f| f.name.as_str()),
            Some("src/frame.cdt")
        );
        let alpha = ModuleKey::new("p", &["alpha".to_owned()]);
        assert_eq!(
            ix.file_for(&alpha).map(|f| f.name.as_str()),
            Some("src/alpha.cdt")
        );
    }

    // frob:tests crates/cadetta-cli/src/pipeline.rs::lift_resolve_error
    #[test]
    fn lift_resolve_error_keeps_resolver_text_and_adds_related_notes() {
        let ix = index();
        let key = ModuleKey::new("p", &["alpha".to_owned()]);
        let err = ResolveError {
            code: "E003",
            message: "`Part` is defined twice".to_owned(),
            span: Span { start: 0, end: 5 },
            related: vec![Span { start: 6, end: 9 }],
            module: Some(Box::new(key)),
        };
        let base = Some(diag_at("E003", Severity::Error, "x", None, None));
        let d = lift_resolve_error(&err, base, &ix).expect("lifted");
        assert_eq!(d.message, "`Part` is defined twice");
        assert_eq!(d.code, "E003");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.path, None, "a path-less base stays path-less");
        assert!(lift_resolve_error(&err, None, &ix).is_none());
    }

    // frob:tests file_for_drops_std_and_unknown_modules
    #[test]
    fn file_for_drops_std_and_unknown_modules() {
        let ix = index();
        let std = ModuleKey::new("std", &["s".to_owned()]);
        assert!(ix.file_for(&std).is_none());
        let none = ModuleKey::new("p", &["missing".to_owned()]);
        assert!(ix.file_for(&none).is_none());
    }

    /// A one-package project whose solver has a recorded result with the given digest.
    fn solver_project(tag: &str, with_result: bool) -> Project {
        solver_project_with(tag, with_result, "", "")
    }

    /// `solver_project` with extra lines inside the solver block and inside the model.
    fn solver_project_with(
        tag: &str,
        with_result: bool,
        in_solver: &str,
        in_model: &str,
    ) -> Project {
        let dir = std::env::temp_dir().join(format!("cadetta-stale-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("cadetta.toml"),
            "[package]\nname = \"acme.t\"\nversion = \"0.1.0\"\nas_of = \"2026-10-13\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/lib.cdt"),
            format!("model Box {{\n  m: Mass = 2kg\n  solver size {{\n    script \"size.sh\"\n    in m = m\n    out w: Length\n{in_solver}  }}\n{in_model}}}\n"),
        )
        .unwrap();
        std::fs::write(dir.join("size.sh"), "true\n").unwrap();
        if with_result {
            let zero = format!("blake3:{}", "0".repeat(64));
            std::fs::write(
                dir.join("size.out.json"),
                format!(
                    "{{\"schema\":\"cadetta.solve-result/1\",\"results\":[{{\"solver\":\"acme.t::Box.size\",\"input_digest\":\"{zero}\",\"script_digest\":\"{zero}\",\"dep_digests\":{{}},\"inputs\":{{}},\"outputs\":{{\"w\":\"2m\"}},\"diagnostics\":[]}}]}}"
                ),
            )
            .unwrap();
        }
        Project::load(&dir.join("cadetta.toml")).unwrap()
    }

    // frob:ticket 01M5137DWGRARW5K0EMRB93QEK
    // frob:tests crates/cadetta-cli/src/pipeline.rs::unknown_precision_diag
    #[test]
    fn unknown_lint_precision_kind_is_reported() {
        let project = solver_project("precision", false);
        let toml = project.root.join("cadetta.toml");
        let mut text = std::fs::read_to_string(&toml).unwrap();
        text.push_str("[lint]\nprecision = { Lenth = 2, Length = 3 }\n");
        std::fs::write(&toml, text).unwrap();
        let project = Project::load(&toml).unwrap();
        let analysis = analyze(&project, &[0]);
        let hits: Vec<&Diagnostic> = analysis
            .diagnostics
            .iter()
            .filter(|d| d.message.contains("unknown quantity kind"))
            .collect();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].message.contains("`Lenth`"), "{}", hits[0].message);
    }

    // frob:ticket 01M4V1HYS6PKK0F82XMJET4TC0
    // frob:tests crates/cadetta-cli/src/pipeline.rs::stale_solvers
    #[test]
    fn stale_recorded_result_reports_s003_naming_what_changed() {
        let project = solver_project("stale", true);
        let analysis = analyze(&project, &[0]);
        let d = analysis
            .diagnostics
            .iter()
            .find(|d| d.code == "S003")
            .expect("S003 for a result with other digests");
        assert!(d.location.is_some());
        assert!(d.notes.iter().any(|n| n.message == "changed: inputs"));
        assert!(d.notes.iter().any(|n| n.message == "changed: script"));
        // The message itself names what changed, not only the notes.
        assert!(d.message.contains("changed: inputs"), "{}", d.message);
        assert!(d.message.contains("changed: script"), "{}", d.message);
    }

    // frob:ticket 01M50RKKNFVN2SJJGRAMGXDNYP
    // frob:tests crates/cadetta-sema/src/checks.rs::add_solver_checks
    #[test]
    fn solver_own_check_on_a_recorded_output_is_evaluated() {
        // Recorded `w` is 2m, so `w < 1m` fails: C001 from the solver's own check.
        let project = solver_project_with("own", true, "    check small: w < 1m\n", "");
        let analysis = analyze(&project, &[0]);
        assert!(
            analysis.diagnostics.iter().any(|d| d.code == "C001"),
            "{:?}",
            analysis
                .diagnostics
                .iter()
                .map(|d| &d.code)
                .collect::<Vec<_>>()
        );
    }

    // frob:ticket 01M50RKKNFVN2SJJGRAMGXDNYP
    // frob:tests crates/cadetta-sema/src/checks.rs::run_checks
    #[test]
    fn check_reading_an_unsolved_output_is_c002_not_s003() {
        let project = solver_project_with("unsolved", false, "", "  check ok: size.w < 1m\n");
        let analysis = analyze(&project, &[0]);
        let codes: Vec<&str> = analysis
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect();
        assert!(codes.contains(&"C002"), "{codes:?}");
        assert!(!codes.contains(&"S003"), "{codes:?}");
    }

    // frob:ticket 01M4V1HYS6PKK0F82XMJET4TC0
    // frob:tests crates/cadetta-cli/src/pipeline.rs::stale_solvers
    #[test]
    fn solver_without_a_recorded_result_is_unsolved_not_stale() {
        let project = solver_project("missing", false);
        let analysis = analyze(&project, &[0]);
        assert!(analysis.diagnostics.iter().all(|d| d.code != "S003"));
    }

    /// A package whose wheel slot has `alt` as its alternate and whose solver reads the wheel.
    fn variant_project(tag: &str, alt: &str, with_variant_result: bool) -> Project {
        let dir = std::env::temp_dir().join(format!("cadetta-s006-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("cadetta.toml"),
            "[package]\nname = \"acme.t\"\nversion = \"0.1.0\"\nas_of = \"2026-10-13\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/lib.cdt"),
            format!(
                "cite src: web {{ url = \"https://x.example/a.pdf\" }}\n\
                 pub interface Caster {{\n  load_rating: Force\n}}\n\
                 pub ots Good: Caster @src {{\n  load_rating = 500N\n}}\n\
                 pub ots Heavy: Caster @src {{\n  load_rating = 450N\n}}\n\
                 pub ots Same: Caster @src {{\n  load_rating = 500N\n}}\n\
                 pub assembly Rover {{\n  part wheel: Caster = Good alt [{alt}]\n  solver sizing {{\n    script \"size.sh\"\n    in load = wheel.load_rating\n    out w: Length\n  }}\n  check ok: 1mm >= sizing.w \"wall\"\n}}\n"
            ),
        )
        .unwrap();
        std::fs::write(dir.join("size.sh"), "true\n").unwrap();
        let zero = format!("blake3:{}", "0".repeat(64));
        let one = format!("blake3:{}", "1".repeat(64));
        let variant = if with_variant_result {
            format!(
                ",{{\"solver\":\"acme.t::Rover.sizing\",\"variant\":\"acme.t::Rover/wheel=acme.t::{alt}\",\"input_digest\":\"{one}\",\"script_digest\":\"{zero}\",\"dep_digests\":{{}},\"inputs\":{{}},\"outputs\":{{\"w\":\"2m\"}},\"diagnostics\":[]}}"
            )
        } else {
            String::new()
        };
        std::fs::write(
            dir.join("size.out.json"),
            format!(
                "{{\"schema\":\"cadetta.solve-result/1\",\"results\":[{{\"solver\":\"acme.t::Rover.sizing\",\"input_digest\":\"{zero}\",\"script_digest\":\"{zero}\",\"dep_digests\":{{}},\"inputs\":{{}},\"outputs\":{{\"w\":\"1mm\"}},\"diagnostics\":[]}}{variant}]}}"
            ),
        )
        .unwrap();
        Project::load(&dir.join("cadetta.toml")).unwrap()
    }

    // frob:ticket 01M50M66PAWQ2N9P3HRB645GTM
    // frob:tests crates/cadetta-cli/src/pipeline.rs::variant_outputs
    #[test]
    fn variant_with_changed_inputs_and_no_result_is_s006() {
        let analysis = analyze(&variant_project("unsolved", "Heavy", false), &[0]);
        let d = analysis
            .diagnostics
            .iter()
            .find(|d| d.code == "S006")
            .expect("S006 for an unsolved variant");
        assert!(d.location.is_some());
        assert!(d.message.contains("Heavy"), "{}", d.message);
    }

    // frob:ticket 01M50M66PAWQ2N9P3HRB645GTM
    // frob:tests crates/cadetta-cli/src/pipeline.rs::variant_outputs
    #[test]
    fn variant_with_unchanged_inputs_reads_the_primary_without_s006() {
        let analysis = analyze(&variant_project("same", "Same", false), &[0]);
        assert!(analysis.diagnostics.iter().all(|d| d.code != "S006"));
    }

    // frob:ticket 01M50M66PAWQ2N9P3HRB645GTM
    // frob:tests crates/cadetta-cli/src/pipeline.rs::variant_outputs
    #[test]
    fn variant_with_its_own_recorded_result_is_not_s006() {
        let analysis = analyze(&variant_project("solved", "Heavy", true), &[0]);
        assert!(analysis.diagnostics.iter().all(|d| d.code != "S006"));
    }

    /// A one-package project holding a process whose `hole` rule is `rule`, applied to a member.
    fn process_rule_project(tag: &str, rule: &str) -> Project {
        let dir = std::env::temp_dir().join(format!("cadetta-prule-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("cadetta.toml"),
            "[package]\nname = \"acme.t\"\nversion = \"0.1.0\"\nas_of = \"2026-10-13\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/lib.cdt"),
            format!(
                "process Fdm {{\n  rule hole(d: Length) -> Length = {rule}\n}}\n\
                 model Case {{\n  #![process(Fdm)]\n  insert_hole = 3.4mm #[hole]\n  \
                 check drawn_ok: insert_hole.drawn > 3mm\n}}\n"
            ),
        )
        .unwrap();
        Project::load(&dir.join("cadetta.toml")).unwrap()
    }

    // frob:ticket 01M51ND0V1FXBW8EAHVEQX7ZDY
    // frob:tests crates/cadetta-cli/src/pipeline.rs::analyze
    #[test]
    fn process_rule_changing_the_kind_is_an_eval_error() {
        let analysis = analyze(&process_rule_project("kind", "d * 2mm"), &[0]);
        let d = analysis
            .diagnostics
            .iter()
            .find(|d| d.code == "EVAL")
            .expect("EVAL for a rule that changes the argument kind");
        assert!(d.message.contains("changes the kind"), "{}", d.message);
    }

    // frob:ticket 01M51ND0V1FXBW8EAHVEQX7ZDY
    // frob:tests crates/cadetta-cli/src/pipeline.rs::analyze
    #[test]
    fn process_rule_keeping_the_kind_has_no_eval_error() {
        let analysis = analyze(&process_rule_project("keep", "d + 0.2mm"), &[0]);
        assert!(
            analysis.diagnostics.iter().all(|d| d.code != "EVAL"),
            "{:?}",
            analysis.diagnostics
        );
    }
}
