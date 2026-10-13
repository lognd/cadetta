//! frob:ticket 01M4SPWT3FP4SN4RTCHE631Y52
//! The source-to-diagnostics pipeline shared by the CLI verbs: parse, resolve, evaluate,
//! kind-check, lower to IR and run the IR lints (config-cli.md 3, ir.md 1).
//!
//! [`analyze`] is the one entry point; `check` renders its diagnostics, and later verbs
//! (`why`, `impact`, `ir`, `ack`, `obligations`) reuse the lowered packages it returns.
//! Spans in the resolver, evaluator and dependency diagnostics are bare byte offsets with
//! no module identity, so those diagnostics are attributed to a module heuristically
//! (see [`SourceIndex::attribute`]); kind and check diagnostics are exact.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use cadetta_ir::entities::Waiver;
use cadetta_ir::model::{ComponentClass, MemberKind as IrMemberKind, Project as IrProject};
use cadetta_ir::variant_name::VariantName;
use cadetta_lint::diag::{Diagnostic, Location, Position, Severity};
use cadetta_lint::engine::{LintConfig, Suppressions};
use cadetta_lint::rules::s::{self, CitationFacts, CitationKind, OtsValueFacts};
use cadetta_lint::rules::{d, f, interface, m, n, q, unused};
use cadetta_sema::cite::CiteIndex;
use cadetta_sema::deps::{DepGraph, Renames, Severity as SemaSeverity};
use cadetta_sema::eval::Evaluator;
use cadetta_sema::kindcheck::{EmptyEnv, KindChecker};
use cadetta_sema::lower::{LowerInput, SourceText, lower};
use cadetta_sema::modules::module_path_of;
use cadetta_sema::process::Processes;
use cadetta_sema::resolve::{
    CanonPath, EnumDecl as SemaEnum, MemberSeg, ModuleSrc, PackageSrc, Resolver,
};
use cadetta_syntax::ast::{File, ItemKind, MemberKind};
use cadetta_syntax::lexer::Span;
use cadetta_syntax::parser::parse;
use cadetta_syntax::span::SourceFile;
use cadetta_units::{KindTable, UnitTable, Verdict};

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

    /// Pick the project file a module-less span belongs to.
    ///
    /// Resolver, evaluator and dependency diagnostics carry no module identity (gap
    /// ticket filed). The rule: among non-std files whose text covers the span, the one
    /// whose AST has a member covering it (`members`), else the first covering file.
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
}

/// Everything one analysis produced.
#[derive(Debug, Default)]
pub struct Analysis {
    /// Raw diagnostics (severity as the producing stage assigned it; run them through
    /// `cadetta_lint::engine::apply` for the effective severity).
    pub diagnostics: Vec<Diagnostic>,
    /// The in-scope packages lowered to IR; empty when parsing failed.
    pub packages: Vec<LoweredPackage>,
    /// Source files, for rendering excerpts.
    pub sources: SourceIndex,
    /// True when a syntax error stopped the pipeline before evaluation.
    pub halted: bool,
    /// The `waive` declarations of the in-scope packages, for the lint engine.
    pub suppressions: Suppressions,
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

/// S001/S002 inputs of one lowered package: `ots` values and citations (the filesystem answer
/// is taken against the package directory).
fn s_facts(ir: &IrProject, dir: Option<&Path>) -> (Vec<OtsValueFacts>, Vec<CitationFacts>) {
    let (mut values, mut cites) = (Vec::new(), Vec::new());
    // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
    let cite_of = |path: String, r: &cadetta_ir::model::CiteRef, span| {
        let item = r
            .cite
            .as_ref()
            .and_then(|p| ir.citations.iter().find(|c| &c.path == p));
        // A `file` field is a local file (S002 checks it exists); a url-only cite is a Url;
        // every other form (note, pseudo-citation, other item) is a Literal.
        let (kind, target) = match item {
            Some(c) if c.file().is_some() => {
                (CitationKind::LocalFile, c.file().unwrap().to_owned())
            }
            Some(c) if c.text("url").is_some() => {
                (CitationKind::Url, c.text("url").unwrap().to_owned())
            }
            _ => (CitationKind::Literal, r.display_name()),
        };
        let exists = match (&kind, dir) {
            (CitationKind::LocalFile, Some(d)) => d.join(&target).exists(),
            _ => true,
        };
        CitationFacts {
            path,
            kind,
            target,
            exists,
            span,
        }
    };
    for c in &ir.components {
        if let Some(cit) = &c.citation {
            cites.push(cite_of(c.path.to_string(), cit, c.span.clone()));
        }
        for m in &c.members {
            if let Some(cit) = &m.citation {
                cites.push(cite_of(m.path.to_string(), cit, m.span.clone()));
            }
            if c.class == ComponentClass::Ots && m.kind == IrMemberKind::Param && m.value.is_some()
            {
                values.push(OtsValueFacts {
                    path: m.path.to_string(),
                    cited: m.citation.is_some(),
                    component_cited: c.citation.is_some(),
                    span: m.span.clone(),
                });
            }
        }
    }
    (values, cites)
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

/// The directory holding the `std` package: `$CADETTA_STD`, else the repository copy.
pub fn std_dir() -> Option<PathBuf> {
    let from_env = std::env::var_os(STD_ENV).map(PathBuf::from);
    let built_in = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../std")).to_path_buf();
    let found = from_env
        .into_iter()
        .chain(std::iter::once(built_in))
        .find(|d| d.join("cadetta.toml").is_file());
    tracing::debug!(dir = ?found, "pipeline: std package location");
    found
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
                    "E001",
                    Severity::Error,
                    format!("bad module path: {error}"),
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
        for d in &parsed.diags {
            let sev = if d.code.code() == "E001" {
                *halted = true;
                Severity::Error
            } else {
                Severity::Warning
            };
            tracing::debug!(file = %shown, code = d.code.code(), "pipeline: syntax diagnostic");
            out.push(diag_at(
                d.code.code(),
                sev,
                d.message.clone(),
                None,
                Some(SourceIndex::location(
                    &IndexedFile {
                        name: shown.clone(),
                        package: name.clone(),
                        module: path.clone(),
                        is_std,
                        source: source.clone(),
                    },
                    d.span,
                )),
            ));
        }
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
    let mut analysis = Analysis::default();
    let mut halted = false;
    let mut parsed: Vec<ParsedPackage> = Vec::new();
    let mut loaded_std: Option<RawPackage> = None;
    let has_std_member = project.packages.iter().any(|p| package_name(p) == "std");
    if !has_std_member && let Some(dir) = std_dir() {
        match load_std(&dir) {
            Ok(p) => loaded_std = Some(p),
            Err(error) => {
                tracing::warn!(%error, "pipeline: std package could not be loaded");
            }
        }
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
    evaluate_stages(&parsed, &scope_names, &mut analysis);
    analysis
}

fn evaluate_stages(parsed: &[ParsedPackage], scope: &BTreeSet<String>, analysis: &mut Analysis) {
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
    let kinds = KindTable::builtin();
    let (resolver, resolve_errors) = Resolver::new(sources.clone());
    let mut evaluator = Evaluator::new(&resolver, &units, &sources);
    let procs = Processes::compile(&mut evaluator, &sources);
    let evaluation = evaluator.evaluate();
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
    let located = |code: &str, sev: Severity, message: String, span: Span| {
        let member_in = |f: &IndexedFile| {
            in_scope.iter().find(|p| p.name == f.package).and_then(|p| {
                let m = p.modules.iter().find(|m| m.path == f.module)?;
                member_path_at(&p.name, &m.path, &m.file, span)
            })
        };
        let file = index.attribute(span, &|f| member_in(f).is_some());
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
    // Kind diagnostics are exact: they come from one module at a time.
    let mut kind_keys: BTreeSet<(String, usize, usize)> = BTreeSet::new();
    for p in &in_scope {
        for m in &p.modules {
            let mut checker = KindChecker::new(&kinds, &units, &EmptyEnv);
            for c in m.file.items().filter_map(|i| match &i.kind {
                ItemKind::Component(c) => Some(c),
                _ => None,
            }) {
                checker.check_component(c);
            }
            let file = index.module(&p.name, &m.path);
            for d in checker.into_diagnostics() {
                kind_keys.insert((d.code.to_owned(), d.span.start, d.span.end));
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
    for e in &resolve_errors {
        out.extend(located(e.code, Severity::Error, e.message.clone(), e.span));
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
                for d in cites.check_module(src, m) {
                    out.extend(located(d.code, Severity::Error, d.message, d.span));
                }
            }
        }
    }
    for d in &evaluation.diagnostics {
        // C001/C002 are reported once, from the structured check results below.
        if d.code == "C001" || d.code == "C002" {
            continue;
        }
        if kind_keys.contains(&(d.code.to_owned(), d.span.start, d.span.end)) {
            continue;
        }
        out.extend(located(
            d.code,
            sema_severity(d.severity),
            d.message.clone(),
            d.span,
        ));
    }
    for d in graph.diagnostics() {
        out.extend(located(
            d.code,
            sema_severity(d.severity),
            d.message.clone(),
            d.span,
        ));
    }
    for r in &evaluation.checks {
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
    let mut lowered = Vec::new();
    for p in &in_scope {
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
        let input = LowerInput {
            name: &p.name,
            version: &p.version,
            packages: &sources,
            sources: &texts,
            package_dir: Some(&p.dir),
            evaluation: &evaluation,
            compensation: Some(&outcome),
            deps: &graph,
            variants: BTreeMap::new(),
            fits: &[],
        };
        let ir = lower(&input);
        let locate = |s: &cadetta_ir::model::SpanRef| index.locate_ref(&s.file, s.start, s.end);
        out.extend(unused::check(&ir, locate));
        out.extend(interface::check_alternates(&ir));
        // IR-only rule families run with the catalog severities; `check` applies `[lint]`
        // overrides afterwards through the engine.
        let lcfg = LintConfig::new();
        out.extend(d::check_d001(&ir, &lcfg, &d::DConfig::default(), locate));
        out.extend(d::check_d002(&ir, &lcfg, &d::DConfig::default(), locate));
        out.extend(f::check_f004(&ir, &lcfg, locate));
        out.extend(m::check_m001(&ir, &lcfg, &m::MConfig::default(), locate));
        out.extend(m::check_m002(&ir, &lcfg, locate));
        out.extend(m::check_m003(&ir, &lcfg, locate));
        out.extend(n::check(&ir, &lcfg, locate));
        out.extend(q::check_inexact(&ir, &lcfg, locate));
        let (ots_values, cites) = s_facts(&ir, Some(&p.dir));
        out.extend(s::check_s001(&ots_values, locate));
        out.extend(s::check_s002(&cites, locate));
        for m in &p.modules {
            module_waivers(&m.file, &mut analysis.suppressions.waivers);
        }
        lowered.push(LoweredPackage {
            name: p.name.clone(),
            ir,
        });
    }
    tracing::info!(
        diagnostics = out.len(),
        packages = lowered.len(),
        "pipeline: analysis complete"
    );
    analysis.diagnostics.extend(out);
    analysis.packages = lowered;
}
