//! frob:ticket 01M4YMHQ5BQW8T67PVT5ZFM9K6
//! `cadetta lint FILE [--format json]`: run the IR-only lint rules on a `cadetta.ir/1` document
//! with catalog severities and no project context (config-cli.md 3.9).
//!
//! A document `ir-validate` rejects exits 2 before any rule runs. Today the rules that read only
//! the decoded model run: D001, D002, D003, F004, M001-M003, N, Q006 and S001/S002/S008. Tables,
//! checks and bindings are not decoded, so D002 sees no table cells; locations are line 1.

use std::io::Write;
use std::path::Path;

use cadetta_ir::decode::decode_project;
use cadetta_ir::model::SpanRef;
use cadetta_lint::diag::{self, Diagnostic, Location, Position};
use cadetta_lint::engine::LintConfig;
use cadetta_lint::rules::d::{self, DConfig};
use cadetta_lint::rules::m::MConfig;
use cadetta_lint::rules::{f, m, n, q, s};
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
                Some(other) => return Err(format!("`--format` accepts only `json`, got `{other}`")),
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

/// The IR-only findings of a schema-valid document, sorted. `dir` is `None`: no package
/// directory exists, so S002 treats every cited file as present.
pub fn lint_document(doc: &Value) -> Result<Vec<Diagnostic>, String> {
    let ir = decode_project(doc)?;
    let locate = |span: &SpanRef| {
        tracing::debug!(file = %span.file, "IR spans are byte offsets; reporting line 1");
        Location::Source {
            file: span.file.clone(),
            start: Position { line: 1, col: 1 },
            end: Position { line: 1, col: 1 },
        }
    };
    // Catalog severities and default rule settings: `lint FILE` has no `[lint]` table.
    let lcfg = LintConfig::new();
    let dcfg = DConfig::default();
    let mcfg = MConfig::default();
    let mut found = Vec::new();
    found.extend(d::check_d001(&ir, &lcfg, &dcfg, locate));
    found.extend(d::check_d002(&ir, &lcfg, &dcfg, locate));
    found.extend(d::check_d003(&ir, locate));
    found.extend(f::check_f004(&ir, &lcfg, locate));
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
pub fn run_lint_ir(args: &[String], globals: &Globals, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
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
        return fail(err, &format!("{}: not a valid cadetta.ir/1 document", parsed.file));
    }
    let diagnostics = match lint_document(&doc) {
        Ok(d) => d,
        Err(message) => return fail(err, &message),
    };
    let format = if parsed.json { Format::Json } else { globals.format };
    let text = output::render(format, &diagnostics, &|_, _| None);
    if let Err(e) = out.write_all(text.as_bytes()) {
        return fail(err, &format!("could not write output: {e}"));
    }
    output::exit_code(&diagnostics, FailOn::Error)
}
