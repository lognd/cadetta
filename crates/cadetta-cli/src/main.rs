//! The `cadetta` binary: a thin shell over [`cadetta::run_with_logging`].
//! frob:ticket 01M4SWN8MPH5ARCRJG4QB9ANVQ

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // No held stdout/stderr locks: `cadetta lsp` writes protocol frames from its own
    // writer thread, which would deadlock against a lock taken here. Each write locks briefly.
    ExitCode::from(cadetta::run_with_logging(
        &args,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    ))
}
