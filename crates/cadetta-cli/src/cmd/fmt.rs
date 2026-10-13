//! frob:ticket 01M4SPWT0RZH7FJYEZ8J7MBN3Z
//! `cadetta fmt [--check]`: canonical formatting of package sources (config-cli.md 3).

use std::io::Write;

use cadetta_syntax::format::format_source;

use crate::project::{Module, Project};
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Exit code when `--check` finds a file that is not canonical.
pub const EXIT_DIFF: u8 = 1;

/// Format `modules` (rewriting files unless `check`); returns the exit code.
///
/// `--check` exits 1 when any file differs and writes nothing. A file with
/// syntax errors cannot be formatted: it is reported and the run exits 2.
pub fn format_modules(
    modules: &[&Module],
    check: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let (mut differing, mut broken) = (0usize, 0usize);
    for module in modules {
        match format_source(&module.source) {
            Ok(formatted) if formatted == module.source => {
                tracing::debug!(file = %module.file.display(), "fmt: already canonical");
            }
            Ok(formatted) => {
                differing += 1;
                if check {
                    tracing::info!(file = %module.file.display(), "fmt: would reformat");
                    let _ = writeln!(out, "would reformat {}", module.file.display());
                } else if let Err(error) = std::fs::write(&module.file, formatted) {
                    tracing::error!(file = %module.file.display(), %error, "fmt: write failed");
                    let _ = writeln!(err, "error: {}: {error}", module.file.display());
                    broken += 1;
                } else {
                    tracing::info!(file = %module.file.display(), "fmt: reformatted");
                    let _ = writeln!(out, "reformatted {}", module.file.display());
                }
            }
            Err(error) => {
                tracing::error!(file = %module.file.display(), %error, "fmt: cannot format");
                let _ = writeln!(err, "error: {}: {error}", module.file.display());
                broken += 1;
            }
        }
    }
    if broken > 0 {
        EXIT_USAGE
    } else if check && differing > 0 {
        EXIT_DIFF
    } else {
        EXIT_OK
    }
}

/// Run `cadetta fmt [--check]`; `args` are the words after `fmt`.
pub fn run_fmt(args: &[String], globals: &Globals, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let mut check = false;
    for arg in args {
        if arg == "--check" {
            check = true;
        } else {
            tracing::error!(%arg, "fmt: unknown argument");
            let _ = writeln!(
                err,
                "error: unknown argument `{arg}`; usage: cadetta fmt [--check]"
            );
            return EXIT_USAGE;
        }
    }
    let (project, scope) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok(opened) => opened,
        Err(error) => {
            tracing::error!(%error, "fmt: no project");
            let _ = writeln!(err, "error: {error}");
            return EXIT_USAGE;
        }
    };
    let modules: Vec<&Module> = scope
        .iter()
        .flat_map(|&i| project.packages[i].modules.iter())
        .collect();
    format_modules(&modules, check, out, err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(name: &str, source: &str) -> Module {
        let file = std::env::temp_dir().join(format!(
            "cadetta-fmt-test-{}-{name}.cdt",
            std::process::id()
        ));
        std::fs::write(&file, source).unwrap();
        Module {
            rel_path: name.to_owned(),
            file,
            source: source.to_owned(),
        }
    }

    const UGLY: &str = "model   M {\n  a: Length   = 1mm\n}\n";

    // frob:tests crates/cadetta-cli/src/cmd/fmt.rs::format_modules
    #[test]
    fn check_exits_one_on_unformatted_and_writes_nothing() {
        let m = module("check", UGLY);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(format_modules(&[&m], true, &mut out, &mut err), EXIT_DIFF);
        assert_eq!(std::fs::read_to_string(&m.file).unwrap(), UGLY);
    }

    // frob:tests crates/cadetta-cli/src/cmd/fmt.rs::format_modules
    #[test]
    fn write_mode_formats_then_check_passes() {
        let m = module("write", UGLY);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(format_modules(&[&m], false, &mut out, &mut err), EXIT_OK);
        let formatted = std::fs::read_to_string(&m.file).unwrap();
        assert_ne!(formatted, UGLY);
        let again = module("again", &formatted);
        assert_eq!(format_modules(&[&again], true, &mut out, &mut err), EXIT_OK);
    }

    // frob:tests crates/cadetta-cli/src/cmd/fmt.rs::format_modules
    #[test]
    fn syntax_error_is_exit_two() {
        let m = module("bad", "model {{{\n");
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(format_modules(&[&m], true, &mut out, &mut err), EXIT_USAGE);
    }
}
