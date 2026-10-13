//! frob:ticket 01M4SPWT6N1HY3MHN544GZJYS9
//! `cadetta ir [--observed ADAPTER] [--assume-unit U]`: print the design or observed IR of a
//! package as `cadetta.ir/1` JSON (ir.md, config-cli.md 3).
//!
//! The source-to-IR lowering and the adapter host are not wired into the CLI yet (the same
//! gap `obligations` and `ack` log), so the IR printed has the package identity and no
//! members; the shape is already the final `cadetta.ir/1` document.

use std::io::Write;

use cadetta_ir::model::{Flavour, PackageId, Project as Ir};
use cadetta_ir::observed::ObservedBuilder;
use cadetta_units::UnitTable;

use crate::config::normalise_package_name;
use crate::project::{PackageDir, Project};
use crate::{EXIT_OK, EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta ir [--observed ADAPTER] [--assume-unit U]";

/// Version used when `[package] version` is absent.
const DEFAULT_VERSION: &str = "0.0.0";

/// The parsed command line of `cadetta ir`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct IrArgs {
    /// Print the observed IR read by this configured adapter instead of the design IR.
    pub observed: Option<String>,
    /// Unit assumed for unitless native values.
    pub assume_unit: Option<String>,
}

/// Parse the words after `ir`; the error is a usage message.
pub fn parse_args(args: &[String]) -> Result<IrArgs, String> {
    let mut parsed = IrArgs::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("`{flag}` needs a value"))
        };
        match arg.as_str() {
            "--observed" => parsed.observed = Some(value("--observed")?),
            "--assume-unit" => parsed.assume_unit = Some(value("--assume-unit")?),
            flag if flag.starts_with("--") => return Err(format!("unknown flag `{flag}`")),
            extra => return Err(format!("unexpected argument `{extra}`")),
        }
    }
    Ok(parsed)
}

/// The IR identity of a package: `[package] name` (else the normalised directory name) and version.
pub fn package_id(pkg: &PackageDir) -> PackageId {
    let package = pkg.config.package.as_ref();
    let name = package.and_then(|p| p.name.clone()).unwrap_or_else(|| {
        normalise_package_name(
            &pkg.dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        )
    });
    let version = package
        .and_then(|p| p.version.clone())
        .unwrap_or_else(|| DEFAULT_VERSION.to_owned());
    PackageId { name, version }
}

/// Run `cadetta ir`; `args` are the words after the verb. Returns the exit code.
pub fn run_ir(args: &[String], globals: &Globals, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "ir failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(message) => return fail(err, &format!("{message}; {USAGE}")),
    };
    if let Some(unit) = &parsed.assume_unit
        && let Err(e) = UnitTable::builtin().parse(unit)
    {
        return fail(err, &format!("--assume-unit {unit}: {e}"));
    }
    let (project, scope) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok(opened) => opened,
        Err(e) => return fail(err, &e.to_string()),
    };
    let [index] = scope.as_slice() else {
        return fail(
            err,
            "the IR is per package; run inside one workspace member or pass --cwd",
        );
    };
    let pkg = &project.packages[*index];
    let id = package_id(pkg);
    let ir = match &parsed.observed {
        None => {
            tracing::warn!(package = %id.name, "ir: no source-to-IR pipeline wired yet; printing an empty design IR");
            Ir::new(Flavour::Design, id)
        }
        Some(name) => {
            let known = pkg
                .config
                .adapters
                .iter()
                .any(|a| a.name == *name || a.id.as_deref() == Some(name));
            if !known {
                return fail(err, &format!("no adapter `{name}` in cadetta.toml"));
            }
            tracing::warn!(adapter = %name, "ir: adapter host not wired yet; printing an empty observed IR");
            match ObservedBuilder::new(id, name.clone(), "") {
                Ok(builder) => builder.build(),
                Err(e) => return fail(err, &e.to_string()),
            }
        }
    };
    let text = match serde_json::to_string_pretty(&ir) {
        Ok(t) => t,
        Err(e) => return fail(err, &format!("could not serialize the IR: {e}")),
    };
    match writeln!(out, "{text}") {
        Ok(()) => EXIT_OK,
        Err(e) => fail(err, &format!("could not write output: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Format;
    use serde_json::Value;

    fn project_dir(adapter: bool) -> tempdir::Dir {
        let dir = tempdir::Dir::new();
        let mut toml = "[package]\nname = \"acme.rover\"\nversion = \"1.2.3\"\n".to_owned();
        if adapter {
            toml.push_str("[[adapter]]\nname = \"freecad\"\n");
        }
        std::fs::write(dir.path().join("cadetta.toml"), toml).unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.cdt"),
            "model A { x: Length = 1mm }\n",
        )
        .unwrap();
        dir
    }

    /// A self-removing scratch directory (no extra dev-dependency).
    mod tempdir {
        pub struct Dir(std::path::PathBuf);
        impl Dir {
            pub fn new() -> Self {
                static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let p = std::env::temp_dir().join(format!("cadetta-ir-{}-{n}", std::process::id()));
                std::fs::create_dir_all(&p).unwrap();
                Self(p)
            }
            pub fn path(&self) -> &std::path::Path {
                &self.0
            }
        }
        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    fn run(dir: &tempdir::Dir, args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let globals = Globals {
            format: Format::Json,
            cwd: Some(dir.path().to_owned()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_ir(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    fn validate(text: &str) -> Value {
        let schema: Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/cadetta.ir.1.json"))
                .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let value: Value = serde_json::from_str(text).unwrap();
        assert!(validator.is_valid(&value), "{value}");
        value
    }

    // frob:tests crates/cadetta-cli/src/cmd/ir.rs::run_ir
    #[test]
    fn prints_design_ir_that_validates() {
        let dir = project_dir(false);
        let (code, out, e) = run(&dir, &[]);
        assert_eq!(e, "");
        assert_eq!(code, EXIT_OK);
        let v = validate(&out);
        assert_eq!(v["schema"], "cadetta.ir/1");
        assert_eq!(v["flavour"], "design");
        assert_eq!(v["package"]["name"], "acme.rover");
        assert_eq!(v["package"]["version"], "1.2.3");
    }

    // frob:tests crates/cadetta-cli/src/cmd/ir.rs::run_ir
    #[test]
    fn observed_ir_for_a_configured_adapter() {
        let dir = project_dir(true);
        let (code, out, _) = run(&dir, &["--observed", "freecad", "--assume-unit", "mm"]);
        assert_eq!(code, EXIT_OK);
        assert_eq!(validate(&out)["flavour"], "observed");
    }

    // frob:tests crates/cadetta-cli/src/cmd/ir.rs::run_ir
    #[test]
    fn usage_errors_exit_two() {
        let dir = project_dir(false);
        assert_eq!(run(&dir, &["--observed", "freecad"]).0, EXIT_USAGE);
        assert_eq!(run(&dir, &["--observed"]).0, EXIT_USAGE);
        assert_eq!(run(&dir, &["--assume-unit", "furlong"]).0, EXIT_USAGE);
        assert_eq!(run(&dir, &["x"]).0, EXIT_USAGE);
    }
}
