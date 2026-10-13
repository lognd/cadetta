//! frob:ticket 01M4SWN8DY2SQYR5GCTQQM16Z9
//! Command implementations (config-cli.md 3), one module per verb family.

pub mod cite;

use std::io::Write;

use crate::project::Project;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Run `cite pin NAME`; `args` are the words after `cite`. Returns the exit code.
pub fn run_cite(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let [verb, name] = args else {
        tracing::error!("usage: cadetta cite pin NAME");
        let _ = writeln!(err, "error: usage: cadetta cite pin NAME");
        return EXIT_USAGE;
    };
    if verb != "pin" {
        tracing::error!(%verb, "unknown cite verb");
        let _ = writeln!(
            err,
            "error: unknown cite verb `{verb}`; usage: cadetta cite pin NAME"
        );
        return EXIT_USAGE;
    }
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
    let result = Project::open(globals.cwd.as_deref(), globals.config.as_deref())
        .map_err(|e| e.to_string())
        .and_then(|(project, scope)| {
            cite::pin(&project, &scope, name, &io).map_err(|e| e.to_string())
        });
    match result {
        Ok(pinned) => {
            let _ = writeln!(
                out,
                "pinned {name}: {} -> {} (blake3:{})",
                pinned.url, pinned.local, pinned.digest
            );
            EXIT_OK
        }
        Err(message) => {
            tracing::error!(%message, "cite pin failed");
            let _ = writeln!(err, "error: {message}");
            EXIT_USAGE
        }
    }
}
