//! frob:ticket 01M4YMHQ5BQW8T67PVT5ZFM9K6
//! `cadetta lint FILE [--format json]`: run the IR-only lint rules on a `cadetta.ir/1` document
//! with catalog severities and no project context (config-cli.md 3.9).
//!
//! A document `ir-validate` rejects exits 2 before any rule runs. Today the rules that read only
//! the member dependency graph run (D003); the value-reading rules need a full IR decoder, which
//! is tracked separately.

use std::io::Write;
use std::path::Path;

use cadetta_ir::model::{Component, ComponentClass, Flavour, Member, MemberKind, PackageId};
use cadetta_ir::model::Project as Ir;
use cadetta_ir::path::Path as IrPath;
use cadetta_lint::diag::{self, Diagnostic};
use cadetta_lint::rules::d;
use serde_json::Value;

use crate::cmd::ir_validate::{load_json, violations};
use crate::config::FailOn;
use crate::output::{self, Format};
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

fn class_of(name: &str) -> ComponentClass {
    match name {
        "ots" => ComponentClass::Ots,
        "assembly" => ComponentClass::Assembly,
        "process" => ComponentClass::Process,
        _ => ComponentClass::Model,
    }
}

/// The lint-relevant skeleton of a schema-valid document: components with their members' paths
/// and `deps`. A path the IR path grammar rejects is an error message.
pub fn skeleton(doc: &Value) -> Result<Ir, String> {
    let parse = |v: &Value| -> Result<IrPath, String> {
        let text = v.as_str().unwrap_or_default();
        text.parse::<IrPath>().map_err(|e| format!("path `{text}`: {e}"))
    };
    let package = PackageId {
        name: doc["package"]["name"].as_str().unwrap_or_default().to_owned(),
        version: doc["package"]["version"].as_str().unwrap_or_default().to_owned(),
    };
    let flavour = if doc["flavour"] == "observed" { Flavour::Observed } else { Flavour::Design };
    let mut project = Ir::new(flavour, package);
    for c in doc["components"].as_array().into_iter().flatten() {
        let mut members = Vec::new();
        for m in c["members"].as_array().into_iter().flatten() {
            let mut member = Member::new(parse(&m["path"])?, MemberKind::Param);
            for dep in m["deps"].as_array().into_iter().flatten() {
                member.deps.push(parse(dep)?);
            }
            members.push(member);
        }
        project.components.push(Component {
            // a component path has no member tail; the IR path type requires one, and D003
            // never reads the component path, so a placeholder tail satisfies the grammar
            path: parse(&Value::String(format!("{}.component", c["path"].as_str().unwrap_or_default())))?,
            class: class_of(c["class"].as_str().unwrap_or_default()),
            params_sig: None,
            implements: Vec::new(),
            citation: None,
            attrs: Default::default(),
            members,
            parts: Vec::new(),
            fits: Vec::new(),
            datums: Vec::new(),
            points: Vec::new(),
            placements: Vec::new(),
            span: None,
            digest: None,
        });
    }
    Ok(project)
}

/// The IR-only findings of a schema-valid document, sorted.
pub fn lint_document(doc: &Value) -> Result<Vec<Diagnostic>, String> {
    let project = skeleton(doc)?;
    let mut found = d::check_d003(&project, |span| {
        tracing::debug!(file = %span.file, "IR members carry no spans");
        diag::Location::Source {
            file: span.file.clone(),
            start: diag::Position { line: 1, col: 1 },
            end: diag::Position { line: 1, col: 1 },
        }
    });
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
