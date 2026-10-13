//! frob:ticket 01M4SWN7JR7PMBQ4QEHBZ6BN9J
//! `cadetta swap SLOT ALT --reason "..."`: apply the `impact --swap` suggested edit to source
//! and append a `swap` lock record; a blocked preview writes nothing (obligations.md 3, 5.1).

use std::io::Write;
use std::path::PathBuf;

use cadetta_obligations::as_of::Date as Today;
use cadetta_obligations::lock::Lock;
use cadetta_obligations::swap as swap_record;
use cadetta_syntax::ast::{ItemKind, MemberKind};
use cadetta_syntax::parser::parse;

use super::ack::git_email;
use super::impact_swap::{self, SwapError, Verdict, preview_with};
use crate::pipeline::analyze;
use crate::project::Project;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Exit code of a blocked swap: nothing was written (config-cli.md 3).
pub const EXIT_BLOCKED: u8 = 1;

const USAGE: &str = "usage: cadetta swap SLOT ALT --reason \"...\" [--as EMAIL]";

/// The parsed command line of `cadetta swap`.
#[derive(Debug, PartialEq, Eq)]
pub struct SwapArgs {
    /// The slot path.
    pub slot: String,
    /// The replacement component.
    pub alt: String,
    /// Why the swap is made.
    pub reason: String,
    /// Identity overriding `git config user.email`.
    pub as_email: Option<String>,
}

/// Parse the words after `swap`; the error is a usage message.
pub fn parse_args(args: &[String]) -> Result<SwapArgs, String> {
    let mut positional = Vec::new();
    let (mut reason, mut as_email) = (None, None);
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("`{flag}` needs a value"))
        };
        match arg.as_str() {
            "--reason" => reason = Some(value("--reason")?),
            "--as" => as_email = Some(value("--as")?),
            flag if flag.starts_with("--") => return Err(format!("unknown flag `{flag}`")),
            word => positional.push(word.to_owned()),
        }
    }
    let [slot, alt] = <[String; 2]>::try_from(positional)
        .map_err(|_| "expected exactly SLOT and ALT".to_owned())?;
    Ok(SwapArgs {
        slot,
        alt,
        reason: reason.ok_or("missing `--reason`")?,
        as_email,
    })
}

/// Why the source edit could not be made.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EditError {
    /// No `part` declaration of the slot was found in the project sources.
    #[error("no `part {0}` declaration found in the project sources")]
    NotFound(String),
    /// Several declarations matched; the slot is ambiguous.
    #[error("`part {0}` is declared in several places; cannot edit safely")]
    Ambiguous(String),
    /// The declaration has no `= Primary`, so there is nothing to replace.
    #[error("`part {0}` has no primary (`= Component`); edit it by hand")]
    NoPrimary(String),
}

/// Split a canonical slot path into its component name and slot name.
fn split_slot(slot: &str) -> (&str, &str) {
    let tail = slot.rsplit("::").next().unwrap_or(slot);
    tail.split_once(['.', '/']).unwrap_or(("", tail))
}

/// The byte range of the `Primary [alt [...]]` text of the `part` declaring `slot` in `src`,
/// or `None` when `src` declares no such part.
fn primary_range(src: &str, slot: &str) -> Result<Option<(usize, usize)>, EditError> {
    let (comp, name) = split_slot(slot);
    let parsed = parse(src);
    let mut found = Vec::new();
    for item in parsed.file.items() {
        let ItemKind::Component(c) = &item.kind else {
            continue;
        };
        if !comp.is_empty() && c.name.name != comp {
            continue;
        }
        for m in &c.members {
            if let MemberKind::Part(p) = &m.kind
                && p.name.name == name
            {
                let primary = p
                    .primary
                    .as_ref()
                    .ok_or_else(|| EditError::NoPrimary(name.to_owned()))?;
                let mut end = primary.span.end;
                let rest = &src[end..];
                let trimmed = rest.trim_start();
                if trimmed.starts_with("alt")
                    && let Some(close) = rest.find(']')
                {
                    end += close + 1;
                }
                found.push((primary.span.start, end));
            }
        }
    }
    match found.len() {
        0 => Ok(None),
        1 => Ok(Some(found[0])),
        _ => Err(EditError::Ambiguous(name.to_owned())),
    }
}

/// `src` with the `part` of `slot` re-pointed per `suggested_edit` (the text after ` = `).
///
/// `Ok(None)` when `src` does not declare the slot.
pub fn edit_source(
    src: &str,
    slot: &str,
    suggested_edit: &str,
) -> Result<Option<String>, EditError> {
    let Some((start, end)) = primary_range(src, slot)? else {
        return Ok(None);
    };
    let (_, name) = split_slot(slot);
    let replacement = suggested_edit
        .split_once(" = ")
        .map(|(_, r)| r)
        .ok_or_else(|| EditError::NoPrimary(name.to_owned()))?;
    Ok(Some(format!(
        "{}{replacement}{}",
        &src[..start],
        &src[end..]
    )))
}

/// The exit code that stops the swap before anything is written, or `None` to proceed.
fn gate(swap: &impact_swap::Swap, err: &mut dyn Write) -> Option<u8> {
    if swap.verdict != Verdict::Blocked {
        return None;
    }
    tracing::warn!(slot = %swap.slot, "swap blocked; nothing written");
    let _ = writeln!(err, "swap blocked: nothing written");
    Some(EXIT_BLOCKED)
}

/// Run `cadetta swap`; `args` are the words after `swap`. Returns the exit code.
pub fn run_swap(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "swap failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(message) => return fail(err, &format!("{message}; {USAGE}")),
    };
    let (project, members) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref())
    {
        Ok(o) => o,
        Err(e) => return fail(err, &e.to_string()),
    };
    let Some(by) = parsed.as_email.clone().or_else(|| git_email(&project.root)) else {
        return fail(
            err,
            "cannot determine the author: set git user.email or pass `--as`",
        );
    };
    let analysis = analyze(&project, &members);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    let lock_path = project.root.join("cadetta.lock");
    let mut lock = match Lock::load(&lock_path) {
        Ok(l) => l,
        Err(e) => return fail(err, &e.to_string()),
    };
    let mut last = SwapError::NoSlot(parsed.slot.clone());
    let mut found = None;
    for package in &analysis.packages {
        match preview_with(&package.ir, &parsed.slot, &parsed.alt, &[]) {
            Ok(swap) => {
                let from = package
                    .ir
                    .components
                    .iter()
                    .flat_map(|c| &c.parts)
                    .find(|p| p.path.to_string() == swap.slot)
                    .map(|p| p.primary.clone())
                    .unwrap_or_default();
                found = Some((swap, from));
                break;
            }
            Err(SwapError::NoSlot(_)) => {}
            Err(e) => last = e,
        }
    }
    let Some((swap, from)) = found else {
        return fail(err, &last.to_string());
    };
    impact_swap::render(&swap, globals.format, out);
    if let Some(code) = gate(&swap, err) {
        return code;
    }
    let record = match swap_record::record(
        &swap.slot,
        &from,
        &swap.new,
        &by,
        &Today::today().to_string(),
        &parsed.reason,
    ) {
        Ok(r) => r,
        Err(e) => return fail(err, &e.to_string()),
    };

    // Find and edit the one source file declaring the slot.
    let mut edits: Vec<(PathBuf, String)> = Vec::new();
    for module in project.packages.iter().flat_map(|p| &p.modules) {
        match edit_source(&module.source, &swap.slot, &swap.suggested_edit) {
            Ok(Some(text)) => edits.push((module.file.clone(), text)),
            Ok(None) => {}
            Err(e) => return fail(err, &e.to_string()),
        }
    }
    let mut edits = edits.into_iter();
    let (Some((file, text)), None) = (edits.next(), edits.next()) else {
        return fail(
            err,
            &format!("expected exactly one source file declaring `{}`", swap.slot),
        );
    };
    let original = match std::fs::read_to_string(&file) {
        Ok(t) => t,
        Err(e) => return fail(err, &e.to_string()),
    };
    if let Err(e) = std::fs::write(&file, &text) {
        return fail(err, &format!("{}: {e}", file.display()));
    }
    swap_record::append(&mut lock, record);
    if let Err(e) = lock.save(&lock_path) {
        // Keep source and lock consistent: undo the edit when the lock cannot be written.
        if let Err(undo) = std::fs::write(&file, original) {
            tracing::error!(error = %undo, "swap: could not restore the source after a lock failure");
        }
        return fail(err, &e.to_string());
    }
    tracing::info!(slot = %swap.slot, to = %swap.new, file = %file.display(), "swap applied");
    let _ = writeln!(
        out,
        "swapped {} -> {} in {}",
        swap.slot,
        swap.new,
        file.display()
    );
    EXIT_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "assembly Rover {\n  part wheel[4]: Caster = Vendor alt [Colson, Other]\n}\n";
    const SLOT: &str = "pkg::Rover.wheel";
    const EDIT: &str = "part wheel[4]: Caster = Colson alt [Other, Vendor]";

    // frob:tests crates/cadetta-cli/src/cmd/swap.rs::edit_source
    #[test]
    fn edit_rewrites_only_the_primary_and_alt_list() {
        let got = edit_source(SRC, SLOT, EDIT).unwrap().unwrap();
        assert_eq!(
            got,
            "assembly Rover {\n  part wheel[4]: Caster = Colson alt [Other, Vendor]\n}\n"
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/swap.rs::edit_source
    #[test]
    fn edit_ignores_other_components_and_reports_absence() {
        assert_eq!(edit_source(SRC, "pkg::Other.wheel", EDIT), Ok(None));
        assert_eq!(edit_source(SRC, "pkg::Rover.axle", EDIT), Ok(None));
    }

    // frob:tests crates/cadetta-cli/src/cmd/swap.rs::edit_source
    #[test]
    fn edit_handles_part_without_alt() {
        let src = "assembly Rover {\n  part wheel: Caster = Vendor\n}\n";
        let got = edit_source(src, SLOT, "part wheel: Caster = Colson alt [Vendor]")
            .unwrap()
            .unwrap();
        assert!(got.contains("= Colson alt [Vendor]\n}"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/swap.rs::parse_args
    #[test]
    fn args_need_slot_alt_and_reason() {
        let a = |v: &[&str]| parse_args(&v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
        assert!(a(&["s", "x", "--reason", "why"]).is_ok());
        assert!(a(&["s", "x"]).is_err());
        assert!(a(&["s", "--reason", "why"]).is_err());
        assert!(a(&["s", "x", "--reason", "w", "--bogus"]).is_err());
    }

    const PROJECT: &str = "interface Fan {\n  depth: Length\n}\n\
ots Old: Fan {\n  depth = 25mm\n}\n\
ots Thin: Fan {\n  depth = 20mm\n}\n\
ots Fat: Fan {\n  depth = 40mm\n}\n\
assembly Rig {\n  part fan: Fan = Old alt [Thin, Fat]\n  check bay: fan.depth <= 30mm\n}\n";

    /// A scratch project dir holding `PROJECT`; returns (dir, globals).
    fn scratch(tag: &str) -> (PathBuf, Globals) {
        let dir = std::env::temp_dir().join(format!("cadetta-swap-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("cadetta.toml"),
            "[package]\nname = \"rig\"\nversion = \"0.1.0\"\nas_of = \"2026-10-13\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("src/lib.cdt"), PROJECT).unwrap();
        let globals = Globals {
            format: crate::output::Format::Text,
            cwd: Some(dir.clone()),
            config: None,
        };
        (dir, globals)
    }

    fn words(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    // frob:tests crates/cadetta-cli/src/cmd/swap.rs::run_swap
    #[test]
    fn unblocked_swap_edits_source_and_writes_lock_record() {
        let (dir, globals) = scratch("ok");
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_swap(
            &words(&["fan", "Thin", "--reason", "slimmer", "--as", "a@b.c"]),
            &globals,
            &mut out,
            &mut err,
        );
        assert_eq!(code, EXIT_OK, "{}", String::from_utf8_lossy(&err));
        let src = std::fs::read_to_string(dir.join("src/lib.cdt")).unwrap();
        assert!(src.contains("part fan: Fan = Thin alt [Fat, Old]"), "{src}");
        let lock = std::fs::read_to_string(dir.join("cadetta.lock")).unwrap();
        assert!(lock.contains("from=Old to=Thin by=a@b.c"), "{lock}");
        assert!(lock.contains("reason=\"slimmer\""), "{lock}");
        let _ = std::fs::remove_dir_all(dir);
    }

    // frob:tests crates/cadetta-cli/src/cmd/swap.rs::gate
    #[test]
    fn blocked_verdict_stops_before_any_write() {
        let mut swap = impact_swap::Swap {
            slot: "rig::Rig.fan".into(),
            new: "Fat".into(),
            approved: true,
            verdict: Verdict::Blocked,
            values: vec![],
            flips: vec![],
            suggested_edit: String::new(),
            report: Default::default(),
        };
        let mut err = Vec::new();
        assert_eq!(gate(&swap, &mut err), Some(EXIT_BLOCKED));
        assert!(String::from_utf8(err).unwrap().contains("nothing written"));
        swap.verdict = Verdict::OkWithWarnings;
        assert_eq!(gate(&swap, &mut Vec::new()), None);
    }
}
