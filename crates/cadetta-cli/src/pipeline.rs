//! frob:ticket 01M4SPWT3FP4SN4RTCHE631Y52
//! The source-to-diagnostics pipeline shared by the CLI verbs: parse, resolve, evaluate,
//! kind-check, lower to IR and run the IR lints (config-cli.md 3, ir.md 1).
//!
//! [`analyze`] is the one entry point; `check` renders its diagnostics, and later verbs
//! (`why`, `impact`, `ir`, `ack`, `obligations`) reuse the lowered packages it returns.
//! Resolver, evaluator and dependency diagnostics carry the module their span lives in
//! (`ModuleKey`), so [`SourceIndex::file_for`] maps them to the exact file; only a
//! diagnostic without a module falls back to the span heuristic ([`SourceIndex::attribute`]).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use cadetta_ir::entities::Waiver;
use cadetta_ir::model::{
    Citation, CiteBuiltin, ComponentClass, MemberKind as IrMemberKind, Project as IrProject,
};
use cadetta_ir::variant_name::VariantName;
use cadetta_lint::diag::{Diagnostic, Location, Position, Severity};
use cadetta_lint::engine::{LintConfig, Suppressions};
use cadetta_lint::rules::s::{self, CitationFacts, CitationKind, OtsValueFacts};
use cadetta_lint::rules::d::{Key, NamedValue};
use cadetta_lint::rules::lifecycle::{self, LifecycleFinding, MissingRole, TableMiss};
use cadetta_lint::rules::{c, d, f, interface, m, n, q, u002, unused};
use cadetta_sema::cite::CiteIndex;
use cadetta_sema::deps::{DepGraph, Renames, Severity as SemaSeverity};
use cadetta_sema::eval::{Evaluator, Selection};
use cadetta_sema::fitderive;
use cadetta_sema::kindcheck::{self, EvalEnv, KindChecker};
use cadetta_sema::lower::{LowerInput, SourceText, lower};
use cadetta_sema::modules::module_path_of;
use cadetta_sema::process::Processes;
use cadetta_sema::visibility::VisibilityChecker;
use cadetta_sema::resolve::{
    CanonPath, Ctx, EnumDecl as SemaEnum, MemberSeg, ModuleKey, ModuleSrc, PackageSrc, Resolver,
};
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

    /// The file a diagnostic belongs to: the file of its `module` when it has one
    /// (None for std, whose findings are not the project's), else the span heuristic.
    // frob:ticket 01M4TA3BTW256J8FKMS6KASY82
    fn file_for<'a>(
        &'a self,
        module: Option<&ModuleKey>,
        span: Span,
        covering: &dyn Fn(&IndexedFile) -> bool,
    ) -> Option<&'a IndexedFile> {
        let Some(key) = module else {
            return self.attribute(span, covering);
        };
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

    /// Pick the project file a module-less span belongs to.
    ///
    /// Only used for diagnostics that carry no `ModuleKey`. The rule: among non-std files
    /// whose text covers the span, the one whose AST has a member covering it
    /// (`members`), else the first covering file.
    fn attribute<'a>(
        &'a self,
        span: Span,
        covering: &dyn Fn(&IndexedFile) -> bool,
    ) -> Option<&'a IndexedFile> {
        let fits: Vec<&IndexedFile> = self
            .files
            .iter()
            .filter(|f| !f.is_std && f.source.text().len() >= span.end)
            .collect();
        if fits.len() > 1 {
            tracing::trace!(
                candidates = fits.len(),
                start = span.start,
                "pipeline: ambiguous span attribution"
            );
        }
        fits.iter()
            .copied()
            .find(|f| covering(f))
            .or_else(|| fits.first().copied())
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
fn module_waivers(file: &File, out: &mut Vec<Waiver>) {
    for item in file.items() {
        let ItemKind::Waive(w) = &item.kind else {
            continue;
        };
        let for_alt = match &w.for_alt {
            None => None,
            Some(a) => match VariantName::parse(&format!("{}={}", a.slot.text, a.alt.text)) {
                Ok(v) => Some(v),
                Err(error) => {
                    tracing::warn!(code = %w.code.name, %error, "pipeline: waiver with a bad `for alt`; skipped");
                    continue;
                }
            },
        };
        tracing::debug!(code = %w.code.name, glob = %w.glob.text, "pipeline: waiver collected");
        out.push(Waiver {
            code: w.code.name.clone(),
            glob: w.glob.text.clone(),
            because: w.because.clone(),
            expires: w.expires.as_ref().map(|e| e.text.clone()),
            for_alt,
            targets_closure: BTreeMap::new(),
        });
    }
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
        dir: dir.to_owned(),
        modules,
    })
}

struct ParsedPackage {
    name: String,
    version: String,
    deps: Vec<String>,
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
    kind_spans.iter().any(|(k, s, e)| {
        d.module.as_ref() == Some(k) && d.span.start < *e && *s < d.span.end
    })
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
    evaluate_stages(&parsed, &scope_names, &project.config.lint, as_of, &mut analysis);
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
                    let key_text = m.text.get(row.key.span.start..row.key.span.end).unwrap_or("?");
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
    cache: std::cell::RefCell<BTreeMap<String, bool>>,
}

impl<'a, 'p> VariantEnv<'a, 'p> {
    /// Builds the environment with an empty cache.
    fn new(resolver: &'a Resolver<'p>, units: &'a UnitTable, sources: &'a [PackageSrc<'a>]) -> Self {
        Self { resolver, units, sources, cache: std::cell::RefCell::new(BTreeMap::new()) }
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
        let Some((alt_path, _)) = self.resolver.ctor_component(package, module, alt) else {
            tracing::debug!(slot, "pipeline: alternate does not resolve; not evaluated");
            return true;
        };
        let selection = Selection { owner: owner.clone(), slot: slot.to_owned(), alt: alt_path };
        let key = selection.key();
        if let Some(held) = self.cache.borrow().get(&key) {
            return *held;
        }
        let evaluator =
            Evaluator::new(self.resolver, self.units, self.sources).with_selection(selection);
        let evaluation = evaluator.evaluate();
        let violated: Vec<String> = evaluation
            .checks
            .iter()
            .filter(|r| r.verdict == Verdict::Violates)
            .map(|r| r.path.to_string())
            .collect();
        let held = violated.is_empty();
        tracing::info!(variant = %key, viable = held, ?violated, "pipeline: alternate viability evaluated");
        self.cache.borrow_mut().insert(key, held);
        held
    }
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
                    ots.entry(c.name.name.as_str()).or_default().push((p.name.as_str(), lc));
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
    for p in parsed.iter().filter(|p| !p.is_std && scope.contains(&p.name)) {
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
                    let needs_variants = lc.as_ref().is_some_and(|l| l.status != vl::Status::Active);
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
                            let lifecycle = found.and_then(|(_, l)| l.clone());
                            let mut viable = found.is_some()
                                && !lifecycle.as_ref().is_some_and(|l| l.status.is_end_of_life());
                            if viable && needs_variants {
                                viable = env.alternate_holds(&owner, &part.name.name, &p.name, &m.path, a);
                            }
                            vl::Alternate { name, lifecycle, viable }
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
                tables.entry(cite).or_default().push((t.name.name.clone(), keys));
            }
        }
    }
    let mut out = Vec::new();
    for p in parsed.iter().filter(|p| !p.is_std && scope.contains(&p.name)) {
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
        parsed.iter().flat_map(|p| p.modules.iter().map(|m| &m.file)),
    );
    let (resolver, resolve_errors) = Resolver::new(sources.clone());
    let mut evaluator = Evaluator::new(&resolver, &units, &sources);
    let procs = Processes::compile(&mut evaluator, &sources);
    let fit_ranges = fitderive::eval_ranges(&mut evaluator, &sources);
    let cells = table_cells(&mut evaluator, parsed);
    let misses = table_misses(&mut evaluator, parsed, scope, &analysis.sources);
    let evaluation = evaluator.evaluate();
    let (derived_fits, fit_findings) =
        fitderive::derive_fits_with_findings(&resolver, &evaluation, &fit_ranges, &sources);
    let outcome = procs.apply(&evaluation);
    let renames = Renames::collect(&resolver, &sources);
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
            let member_in = |f: &IndexedFile| {
                in_scope.iter().find(|p| p.name == f.package).and_then(|p| {
                    let m = p.modules.iter().find(|m| m.path == f.module)?;
                    member_path_at(&p.name, &m.path, &m.file, span)
                })
            };
            let file = index.file_for(module, span, &|f| member_in(f).is_some());
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
    out.extend(lifecycle::lift_lifecycle(
        &lifecycle_findings(parsed, scope, as_of, &analysis.sources),
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
                    .flat_map(|i| i.members.iter().map(|f| (f.name.clone(), f.type_name.clone())))
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
    out.extend(resolve_errors.iter().flat_map(|e| {
        located(
            e.code,
            Severity::Error,
            e.message.clone(),
            e.span,
            e.module.as_deref(),
        )
    }));
    let eager_diags: Vec<Diagnostic> = resolver
        .check_use_targets()
        .iter()
        .flat_map(|e| {
            located(
                e.code,
                Severity::Error,
                e.message.clone(),
                e.span,
                e.module.as_deref(),
            )
        })
        .collect();
    // frob:ticket 01M4YR00X3TTZGSBAKQZDNTT6T
    // Privacy: E016 on `use` targets and E017 on public signatures (packages.md 3).
    {
        let checker = VisibilityChecker::new(&resolver, &sources);
        for e in checker.check_uses().iter().chain(checker.check_signatures().iter()) {
            if e.module.as_ref().is_some_and(|k| scope.contains(&k.package)) {
                out.extend(located(e.code, Severity::Error, e.message.clone(), e.span, e.module.as_deref()));
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
            tracing::debug!(code = d.code, "dropping evaluator report shadowed by a kind diagnostic");
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
            variants: BTreeMap::new(),
            fits: &derived_fits,
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
        out.extend(interface::check_members(&ir.components, &all_interfaces, locate));
        out.extend(c::check_interface_bounds(&ir.components, &all_interfaces, locate));
        out.extend(interface::check_roots(
            &ir,
            |c| lint_facts::is_private(c, &own_views),
            locate,
        ));
        out.extend(u002::check(
            &lint_facts::item_facts(&p.name, &own_views, &ir),
            locate,
        ));
        analysis
            .suppressions
            .allows
            .extend(lint_facts::allows(&ir));
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
        let mcfg = lint.m_config().expect("[lint] settings were validated at load");
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
            module_waivers(&m.file, &mut analysis.suppressions.waivers);
        }
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
            o.code == d.code && o.message == d.message && o.path == d.path && o.location == d.location
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
        let span = Span::new(0, 5);
        let all = |_: &IndexedFile| true;
        // The span fits every file; the module key, not the span, decides.
        let frame = ModuleKey::new("p", &["frame".to_owned()]);
        assert_eq!(
            ix.file_for(Some(&frame), span, &all)
                .map(|f| f.name.as_str()),
            Some("src/frame.cdt")
        );
        let alpha = ModuleKey::new("p", &["alpha".to_owned()]);
        assert_eq!(
            ix.file_for(Some(&alpha), span, &all)
                .map(|f| f.name.as_str()),
            Some("src/alpha.cdt")
        );
    }

    // frob:tests file_for_drops_std_and_unknown_modules
    #[test]
    fn file_for_drops_std_and_unknown_modules() {
        let ix = index();
        let span = Span::new(0, 5);
        let all = |_: &IndexedFile| true;
        let std = ModuleKey::new("std", &["s".to_owned()]);
        assert!(ix.file_for(Some(&std), span, &all).is_none());
        let none = ModuleKey::new("p", &["missing".to_owned()]);
        assert!(ix.file_for(Some(&none), span, &all).is_none());
    }

    // frob:tests file_for_without_a_module_uses_the_heuristic
    #[test]
    fn file_for_without_a_module_uses_the_heuristic() {
        let ix = index();
        let got = ix.file_for(None, Span::new(0, 5), &|_: &IndexedFile| true);
        assert!(got.is_some_and(|f| !f.is_std));
    }
}
