//! frob:ticket 01M4SPWT0RZH7FJYEZ8J7MBN3Z
//! `cadetta init`: write the missing project files (config-cli.md 3.2).

use std::io::Write;
use std::path::Path;

use crate::config::normalise_package_name;
use crate::project::CONFIG_FILE;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// The adapter block `init --adapter` can write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    /// External Windows COM adapter.
    Solidworks,
    /// External FreeCAD adapter.
    Freecad,
    /// Built-in SOLIDWORKS equations emitter.
    SwEquations,
}

impl AdapterKind {
    /// Parse the `--adapter` value.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "solidworks" => Some(Self::Solidworks),
            "freecad" => Some(Self::Freecad),
            "sw-equations" => Some(Self::SwEquations),
            _ => None,
        }
    }

    /// The `[[adapter]]` table text.
    fn block(self) -> &'static str {
        match self {
            Self::Solidworks => {
                "\n[[adapter]]\nname = \"solidworks\"     # external, Windows only\n\
                 command = [\"cadetta-solidworks\"]\nversion = \"0.1\"\n\
                 files = [\"**/*.SLDPRT\", \"**/*.SLDASM\"]\n\
                 save = \"after-sync\"            # after-sync | never\n\
                 native_unit = \"mm\"             # unit for unitless native values\n"
            }
            Self::Freecad => {
                "\n[[adapter]]\nname = \"freecad\"         # external\n\
                 command = [\"cadetta-freecad\"]\nversion = \"0.1\"\n\
                 files = [\"**/*.FCStd\"]\n\
                 save = \"after-sync\"            # after-sync | never\n\
                 native_unit = \"mm\"             # unit for unitless native values\n"
            }
            Self::SwEquations => {
                "\n[[adapter]]\nname = \"sw-equations\"   # built-in\n\
                 out = \"build/solidworks/equations.txt\"\n\
                 bind = \"scope\"               # also bind every in-scope path to its created global\n\
                 native_unit = \"mm\"             # unit for unitless native values\n"
            }
        }
    }

    /// The install line printed after writing the block, if any.
    fn install_line(self) -> Option<&'static str> {
        match self {
            Self::Solidworks => Some(
                "install: pip install \"cadetta[solidworks]\" (run in the Windows interpreter)",
            ),
            Self::Freecad => Some("install: pip install \"cadetta[freecad]\""),
            Self::SwEquations => None,
        }
    }
}

/// Parsed `init` options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InitOptions {
    /// `--adapter KIND`.
    pub adapter: Option<AdapterKind>,
    /// `--bare`: omit the hello-world item.
    pub bare: bool,
}

/// The `cadetta.toml` text for package `name` (every key at its default, commented).
fn config_text(name: &str, adapter: Option<AdapterKind>) -> String {
    let mut text = format!(
        "[package]\n\
         name = \"{name}\"          # lower.dotted (N001)\n\
         version = \"0.1.0\"        # semver\n\
         src = \"src\"              # module root\n\
         # as_of = \"2026-01-01\"   # fixed date for expiry; default today\n\n\
         [lint]\n\
         fail_on = \"error\"        # error | warning | info | never\n\n\
         [emit]\n\
         point = \"nominal\"        # nominal | mid | mmc | lmc\n"
    );
    if let Some(kind) = adapter {
        text.push_str(kind.block());
    }
    text
}

/// The `src/lib.cdt` text; `bare` omits the hello-world item.
fn lib_text(bare: bool) -> &'static str {
    if bare {
        "// Package root module.\n"
    } else {
        "// Package root module.\n\n/// A first component: edit or delete it.\nmodel Hello {\n  size: Length = 10mm\n}\n"
    }
}

/// Create `rel` under `dir` with `text` unless it exists; returns whether it was written.
fn write_missing(dir: &Path, rel: &str, text: &str, out: &mut dyn Write) -> std::io::Result<bool> {
    let path = dir.join(rel);
    if path.exists() {
        tracing::info!(path = %path.display(), "init: file exists, skipped");
        let _ = writeln!(out, "info: skipped {rel} (already exists)");
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, text)?;
    tracing::info!(path = %path.display(), "init: file written");
    let _ = writeln!(out, "created {rel}");
    Ok(true)
}

/// Run `init` in `dir`: write missing files only, echo the package name; returns the exit code.
pub fn init(dir: &Path, options: InitOptions, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let abs = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_owned());
    let dir_name = abs.file_name().map_or_else(
        || "package".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let name = normalise_package_name(&dir_name);
    let _ = writeln!(out, "package name: {name}");
    tracing::info!(%name, dir = %abs.display(), "init");
    let result = (|| {
        let wrote_config =
            write_missing(&abs, CONFIG_FILE, &config_text(&name, options.adapter), out)?;
        write_missing(&abs, "src/lib.cdt", lib_text(options.bare), out)?;
        if wrote_config && let Some(line) = options.adapter.and_then(AdapterKind::install_line) {
            let _ = writeln!(out, "{line}");
        }
        Ok::<(), std::io::Error>(())
    })();
    match result {
        Ok(()) => EXIT_OK,
        Err(error) => {
            tracing::error!(%error, "init failed");
            let _ = writeln!(err, "error: {error}");
            EXIT_USAGE
        }
    }
}

/// Run `cadetta init [--adapter KIND] [--bare]`; `args` are the words after `init`.
pub fn run_init(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let mut options = InitOptions::default();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--bare" {
            options.bare = true;
        } else if arg == "--adapter" || arg.starts_with("--adapter=") {
            let value = match arg.strip_prefix("--adapter=") {
                Some(v) => Some(v),
                None => {
                    i += 1;
                    args.get(i).map(String::as_str)
                }
            };
            match value.and_then(AdapterKind::parse) {
                Some(kind) => options.adapter = Some(kind),
                None => {
                    tracing::error!(?value, "init: bad --adapter");
                    let _ = writeln!(
                        err,
                        "error: --adapter needs solidworks, freecad or sw-equations"
                    );
                    return EXIT_USAGE;
                }
            }
        } else {
            tracing::error!(%arg, "init: unknown argument");
            let _ = writeln!(
                err,
                "error: unknown argument `{arg}`; usage: cadetta init [--adapter solidworks|freecad|sw-equations] [--bare]"
            );
            return EXIT_USAGE;
        }
        i += 1;
    }
    init(
        globals.cwd.as_deref().unwrap_or(Path::new(".")),
        options,
        out,
        err,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    /// A fresh scratch directory named `My-Rover` under the system temp dir.
    fn scratch() -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join(format!(
                "cadetta-init-test-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ))
            .join("My-Rover");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn run(dir: &Path, options: InitOptions) -> (u8, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = init(dir, options, &mut out, &mut err);
        (code, String::from_utf8(out).unwrap())
    }

    // frob:tests crates/cadetta-cli/src/cmd/init.rs::init
    #[test]
    fn repeat_init_overwrites_nothing() {
        let dir = scratch();
        let (code, _) = run(&dir, InitOptions::default());
        assert_eq!(code, EXIT_OK);
        std::fs::write(dir.join("cadetta.toml"), "[package]\nname = \"mine\"\n").unwrap();
        std::fs::write(dir.join("src/lib.cdt"), "// edited\n").unwrap();
        let (code, out) = run(&dir, InitOptions::default());
        assert_eq!(code, EXIT_OK);
        assert_eq!(
            std::fs::read_to_string(dir.join("cadetta.toml")).unwrap(),
            "[package]\nname = \"mine\"\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("src/lib.cdt")).unwrap(),
            "// edited\n"
        );
        assert!(out.contains("info: skipped cadetta.toml"));
        assert!(out.contains("info: skipped src/lib.cdt"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/init.rs::init
    #[test]
    fn creates_missing_files_only_and_echoes_name() {
        let dir = scratch();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/lib.cdt"), "// mine\n").unwrap();
        let (code, out) = run(&dir, InitOptions::default());
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("package name: my_rover"));
        assert!(out.contains("info: skipped src/lib.cdt"));
        assert_eq!(
            std::fs::read_to_string(dir.join("src/lib.cdt")).unwrap(),
            "// mine\n"
        );
        let config = Config::load(&dir.join("cadetta.toml")).unwrap();
        assert_eq!(config.package.unwrap().name.as_deref(), Some("my_rover"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/init.rs::lib_text
    #[test]
    fn bare_omits_hello_world_and_both_forms_format_clean() {
        for bare in [true, false] {
            let dir = scratch();
            run(
                &dir,
                InitOptions {
                    adapter: None,
                    bare,
                },
            );
            let lib = std::fs::read_to_string(dir.join("src/lib.cdt")).unwrap();
            assert_eq!(lib.contains("model Hello"), !bare);
            assert_eq!(cadetta_syntax::format::is_formatted(&lib), Ok(true));
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/init.rs::AdapterKind
    #[test]
    fn adapter_blocks_load_and_solidworks_prints_install_line() {
        for (text, kind) in [
            ("solidworks", AdapterKind::Solidworks),
            ("freecad", AdapterKind::Freecad),
            ("sw-equations", AdapterKind::SwEquations),
        ] {
            assert_eq!(AdapterKind::parse(text), Some(kind));
            let dir = scratch();
            let (_, out) = run(
                &dir,
                InitOptions {
                    adapter: Some(kind),
                    bare: true,
                },
            );
            let config = Config::load(&dir.join("cadetta.toml")).unwrap();
            assert_eq!(config.adapters.len(), 1);
            assert_eq!(config.adapters[0].name, text);
            // frob:tests crates/cadetta-cli/src/config.rs::BindMode
            let want = if kind == AdapterKind::SwEquations {
                crate::config::BindMode::Scope
            } else {
                crate::config::BindMode::Explicit
            };
            assert_eq!(config.adapters[0].bind, want);
            assert_eq!(
                out.contains("pip install \"cadetta[solidworks]\""),
                kind == AdapterKind::Solidworks
            );
        }
        assert_eq!(AdapterKind::parse("nope"), None);
    }
}
