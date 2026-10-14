//! frob:ticket 01M4SWN8DY2SQYR5GCTQQM16Z9
//! Command implementations (config-cli.md 3), one module per verb family.

pub mod ack;
pub mod bind;
pub mod check;
pub(crate) mod check_drift;
pub mod check_watch;
pub mod cite;
pub mod compare;
pub mod emit;
pub mod explain;
pub mod fmt;
pub mod impact;
pub mod impact_since;
pub mod impact_swap;
pub mod import;
pub mod init;
pub mod ir;
pub mod lock;
pub mod lsp;
pub mod names;
pub mod obligations;
pub mod solve;
pub mod swap;
pub mod sync;
pub mod units;
pub mod update;
pub mod why;

use std::io::Write;

use crate::project::Project;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

pub use ack::run_ack;
pub use bind::run_bind;
pub use check::run_check;
pub use compare::run_compare;
pub use emit::run_emit;
pub use explain::run_explain;
pub use fmt::run_fmt;
pub use import::run_import;
pub use init::run_init;
pub use ir::run_ir;
pub use lock::run_lock;
pub use lsp::run_lsp;
pub use names::run_names;
pub use obligations::run_obligations;
pub use solve::run_solve;
pub use swap::run_swap;
pub use sync::run_sync;
pub use units::run_units;
pub use update::run_update;
pub use why::run_why;

/// Run `cadetta impact`: `--swap PART NEW` previews a part swap, `--since REV` / `--worktree` diffs two evaluations, otherwise `PATH` queries
/// downstream dependents.
pub fn run_impact(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    match args.split_first() {
        Some((flag, rest)) if flag == "--swap" => impact_swap::run_swap(rest, globals, out, err),
        Some((flag, _)) if flag == "--since" || flag == "--worktree" => {
            impact_since::run_since(args, globals, out, err)
        }
        _ => impact::run_impact(args, globals, out, err),
    }
}

/// Run `cite pin NAME [--as-of DATE]` or `cite list [--format F] [--module PATH] [GLOB...]`.
// frob:ticket 01M4VB63RTQENVNZ4DMXBFGJX3
pub fn run_cite(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    const USAGE: &str = "usage: cadetta cite pin NAME [--as-of DATE] | cite list [--format ieee|bibtex|json] [--module PATH] [GLOB...]";
    let result = match args.split_first() {
        Some((verb, rest)) if verb == "pin" => cite_pin(rest, globals, out),
        Some((verb, rest)) if verb == "list" => cite_list(rest, globals, out),
        Some((verb, _)) => Err(format!("unknown cite verb `{verb}`; {USAGE}")),
        None => Err(USAGE.to_owned()),
    };
    match result {
        Ok(()) => EXIT_OK,
        Err(message) => {
            tracing::error!(%message, "cite failed");
            let _ = writeln!(err, "error: {message}");
            EXIT_USAGE
        }
    }
}

/// `cite pin NAME [--as-of DATE]`: download the cited url and record it in the source.
fn cite_pin(args: &[String], globals: &Globals, out: &mut dyn Write) -> Result<(), String> {
    let mut name = None;
    let mut as_of = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(v) = arg.strip_prefix("--as-of=") {
            as_of = Some(v.to_owned());
        } else if arg == "--as-of" {
            i += 1;
            as_of = Some(args.get(i).cloned().ok_or("--as-of needs a date")?);
        } else if arg.starts_with("--") || name.is_some() {
            return Err(format!(
                "unexpected argument `{arg}`; usage: cadetta cite pin NAME [--as-of DATE]"
            ));
        } else {
            name = Some(arg.clone());
        }
        i += 1;
    }
    let name = name.ok_or("usage: cadetta cite pin NAME [--as-of DATE]")?;
    let today = match as_of {
        Some(text) => cadetta_obligations::as_of::Date::parse(&text)
            .map_err(|e| format!("--as-of {text}: {e}"))?,
        None => cadetta_obligations::as_of::Date::today(),
    };
    let fetch = |url: &str| -> Result<Vec<u8>, String> {
        let mut response = ureq::get(url).call().map_err(|e| e.to_string())?;
        response.body_mut().read_to_vec().map_err(|e| e.to_string())
    };
    let write = |path: &std::path::Path, bytes: &[u8]| -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)
    };
    let io = cite::Io {
        fetch: &fetch,
        write: &write,
    };
    let (project, scope) = Project::open(globals.cwd.as_deref(), globals.config.as_deref())
        .map_err(|e| e.to_string())?;
    let pinned = cite::pin(&project, &scope, &name, today, &io).map_err(|e| e.to_string())?;
    let _ = writeln!(
        out,
        "pinned {name}: {} -> {} (blake3:{})",
        pinned.url, pinned.local, pinned.digest
    );
    Ok(())
}

/// `cite list [--format F] [--module PATH] [GLOB...]`: the numbered reference list.
fn cite_list(args: &[String], globals: &Globals, out: &mut dyn Write) -> Result<(), String> {
    let mut format = None;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if let Some(v) = args[i].strip_prefix("--format=") {
            format = Some(v.to_owned());
        } else if args[i] == "--format" {
            i += 1;
            format = Some(
                args.get(i)
                    .cloned()
                    .ok_or("--format needs ieee, bibtex or json")?,
            );
        } else {
            rest.push(args[i].clone());
        }
        i += 1;
    }
    let format = match format {
        Some(text) => text.parse::<cite::ListFormat>()?,
        None if globals.format == crate::Format::Json => cite::ListFormat::Json,
        None => cite::ListFormat::Ieee,
    };
    let parsed = emit::parse_args(&rest)?;
    if !parsed.adapters.is_empty() {
        return Err("cite list takes no adapter id".to_owned());
    }
    let run_scope =
        crate::scope::Scope::new(&parsed.modules, &parsed.globs).map_err(|e| e.to_string())?;
    let (project, scope) = Project::open(globals.cwd.as_deref(), globals.config.as_deref())
        .map_err(|e| e.to_string())?;
    let text = cite::list(&project, &scope, &run_scope, format)?;
    let _ = write!(out, "{text}");
    Ok(())
}
