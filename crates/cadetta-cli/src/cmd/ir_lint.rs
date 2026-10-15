//! frob:ticket 01M4YMHQ5BQW8T67PVT5ZFM9K6
//! `cadetta lint FILE [--format json]`: run the IR-only lint rules on a `cadetta.ir/1` document
//! with catalog severities and no project context (config-cli.md 3.9).
//!
//! A document `ir-validate` rejects exits 2 before any rule runs. Today the rules that read only
//! the decoded model run: D001, D002, D003, F004, M001-M003, N, Q006 and S001/S002/S008. The
//! decoder reads every top-level entity, so D002 sees the document's table cells. IR spans are
//! byte offsets into the source file they name; that file is looked up beside the IR document and
//! offsets become real line and column. An unreadable source falls back to line 1, column 1.
// frob:ticket 01M4YP4QF161QNHCR0Y06GWDX5

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use cadetta_ir::decode::decode_project;
use cadetta_ir::entities::Cell;
use cadetta_ir::model::{Nominal, Project, SpanRef};
use cadetta_lint::diag::{self, Diagnostic, Location, Position};
use cadetta_lint::engine::LintConfig;
use cadetta_lint::rules::d::{self, DConfig, Key, NamedValue};
use cadetta_lint::rules::m::MConfig;
use cadetta_lint::rules::{c, f, m, n, q, s};
use cadetta_syntax::span::SourceFile;
use serde_json::Value;

use crate::cmd::ir_validate::{load_json, violations};
use crate::config::FailOn;
use crate::output::{self, Format};
use crate::pipeline::s_facts;
use crate::{EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta lint FILE [--format json]";

/// The parsed command line of `cadetta lint FILE`.
#[derive(Debug, PartialEq, Eq)]
pub struct LintArgs {
    /// The IR document.
    pub file: String,
    /// True for `--format json`.
    pub json: bool,
}

/// Parse the words after `lint`; the error is a usage message.
pub fn parse_args(args: &[String]) -> Result<LintArgs, String> {
    let mut file = None;
    let mut json = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--format" => match it.next().map(String::as_str) {
                Some("json") => json = true,
                Some(other) => {
                    return Err(format!("`--format` accepts only `json`, got `{other}`"));
                }
                None => return Err("`--format` needs a value".to_owned()),
            },
            flag if flag.starts_with("--") => return Err(format!("unknown flag `{flag}`")),
            name if file.is_none() => file = Some(name.to_owned()),
            extra => return Err(format!("unexpected argument `{extra}`")),
        }
    }
    Ok(LintArgs {
        file: file.ok_or_else(|| "missing FILE".to_owned())?,
        json,
    })
}

/// Every exact quantity table cell of the document, named `Table[key].column` for D002.
// frob:ticket 01M4YP4QF161QNHCR0Y06GWDX5
fn table_cells(ir: &Project) -> Vec<NamedValue> {
    let mut out = Vec::new();
    for t in &ir.tables {
        for row in &t.rows {
            let key_text = match &row.key {
                Cell::Value(v) => v.authored.clone(),
                Cell::Variant(name) => name.clone(),
            };
            for (col, cell) in t.cols.iter().zip(&row.cells) {
                let Cell::Value(v) = cell else { continue };
                let Nominal::Exact(exact) = &v.nominal else {
                    tracing::trace!(table = %t.path.item, "IR lint: inexact table cell skipped");
                    continue;
                };
                out.push(NamedValue {
                    name: format!("{}[{key_text}].{}", t.path.item, col.name),
                    key: Key::new(&v.dim, &exact.0),
                });
            }
        }
    }
    tracing::debug!(cells = out.len(), "IR lint: table cells collected for D002");
    out
}

/// A span's source location: byte offsets resolved against the file beside the IR document
/// (`dir`), or line 1, column 1 when no such file is readable.
// frob:ticket 01M4YP4QF161QNHCR0Y06GWDX5
fn locate_span(
    span: &SpanRef,
    dir: Option<&Path>,
    cache: &RefCell<HashMap<String, Option<SourceFile>>>,
) -> Location {
    let mut files = cache.borrow_mut();
    let source = files.entry(span.file.clone()).or_insert_with(|| {
        let text = dir.and_then(|d| std::fs::read_to_string(d.join(&span.file)).ok());
        if text.is_none() {
            tracing::debug!(file = %span.file, "IR lint: source not readable; reporting line 1");
        }
        text.map(|t| SourceFile::new(span.file.clone(), t))
    });
    let at = |offset: u32| match &*source {
        Some(f) => {
            let lc = f.line_col(offset as usize);
            Position {
                line: u32::try_from(lc.line).unwrap_or(u32::MAX),
                col: u32::try_from(lc.col).unwrap_or(u32::MAX),
            }
        }
        None => Position { line: 1, col: 1 },
    };
    Location::Source {
        file: span.file.clone(),
        start: at(span.start),
        end: at(span.end),
    }
}

/// The IR-only findings of a schema-valid document, sorted. `dir` is the directory span files
/// are resolved against (the IR file's own); the package directory is unknown, so S002 treats
/// every cited file as present.
pub fn lint_document(doc: &Value, dir: Option<&Path>) -> Result<Vec<Diagnostic>, String> {
    let ir = decode_project(doc)?;
    let cache = RefCell::new(HashMap::new());
    let locate = |span: &SpanRef| locate_span(span, dir, &cache);
    // Catalog severities and default rule settings: `lint FILE` has no `[lint]` table.
    let lcfg = LintConfig::new();
    let dcfg = DConfig {
        table_cells: table_cells(&ir),
        ..DConfig::default()
    };
    let mcfg = MConfig::default();
    let mut found = Vec::new();
    found.extend(d::check_d001(&ir, &lcfg, &dcfg, locate));
    found.extend(d::check_d002(&ir, &lcfg, &dcfg, locate));
    found.extend(d::check_d003(&ir, locate));
    found.extend(f::check_f004(&ir, &lcfg, locate));
    found.extend(c::check_fit_ranges(&ir, locate));
    found.extend(m::check_m001(&ir, &lcfg, &mcfg, locate));
    found.extend(m::check_m002(&ir, &lcfg, locate));
    found.extend(m::check_m003(&ir, &lcfg, locate));
    found.extend(n::check(&ir, &lcfg, locate));
    found.extend(q::check_inexact(&ir, &lcfg, locate));
    let (ots_values, cites, own_cites) = s_facts(&ir, &ir.package.name, None);
    found.extend(s::check_s001(&ots_values, locate));
    found.extend(s::check_s002(&cites, locate));
    found.extend(s::check_s008(&own_cites, &ots_values, locate));
    diag::sort(&mut found);
    tracing::info!(count = found.len(), "IR lint finished");
    Ok(found)
}

/// Run `cadetta lint FILE`; `args` are the words after the verb. Returns the exit code.
pub fn run_lint_ir(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "lint failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(message) => return fail(err, &format!("{message}\n{USAGE}")),
    };
    let doc = match load_json(Path::new(&parsed.file)) {
        Ok(doc) => doc,
        Err(message) => return fail(err, &message),
    };
    let invalid = violations(&doc);
    if !invalid.is_empty() {
        for line in &invalid {
            let _ = writeln!(err, "{line}");
        }
        return fail(
            err,
            &format!("{}: not a valid cadetta.ir/1 document", parsed.file),
        );
    }
    let diagnostics = match lint_document(&doc, Path::new(&parsed.file).parent()) {
        Ok(d) => d,
        Err(message) => return fail(err, &message),
    };
    let format = if parsed.json {
        Format::Json
    } else {
        globals.format
    };
    let text = output::render(format, &diagnostics, &|_, _| None);
    if let Err(e) = out.write_all(text.as_bytes()) {
        return fail(err, &format!("could not write output: {e}"));
    }
    output::exit_code(&diagnostics, FailOn::Error)
}
