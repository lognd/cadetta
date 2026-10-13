//! frob:ticket 01M4SY4XVHM745JWB285NFC7G7
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

/// Split leading `-v`, `-vv`, `-vvv` flags (config-cli.md 4) off `args`;
/// returns the verbosity count and the remaining arguments.
pub fn split_verbosity(args: &[String]) -> (u8, Vec<String>) {
    let mut verbosity: u8 = 0;
    let mut rest = Vec::with_capacity(args.len());
    for arg in args {
        let is_v = arg.len() > 1 && arg.starts_with('-') && arg[1..].bytes().all(|b| b == b'v');
        if is_v {
            verbosity = verbosity.saturating_add(u8::try_from(arg.len() - 1).unwrap_or(u8::MAX));
        } else {
            rest.push(arg.clone());
        }
    }
    (verbosity, rest)
}

/// Install logging (`CADETTA_LOG`, then `-v` count), then [`run`].
///
/// An invalid `CADETTA_LOG` is a usage error reported on `err`, never a
/// panic; an already installed subscriber (the Python host) is fine.
pub fn run_with_logging(
    args: &[String],
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
) -> u8 {
    let (verbosity, rest) = split_verbosity(args);
    match cadetta_log::init("cadetta", verbosity, false) {
        Ok(()) => {}
        Err(cadetta_log::InitError::AlreadyInitialized(_)) => {
            tracing::debug!("logging already initialised by the host");
        }
        Err(error) => {
            // No subscriber exists yet, so the usage error goes straight to `err`.
            let _ = writeln!(err, "error: {error}");
            return EXIT_USAGE;
        }
    }
    run(&rest, out)
}

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

    // frob:tests crates/cadetta-cli/src/lib.rs::run_with_logging
    #[test]
    fn run_with_logging_installs_once_and_runs() {
        let args: Vec<String> = ["-v", "--version"].map(str::to_owned).to_vec();
        for _ in 0..2 {
            let (mut out, mut err) = (Vec::new(), Vec::new());
            assert_eq!(run_with_logging(&args, &mut out, &mut err), EXIT_OK);
            assert_eq!(out, format!("{VERSION}\n").into_bytes());
        }
    }

    // frob:tests crates/cadetta-cli/src/lib.rs::split_verbosity
    #[test]
    fn verbosity_flags_are_split_off() {
        let args: Vec<String> = ["-v", "--version", "-vv"].map(str::to_owned).to_vec();
        assert_eq!(split_verbosity(&args), (3, vec!["--version".to_owned()]));
        assert_eq!(split_verbosity(&[]), (0, vec![]));
    }

    #[test]
    fn unknown_argument_is_a_usage_error() {
        assert_eq!(run_to_string(&["sync"]), (EXIT_USAGE, String::new()));
    }
}
