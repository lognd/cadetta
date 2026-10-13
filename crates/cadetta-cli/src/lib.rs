//! The `cadetta` command line as a library, so the binary and the Python
//! package (bindings/python) run the same code.
//!
//! Specification: `docs/spec/config-cli.md`. Only `--version` exists so far.

/// Version string printed by `cadetta --version`.
pub const VERSION: &str = concat!("cadetta ", env!("CARGO_PKG_VERSION"));

/// Exit code for success (config-cli.md 2).
pub const EXIT_OK: u8 = 0;

/// Exit code for a command-line usage or I/O error (config-cli.md 2).
pub const EXIT_USAGE: u8 = 2;

/// What the command line asked for.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Version,
    Unknown(String),
}

/// Parse the arguments after the program name.
fn parse(args: &[String]) -> Command {
    match args.first().map(String::as_str) {
        Some("--version" | "-V") | None => Command::Version,
        Some(other) => Command::Unknown(other.to_owned()),
    }
}

/// Run one command line (arguments after the program name), writing
/// command output to `out`; returns the process exit code.
pub fn run(args: &[String], out: &mut dyn std::io::Write) -> u8 {
    let command = parse(args);
    tracing::debug!(?command, "parsed command line");
    match command {
        Command::Version => match writeln!(out, "{VERSION}") {
            Ok(()) => EXIT_OK,
            Err(error) => {
                tracing::error!(%error, "could not write to stdout");
                EXIT_USAGE
            }
        },
        Command::Unknown(arg) => {
            tracing::error!(%arg, "unknown command; only --version exists so far");
            EXIT_USAGE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_to_string(args: &[&str]) -> (u8, String) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let mut out = Vec::new();
        let code = run(&args, &mut out);
        (code, String::from_utf8(out).expect("utf-8 output"))
    }

    #[test]
    fn no_args_and_version_flag_print_version() {
        assert_eq!(run_to_string(&[]), (EXIT_OK, format!("{VERSION}\n")));
        assert_eq!(
            run_to_string(&["--version"]),
            (EXIT_OK, format!("{VERSION}\n"))
        );
    }

    #[test]
    fn unknown_argument_is_a_usage_error() {
        assert_eq!(run_to_string(&["sync"]), (EXIT_USAGE, String::new()));
    }
}
