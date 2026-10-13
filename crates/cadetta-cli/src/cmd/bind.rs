// frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
//! `cadetta bind PATH --adapter ID --locator L --kind K [...]` and
//! `cadetta bind --remove PATH --adapter ID --locator L`: add or remove one explicit locator in
//! an adapter's bindings file (config-cli.md 3, adapters.md 3). Validated per adapters.md 3,
//! written in canonical order, idempotent. Rewriting the file drops its comments.

use std::io::Write;

use cadetta_adapter::AdapterError;
use cadetta_adapter::bindings::{BindingSet, kind_text};
use cadetta_adapter::locator::{check_native_collisions, validate};
use cadetta_ir::model::{Locator, LocatorKind, LocatorMode, Sign};
use cadetta_ir::path::{Path, PathContext};
use serde::Serialize;

use crate::cmd::import::find_adapter;
use crate::output::Format;
use crate::project::Project;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta bind PATH --adapter ID --locator L --kind K [--mode M] \
[--native-unit U] [--for ALT] [--config C] [--sign -1] [--component C] [--tolerance drive|ignore] \
| cadetta bind --remove PATH --adapter ID --locator L";

/// The parsed command line of `cadetta bind`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BindArgs {
    /// True for `--remove`.
    pub remove: bool,
    /// The canonical path text.
    pub path: String,
    /// `--adapter`.
    pub adapter: String,
    /// `--locator`.
    pub locator: String,
    /// `--kind`.
    pub kind: Option<String>,
    /// `--mode`.
    pub mode: Option<String>,
    /// `--native-unit`.
    pub native_unit: Option<String>,
    /// `--for`.
    pub for_alternate: Option<String>,
    /// `--config`.
    pub config: Option<String>,
    /// `--sign`.
    pub sign: Option<String>,
    /// `--component`.
    pub component: Option<String>,
    /// `--tolerance`.
    pub tolerance: Option<String>,
}

/// Parse the words after `bind`; a flag takes the next word as its value even if it starts with `-`.
pub fn parse_args(args: &[String]) -> Result<BindArgs, String> {
    let mut a = BindArgs::default();
    let mut path: Option<String> = None;
    let mut adapter = None;
    let mut locator = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("`{flag}` needs a value"))
        };
        match arg.as_str() {
            "--remove" => a.remove = true,
            "--adapter" => adapter = Some(value("--adapter")?),
            "--locator" => locator = Some(value("--locator")?),
            "--kind" => a.kind = Some(value("--kind")?),
            "--mode" => a.mode = Some(value("--mode")?),
            "--native-unit" => a.native_unit = Some(value("--native-unit")?),
            "--for" => a.for_alternate = Some(value("--for")?),
            "--config" => a.config = Some(value("--config")?),
            "--sign" => a.sign = Some(value("--sign")?),
            "--component" => a.component = Some(value("--component")?),
            "--tolerance" => a.tolerance = Some(value("--tolerance")?),
            flag if flag.starts_with("--") => return Err(format!("unknown flag `{flag}`")),
            word => {
                if path.replace(word.to_owned()).is_some() {
                    return Err("exactly one PATH".to_owned());
                }
            }
        }
    }
    a.path = path.ok_or("missing PATH")?;
    a.adapter = adapter.ok_or("missing `--adapter`")?;
    a.locator = locator.ok_or("missing `--locator`")?;
    if !a.remove && a.kind.is_none() {
        return Err("missing `--kind`".to_owned());
    }
    Ok(a)
}

/// The `bind.json` document (config-cli.md 3).
#[derive(Debug, Serialize)]
pub struct BindReport {
    /// `add` or `remove`.
    pub action: &'static str,
    /// Canonical path.
    pub path: String,
    /// Adapter id.
    pub adapter: String,
    /// Native locator text.
    pub locator: String,
    /// The bindings file, relative to the package directory.
    pub file: String,
    /// False when the request was already satisfied and the file was left untouched.
    pub changed: bool,
}

fn parse_kind(text: &str) -> Result<LocatorKind, String> {
    [
        LocatorKind::Global,
        LocatorKind::Dimension,
        LocatorKind::Property,
        LocatorKind::Frame,
        LocatorKind::Instance,
    ]
    .into_iter()
    .find(|k| kind_text(*k) == text)
    .ok_or_else(|| {
        format!("unknown kind `{text}`; expected global, dimension, property, frame or instance")
    })
}

fn build_locator(a: &BindArgs) -> Result<Locator, String> {
    if a.tolerance.is_some() {
        return Err(
            "`--tolerance` is not supported yet: bindings carry no tolerance field".to_owned(),
        );
    }
    let kind = parse_kind(a.kind.as_deref().unwrap_or_default())?;
    let mode = match a.mode.as_deref() {
        Some("value") => LocatorMode::Value,
        Some("ref") => LocatorMode::Ref,
        Some(other) => return Err(format!("unknown mode `{other}`; expected value or ref")),
        None if kind == LocatorKind::Dimension => LocatorMode::Ref,
        None => LocatorMode::Value,
    };
    let sign = match a.sign.as_deref() {
        None => None,
        Some("1") => Some(Sign::Plus),
        Some("-1") => Some(Sign::Minus),
        Some(other) => return Err(format!("sign must be 1 or -1, got `{other}`")),
    };
    Ok(Locator {
        locator: a.locator.clone(),
        kind,
        mode,
        native_unit: a.native_unit.clone().unwrap_or_default(),
        ref_to: None,
        for_alternate: a.for_alternate.clone(),
        config: a.config.clone(),
        sign,
        component: a.component.clone(),
        implicit: None,
    })
}

/// Run `cadetta bind`; `args` are the words after the verb. Returns the exit code.
pub fn run_bind(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "bind failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(m) => return fail(err, &format!("{m}; {USAGE}")),
    };
    let path = match Path::parse(&parsed.path, PathContext::Binding) {
        Ok(p) => p,
        Err(e) => return fail(err, &format!("path `{}`: {e}", parsed.path)),
    };
    let new_locator = if parsed.remove {
        None
    } else {
        match build_locator(&parsed) {
            Ok(l) => match validate(&l) {
                Ok(()) => Some(l),
                Err(e) => return fail(err, &format!("invalid locator: {e}")),
            },
            Err(m) => return fail(err, &m),
        }
    };
    let (project, scope) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok(opened) => opened,
        Err(e) => return fail(err, &e.to_string()),
    };
    let Some((pkg, adapter)) = find_adapter(&project, &scope, &parsed.adapter) else {
        return fail(
            err,
            &format!("no adapter `{}` in cadetta.toml", parsed.adapter),
        );
    };
    let rel = adapter.bindings_path();
    let file = pkg.dir.join(&rel);
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let mut set = match BindingSet::load_str(adapter.id(), &text) {
        Ok(set) => set,
        Err(e) => return fail(err, &e.to_string()),
    };
    let changed = match new_locator {
        Some(loc) => match set.bind(&path, loc) {
            Ok(c) => c,
            Err(e) => return fail(err, &e.to_string()),
        },
        None => set.unbind(&path, &parsed.locator),
    };
    if changed {
        if !parsed.remove
            && let Err(e) = check_native_collisions(&set.file_entries())
        {
            let message = match e {
                AdapterError::Failed(ds) => ds
                    .iter()
                    .map(|d| format!("{}: {}", d.code, d.message))
                    .collect::<Vec<_>>()
                    .join("; "),
                other => format!("{other:?}"),
            };
            return fail(err, &message);
        }
        let write = file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&file, set.to_toml()));
        if let Err(e) = write {
            return fail(err, &format!("could not write {}: {e}", file.display()));
        }
        tracing::info!(file = %file.display(), "bind: bindings file written");
    } else {
        tracing::info!(file = %file.display(), "bind: nothing to change");
    }
    let report = BindReport {
        action: if parsed.remove { "remove" } else { "add" },
        path: path.to_string(),
        adapter: adapter.id().to_owned(),
        locator: parsed.locator.clone(),
        file: rel,
        changed,
    };
    let text = match globals.format {
        Format::Json => serde_json::to_string_pretty(&report).unwrap_or_default(),
        _ => format!(
            "{} {} {} ({}){}",
            if parsed.remove { "unbound" } else { "bound" },
            report.path,
            report.locator,
            report.file,
            if changed { "" } else { ": no change" }
        ),
    };
    let _ = writeln!(out, "{text}");
    EXIT_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A self-removing scratch directory (no extra dev-dependency).
    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("cadetta-bind-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn bindings(&self) -> String {
            std::fs::read_to_string(self.0.join("bindings/equations.toml")).unwrap_or_default()
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn project() -> Dir {
        let dir = Dir::new();
        std::fs::write(
            dir.0.join("cadetta.toml"),
            "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"sw-equations\"\nid = \"equations\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.0.join("src")).unwrap();
        std::fs::write(
            dir.0.join("src/lib.cdt"),
            "model CoverPlate {\n  thickness: Length = 6mm\n  offset: Length = 2mm\n}\n",
        )
        .unwrap();
        dir
    }

    fn run(dir: &Dir, format: Format, args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let globals = Globals {
            format,
            cwd: Some(dir.0.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_bind(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    const P: &str = "acme.plate::CoverPlate.thickness";

    fn add<'a>(extra: &[&'a str]) -> Vec<&'a str> {
        let mut v = vec![
            P,
            "--adapter",
            "equations",
            "--locator",
            "Plate_Thk",
            "--kind",
            "global",
        ];
        v.extend_from_slice(extra);
        v
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/bind.rs::run_bind
    #[test]
    fn add_is_canonical_and_idempotent() {
        let dir = project();
        let (code, out, e) = run(&dir, Format::Text, &add(&["--native-unit", "mm"]));
        assert_eq!((code, e.as_str()), (EXIT_OK, ""), "{out}");
        let first = dir.bindings();
        assert!(
            first.contains("Plate_Thk") && first.contains("native_unit = \"mm\""),
            "{first}"
        );
        let (code, out, _) = run(&dir, Format::Json, &add(&["--native-unit", "mm"]));
        assert_eq!(code, EXIT_OK);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["changed"], false);
        assert_eq!(dir.bindings(), first);
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/bind.rs::run_bind
    #[test]
    fn remove_drops_the_locator() {
        let dir = project();
        run(&dir, Format::Text, &add(&["--sign", "-1"]));
        assert!(dir.bindings().contains("sign = -1"), "{}", dir.bindings());
        let rm = [
            P,
            "--remove",
            "--adapter",
            "equations",
            "--locator",
            "Plate_Thk",
        ];
        let (code, out, _) = run(&dir, Format::Json, &rm);
        assert_eq!(code, EXIT_OK, "{out}");
        assert!(!dir.bindings().contains("Plate_Thk"));
        let (_, out, _) = run(&dir, Format::Json, &rm);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            (v["action"].as_str(), v["changed"].as_bool()),
            (Some("remove"), Some(false))
        );
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/bind.rs::run_bind
    #[test]
    fn rejects_conflicts_and_invalid_requests() {
        let dir = project();
        run(&dir, Format::Text, &add(&["--native-unit", "mm"]));
        let before = dir.bindings();
        assert_eq!(
            run(&dir, Format::Text, &add(&["--native-unit", "in"])).0,
            EXIT_USAGE
        );
        assert_eq!(dir.bindings(), before);
        let bad_kind = [
            P,
            "--adapter",
            "equations",
            "--locator",
            "X",
            "--kind",
            "bogus",
        ];
        assert_eq!(run(&dir, Format::Text, &bad_kind).0, EXIT_USAGE);
        let no_adapter = [P, "--adapter", "nope", "--locator", "X", "--kind", "global"];
        assert_eq!(run(&dir, Format::Text, &no_adapter).0, EXIT_USAGE);
        assert_eq!(
            run(&dir, Format::Text, &add(&["--tolerance", "drive"])).0,
            EXIT_USAGE
        );
        assert_eq!(
            run(&dir, Format::Text, &add(&["--sign", "2"])).0,
            EXIT_USAGE
        );
        assert_eq!(
            run(&dir, Format::Text, &["--adapter", "equations"]).0,
            EXIT_USAGE
        );
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/bind.rs::run_bind
    #[test]
    fn refuses_native_name_collisions() {
        let dir = project();
        run(&dir, Format::Text, &add(&[]));
        let other = [
            "acme.plate::CoverPlate.offset",
            "--adapter",
            "equations",
            "--locator",
            "Plate_Thk",
            "--kind",
            "global",
        ];
        let (code, _, e) = run(&dir, Format::Text, &other);
        assert_eq!(code, EXIT_USAGE, "{e}");
        assert!(!dir.bindings().contains("offset"));
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/bind.rs::parse_args
    #[test]
    fn flag_values_may_start_with_a_dash() {
        let args: Vec<String> = [
            P,
            "--adapter",
            "a",
            "--locator",
            "L",
            "--kind",
            "global",
            "--sign",
            "-1",
        ]
        .map(String::from)
        .to_vec();
        let p = parse_args(&args).unwrap();
        assert_eq!(p.sign.as_deref(), Some("-1"));
        assert!(parse_args(&args[..2]).is_err());
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/bind.rs::run_bind
    #[test]
    fn json_validates_against_bind_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/bind.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let dir = project();
        for _ in 0..2 {
            let (_, out, _) = run(&dir, Format::Json, &add(&[]));
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert!(validator.is_valid(&v), "{v}");
        }
    }
}
