//! The `cadetta` binary: logging setup around [`cadetta::run`].

use std::process::ExitCode;

use tracing_subscriber::EnvFilter;

/// Route all logs to stderr so stdout stays for command output (JSON with `--format json`).
fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("CADETTA_LOG"))
        .with_writer(std::io::stderr)
        .init();
}

fn main() -> ExitCode {
    init_logging();
    let args: Vec<String> = std::env::args().skip(1).collect();
    ExitCode::from(cadetta::run(&args, &mut std::io::stdout().lock()))
}
