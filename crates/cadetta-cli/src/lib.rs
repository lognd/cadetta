//! frob:ticket 01M4SY4XVHM745JWB285NFC7G7
//! The `cadetta` command line as a library, so the binary and the Python
//! package (bindings/python) run the same code.
//!
//! Specification: `docs/spec/config-cli.md`. Global flags (`--format`, `--cwd`,
//! `--config`, `-v`) are parsed here; only `--version` exists as a command so far.

pub mod cmd;
pub mod config;
mod env;
pub mod lint_facts;
pub mod output;
pub mod pipeline;
pub mod pkg;
pub mod project;
pub mod scope;
pub mod std_embed;
pub mod universe;

use std::path::PathBuf;

use output::Format;

/// Version string printed by `cadetta --version`.
pub const VERSION: &str = concat!("cadetta ", env!("CARGO_PKG_VERSION"));

/// Exit code for success (config-cli.md 2).
pub const EXIT_OK: u8 = 0;

/// Exit code for a failed check or emit (config-cli.md 2).
pub const EXIT_FAIL: u8 = 1;

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
    let env_format = env::format_var();
    run_with_env(&rest, env_format.as_deref(), out, err)
}

/// The global flags (config-cli.md 4) resolved against the environment.
#[derive(Debug, PartialEq, Eq)]
pub struct Globals {
    /// Output format: `--format`, else `CADETTA_FORMAT`, else text. Never
    /// chosen by terminal detection (config-cli.md 5).
    pub format: Format,
    /// `--cwd DIR`: where root discovery starts.
    pub cwd: Option<PathBuf>,
    /// `--config FILE`: an explicit `cadetta.toml`.
    pub config: Option<PathBuf>,
}

/// A bad global flag; always exit code 2.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UsageError {
    /// A flag that needs a value had none.
    #[error("`{0}` needs a value")]
    MissingValue(String),
    /// The `--format` / `CADETTA_FORMAT` value is not a format.
    #[error(transparent)]
    Format(#[from] output::UnknownFormat),
    /// `--cwd` names something that is not a directory.
    #[error("--cwd {0}: not a directory")]
    BadCwd(String),
    /// `--config` names something that is not a file.
    #[error("--config {0}: not a file")]
    BadConfig(String),
}

/// Take the value of flag `name` at `args[i]`, from `--name=VALUE` or the next argument.
fn flag_value(args: &[String], i: &mut usize, name: &str) -> Result<Option<String>, UsageError> {
    let arg = &args[*i];
    if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
        return Ok(Some(value.to_owned()));
    }
    if arg != name {
        return Ok(None);
    }
    *i += 1;
    args.get(*i)
        .cloned()
        .map(Some)
        .ok_or_else(|| UsageError::MissingValue(name.to_owned()))
}

/// Split the global flags off `args`; returns them and the remaining arguments.
///
/// `env_format` is the value of `CADETTA_FORMAT`; an explicit `--format` wins.
pub fn parse_globals(
    args: &[String],
    env_format: Option<&str>,
) -> Result<(Globals, Vec<String>), UsageError> {
    let (mut format, mut cwd, mut config) = (None, None, None);
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        // frob:ticket 01M4VB63RTQENVNZ4DMXBFGJX3
        // `cite list` owns a `--format ieee|bibtex|json` of its own; leave it for the verb.
        let cite_list = rest.len() >= 2 && rest[0] == "cite" && rest[1] == "list";
        if !cite_list && let Some(v) = flag_value(args, &mut i, "--format")? {
            format = Some(v.parse::<Format>()?);
        } else if let Some(v) = flag_value(args, &mut i, "--cwd")? {
            if !std::path::Path::new(&v).is_dir() {
                return Err(UsageError::BadCwd(v));
            }
            cwd = Some(PathBuf::from(v));
        } else if let Some(v) = flag_value(args, &mut i, "--config")? {
            let path = cwd.as_deref().unwrap_or(std::path::Path::new(".")).join(&v);
            if !path.is_file() {
                return Err(UsageError::BadConfig(v));
            }
            config = Some(path);
        } else {
            rest.push(args[i].clone());
        }
        i += 1;
    }
    let format = match (format, env_format) {
        (Some(f), _) => f,
        (None, Some(text)) => text.parse()?,
        (None, None) => Format::Text,
    };
    tracing::debug!(?format, ?cwd, ?config, "resolved global flags");
    Ok((
        Globals {
            format,
            cwd,
            config,
        },
        rest,
    ))
}

/// What the command line asked for.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Version,
    Cite(Vec<String>),
    Init(Vec<String>),
    Fmt(Vec<String>),
    Solve(Vec<String>),
    Obligations(Vec<String>),
    Ack(Vec<String>),
    Check(Vec<String>),
    Why(Vec<String>),
    Impact(Vec<String>),
    /// `compare IFACE`.
    Compare(Vec<String>),
    Ir(Vec<String>),
    /// `ir-validate FILE`.
    IrValidate(Vec<String>),
    /// `lint FILE [--format json]`.
    LintIr(Vec<String>),
    Explain(Vec<String>),
    Units(Vec<String>),
    Update(Vec<String>),
    /// `swap SLOT ALT --reason`.
    Swap(Vec<String>),
    Emit(Vec<String>),
    /// `names [ADAPTER-ID] [--module PATH] [GLOB...]`.
    Names(Vec<String>),
    /// `bind PATH --adapter ID --locator L --kind K ...` / `bind --remove ...`.
    Bind(Vec<String>),
    Import(Vec<String>),
    /// `sync [--dry-run] [--only GLOB] [--force-design] [--no-regen] [--module PATH] [ADAPTER-ID...]`.
    Sync(Vec<String>),
    /// `lock verify`: check `cadetta.lock` (`--format json` for `cadetta.lock/1`).
    Lock(Vec<String>),
    /// `lsp`: the language server on stdio.
    Lsp(Vec<String>),
    Unknown(String),
}

/// Parse the arguments after the program name.
fn parse(args: &[String]) -> Command {
    match args.first().map(String::as_str) {
        Some("--version" | "-V") | None => Command::Version,
        Some("cite") => Command::Cite(args[1..].to_vec()),
        Some("init") => Command::Init(args[1..].to_vec()),
        Some("fmt") => Command::Fmt(args[1..].to_vec()),
        Some("solve") => Command::Solve(args[1..].to_vec()),
        Some("obligations") => Command::Obligations(args[1..].to_vec()),
        Some("ack") => Command::Ack(args[1..].to_vec()),
        Some("check") => Command::Check(args[1..].to_vec()),
        Some("why") => Command::Why(args[1..].to_vec()),
        Some("impact") => Command::Impact(args[1..].to_vec()),
        Some("compare") => Command::Compare(args[1..].to_vec()),
        Some("ir") => Command::Ir(args[1..].to_vec()),
        Some("ir-validate") => Command::IrValidate(args[1..].to_vec()),
        Some("lint") => Command::LintIr(args[1..].to_vec()),
        Some("explain") => Command::Explain(args[1..].to_vec()),
        Some("units") => Command::Units(args[1..].to_vec()),
        Some("update") => Command::Update(args[1..].to_vec()),
        Some("swap") => Command::Swap(args[1..].to_vec()),
        Some("emit") => Command::Emit(args[1..].to_vec()),
        Some("names") => Command::Names(args[1..].to_vec()),
        Some("bind") => Command::Bind(args[1..].to_vec()),
        Some("import") => Command::Import(args[1..].to_vec()),
        Some("sync") => Command::Sync(args[1..].to_vec()),
        Some("lock") => Command::Lock(args[1..].to_vec()),
        Some("lsp") => Command::Lsp(args[1..].to_vec()),
        Some(other) => Command::Unknown(other.to_owned()),
    }
}

/// Run one command line (arguments after the program name), writing
/// command output to `out`; returns the process exit code.
pub fn run(args: &[String], out: &mut dyn std::io::Write) -> u8 {
    run_with_env(args, None, out, &mut std::io::sink())
}

/// [`run`] with the `CADETTA_FORMAT` value given explicitly; usage errors
/// that precede logging are written to `err`.
pub fn run_with_env(
    args: &[String],
    env_format: Option<&str>,
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
) -> u8 {
    let (globals, rest) = match parse_globals(args, env_format) {
        Ok(parsed) => parsed,
        Err(error) => {
            tracing::error!(%error, "invalid global flags");
            let _ = writeln!(err, "error: {error}");
            return EXIT_USAGE;
        }
    };
    let command = parse(&rest);
    tracing::debug!(?command, ?globals, "parsed command line");
    match command {
        Command::Version => match writeln!(out, "{VERSION}") {
            Ok(()) => EXIT_OK,
            Err(error) => {
                tracing::error!(%error, "could not write to stdout");
                EXIT_USAGE
            }
        },
        Command::Cite(rest) => cmd::run_cite(&rest, &globals, out, err),
        Command::Init(rest) => cmd::run_init(&rest, &globals, out, err),
        Command::Fmt(rest) => cmd::run_fmt(&rest, &globals, out, err),
        Command::Solve(rest) => cmd::run_solve(&rest, &globals, out, err),
        Command::Obligations(rest) => cmd::run_obligations(&rest, &globals, out, err),
        Command::Ack(rest) => cmd::run_ack(&rest, &globals, out, err),
        Command::Check(rest) => cmd::run_check(&rest, &globals, out, err),
        Command::Why(rest) => cmd::run_why(&rest, &globals, out, err),
        Command::Impact(rest) => cmd::run_impact(&rest, &globals, out, err),
        Command::Compare(rest) => cmd::run_compare(&rest, &globals, out, err),
        Command::Ir(rest) => cmd::run_ir(&rest, &globals, out, err),
        Command::IrValidate(rest) => cmd::run_ir_validate(&rest, &globals, out, err),
        Command::LintIr(rest) => cmd::run_lint_ir(&rest, &globals, out, err),
        Command::Explain(rest) => cmd::run_explain(&rest, &globals, out, err),
        Command::Units(rest) => cmd::run_units(&rest, &globals, out, err),
        Command::Update(rest) => cmd::run_update(&rest, &globals, out, err),
        Command::Swap(rest) => cmd::run_swap(&rest, &globals, out, err),
        Command::Emit(rest) => cmd::run_emit(&rest, &globals, out, err),
        Command::Names(rest) => cmd::run_names(&rest, &globals, out, err),
        Command::Bind(rest) => cmd::run_bind(&rest, &globals, out, err),
        Command::Import(rest) => cmd::run_import(&rest, &globals, out, err),
        Command::Sync(rest) => cmd::run_sync(&rest, &globals, out, err),
        Command::Lock(rest) => cmd::run_lock(&rest, &globals, out, err),
        Command::Lsp(rest) => cmd::run_lsp(&rest, &globals, out, err),
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
        assert_eq!(run_to_string(&["bogus"]), (EXIT_USAGE, String::new()));
    }

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| (*a).to_owned()).collect()
    }

    // frob:tests crates/cadetta-cli/src/lib.rs::parse_globals
    #[test]
    fn text_is_the_default_format_regardless_of_terminal() {
        // Tests run with stdout captured (not a terminal); the format is still text.
        let (globals, rest) = parse_globals(&strings(&["--version"]), None).unwrap();
        assert_eq!(globals.format, Format::Text);
        assert_eq!(rest, strings(&["--version"]));
    }

    // frob:tests crates/cadetta-cli/src/lib.rs::parse_globals
    #[test]
    fn format_flag_beats_env_and_both_validate() {
        let pf = |a: &[&str], e| parse_globals(&strings(a), e).map(|(g, _)| g.format);
        assert_eq!(pf(&[], Some("json")), Ok(Format::Json));
        assert_eq!(pf(&["--format", "short"], Some("json")), Ok(Format::Short));
        assert_eq!(pf(&["--format=sarif"], None), Ok(Format::Sarif));
        assert!(pf(&["--format", "yaml"], None).is_err());
        assert!(pf(&[], Some("yaml")).is_err());
        assert_eq!(
            pf(&["--format"], None),
            Err(UsageError::MissingValue("--format".into()))
        );
    }

    // frob:tests crates/cadetta-cli/src/lib.rs::parse_globals
    // frob:ticket 01M4VB63RTQENVNZ4DMXBFGJX3
    #[test]
    fn cite_list_keeps_its_own_format_flag() {
        let (g, rest) =
            parse_globals(&strings(&["cite", "list", "--format", "bibtex"]), None).unwrap();
        assert_eq!(g.format, Format::Text);
        assert_eq!(rest, strings(&["cite", "list", "--format", "bibtex"]));
        let (g, rest) =
            parse_globals(&strings(&["--format", "json", "cite", "list"]), None).unwrap();
        assert_eq!(g.format, Format::Json);
        assert_eq!(rest, strings(&["cite", "list"]));
    }

    #[test]
    fn cwd_and_config_are_validated() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let d = dir.to_str().unwrap();
        let (g, rest) =
            parse_globals(&strings(&["--cwd", d, "--config", "Cargo.toml", "x"]), None).unwrap();
        assert_eq!(g.cwd.as_deref(), Some(dir.as_path()));
        assert_eq!(g.config, Some(dir.join("Cargo.toml")));
        assert_eq!(rest, strings(&["x"]));
        assert!(parse_globals(&strings(&["--cwd", "/no/such/dir"]), None).is_err());
        assert!(parse_globals(&strings(&["--config", "/no/such.toml"]), None).is_err());
    }

    // frob:tests crates/cadetta-cli/src/lib.rs::run_with_env
    #[test]
    fn usage_errors_exit_two_with_message() {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_with_env(&strings(&["--format", "yaml"]), None, &mut out, &mut err);
        assert_eq!(code, EXIT_USAGE);
        assert!(
            String::from_utf8(err)
                .unwrap()
                .contains("unknown output format")
        );
        assert!(out.is_empty());
    }
}
