//! The `cadetta` binary: a thin shell over [`cadetta::run_with_logging`].

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    ExitCode::from(cadetta::run_with_logging(
        &args,
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    ))
}
