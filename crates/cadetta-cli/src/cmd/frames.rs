// frob:ticket 01M54B7EJ996S1WG6ERSPT082Y
//! `cadetta frames [--native-unit U] [--precision N] [--module PATH] [GLOB...]`: print the composed
//! datum frames and instance (placement anchor) frames of the evaluated design as 3x4 transforms
//! in the native length unit (geometry.md 2 and 4, adapters.md 3.4).

use std::io::Write;

use cadetta_adapter::builtin::sw_equations::DEFAULT_PRECISION;
use cadetta_adapter::geometry_bind::Composer;
use cadetta_ir::model::{Flavour, Project as Ir};
use cadetta_units::UnitTable;
use serde::Serialize;

use crate::cmd::emit::DEFAULT_NATIVE_UNIT;
use crate::cmd::ir::package_id;
use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::Project;
use crate::scope::Scope as RunScope;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

const USAGE: &str =
    "usage: cadetta frames [--native-unit U] [--precision N] [--module PATH] [GLOB...]";

/// Schema tag of the JSON output (`docs/schemas/frames.json`).
pub const SCHEMA: &str = "cadetta.frames/1";

/// One composed frame.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrameRow {
    /// `datum` (frame of a datum in its own component) or `instance` (frame a placement drives to).
    pub kind: &'static str,
    /// Canonical path of the datum or placement.
    pub path: String,
    /// Three rows of rotation columns then the translation in the native unit; null when the
    /// chain could not be composed (missing evaluated value, ambiguous anchor, too deep).
    pub transform: Option<[[f64; 4]; 3]>,
}

/// The `cadetta.frames/1` document.
#[derive(Debug, Serialize)]
pub struct FramesReport {
    /// Always [`SCHEMA`].
    pub schema: &'static str,
    /// The native length unit translations are expressed in.
    pub native_unit: String,
    /// Digits after the decimal point kept in transforms.
    pub precision: u32,
    /// Frames sorted by kind then path.
    pub frames: Vec<FrameRow>,
}

/// The parsed command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FramesArgs {
    /// `--native-unit`, default `mm`.
    pub native_unit: Option<String>,
    /// `--precision`, default the sw-equations precision.
    pub precision: Option<u32>,
    /// `--module` prefixes (repeatable).
    pub modules: Vec<String>,
    /// Canonical-path globs.
    pub globs: Vec<String>,
}

/// Parse the words after `frames`.
pub fn parse_args(args: &[String]) -> Result<FramesArgs, String> {
    let mut parsed = FramesArgs::default();
    let mut i = 0;
    let value = |flag: &str, i: &mut usize| {
        *i += 1;
        args.get(*i)
            .cloned()
            .ok_or_else(|| format!("`{flag}` needs a value"))
    };
    while let Some(arg) = args.get(i) {
        match arg.as_str() {
            "--module" => parsed.modules.push(value("--module", &mut i)?),
            "--native-unit" => parsed.native_unit = Some(value("--native-unit", &mut i)?),
            "--precision" => {
                let v = value("--precision", &mut i)?;
                parsed.precision = Some(
                    v.parse()
                        .map_err(|_| format!("`--precision` needs a number, got `{v}`"))?,
                );
            }
            flag if flag.starts_with("--") => return Err(format!("unknown flag `{flag}`")),
            glob => parsed.globs.push(glob.to_owned()),
        }
        i += 1;
    }
    Ok(parsed)
}

/// The SI length of one native unit, or `None` when `unit` is unknown.
fn scale_of(unit: &str) -> Option<f64> {
    UnitTable::builtin().lookup(unit).map(|u| u.factor.to_f64())
}

/// Compose every in-scope datum and placement of `ir` into frame rows.
// frob:ticket 01M54B7EJ996S1WG6ERSPT082Y
pub fn frames_of(ir: &Ir, scale: f64, precision: u32, scope: &RunScope) -> Vec<FrameRow> {
    use cadetta_adapter::geometry_bind::Mat34;
    let composer = Composer::new(&ir.components, scale);
    let rounded = |m: Mat34| {
        Mat34::parse_literal(&m.literal(precision))
            .ok()
            .map(|m| m.0)
    };
    let mut rows = Vec::new();
    for c in &ir.components {
        for d in c
            .datums
            .iter()
            .filter(|d| scope.contains(&d.path.to_string()))
        {
            let transform = composer.datum_frame(&d.path).and_then(rounded);
            if transform.is_none() {
                tracing::warn!(path = %d.path, "frames: datum frame not composable");
            }
            rows.push(FrameRow {
                kind: "datum",
                path: d.path.to_string(),
                transform,
            });
        }
        for p in c
            .placements
            .iter()
            .filter(|p| scope.contains(&p.path.to_string()))
        {
            let transform = composer.anchor_frame(c, p).and_then(rounded);
            if transform.is_none() {
                tracing::warn!(path = %p.path, "frames: instance frame not composable");
            }
            rows.push(FrameRow {
                kind: "instance",
                path: p.path.to_string(),
                transform,
            });
        }
    }
    rows.sort_by(|a, b| (a.kind, &a.path).cmp(&(b.kind, &b.path)));
    tracing::debug!(rows = rows.len(), "frames: composed");
    rows
}

fn render_text(report: &FramesReport) -> String {
    let mut s = String::new();
    for f in &report.frames {
        let t = match &f.transform {
            Some(m) => m
                .iter()
                .map(|r| {
                    format!(
                        "[{}]",
                        r.iter()
                            .map(|x| format!("{x:.p$}", p = report.precision as usize))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join(" "),
            None => "-".to_owned(),
        };
        s.push_str(&format!("{}  {}  {t}\n", f.kind, f.path));
    }
    if s.is_empty() {
        s.push_str("no frames\n");
    }
    s
}

/// Run `cadetta frames`; `args` are the words after the verb. Returns the exit code.
pub fn run_frames(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "frames failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(message) => return fail(err, &format!("{message}; {USAGE}")),
    };
    let unit = parsed
        .native_unit
        .clone()
        .unwrap_or_else(|| DEFAULT_NATIVE_UNIT.to_owned());
    let Some(scale) = scale_of(&unit) else {
        return fail(err, &format!("unknown native unit `{unit}`; {USAGE}"));
    };
    let precision = parsed.precision.unwrap_or(DEFAULT_PRECISION);
    let run_scope = match RunScope::new(&parsed.modules, &parsed.globs) {
        Ok(s) => s,
        Err(e) => return fail(err, &format!("{e}; {USAGE}")),
    };
    let (project, scope) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok(opened) => opened,
        Err(e) => return fail(err, &e.to_string()),
    };
    let analysis = analyze(&project, &scope);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    let mut frames = Vec::new();
    for index in &scope {
        let id = package_id(&project.packages[*index]);
        let ir = analysis
            .packages
            .iter()
            .find(|p| p.name == id.name)
            .map_or_else(|| Ir::new(Flavour::Design, id.clone()), |p| p.ir.clone());
        frames.extend(frames_of(&ir, scale, precision, &run_scope));
    }
    frames.sort_by(|a, b| (a.kind, &a.path).cmp(&(b.kind, &b.path)));
    tracing::info!(n = frames.len(), %unit, "frames: reported");
    let report = FramesReport {
        schema: SCHEMA,
        native_unit: unit,
        precision,
        frames,
    };
    let rendered = match globals.format {
        Format::Json => serde_json::to_string_pretty(&report).map_err(|e| e.to_string()),
        _ => Ok(render_text(&report)),
    };
    match rendered {
        Ok(text) => match writeln!(out, "{}", text.trim_end()) {
            Ok(()) => EXIT_OK,
            Err(e) => fail(err, &format!("could not write output: {e}")),
        },
        Err(e) => fail(err, &format!("could not render the report: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A self-removing scratch directory.
    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("cadetta-frames-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const SOURCE: &str = "model Plate {\n  height = 10mm\n  datum top = origin.offset(z = height)\n}\n\nmodel Blank {\n  datum base = origin\n}\n\nassembly Asm {\n  part plate: Plate\n  part blank: Blank\n  place plate.origin at origin\n  place blank.base at plate.top.offset(x = 5mm)\n}\n";

    fn project() -> Dir {
        let dir = Dir::new();
        std::fs::write(
            dir.0.join("cadetta.toml"),
            "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.0.join("src")).unwrap();
        std::fs::write(dir.0.join("src/lib.cdt"), SOURCE).unwrap();
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
        let code = run_frames(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    // frob:tests crates/cadetta-cli/src/cmd/frames.rs::run_frames
    // frob:tests crates/cadetta-cli/src/cmd/frames.rs::frames_of
    #[test]
    fn reports_datum_and_instance_frames_in_native_unit() {
        let dir = project();
        let (code, out, err) = run(&dir, Format::Json, &[]);
        assert_eq!(code, EXIT_OK, "{out} {err}");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["schema"], SCHEMA);
        let frames = v["frames"].as_array().unwrap();
        let datum = frames
            .iter()
            .find(|f| f["path"] == "acme.plate::Plate.top")
            .unwrap();
        assert_eq!(datum["transform"][2][3], 10.0, "{out}");
        let inst = frames
            .iter()
            .find(|f| f["path"] == "acme.plate::Asm/blank.place")
            .unwrap();
        assert_eq!(inst["transform"][0][3], 5.0, "{out}");
        assert_eq!(inst["transform"][2][3], 10.0, "{out}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/frames.rs::run_frames
    #[test]
    fn native_unit_scales_translations_and_globs_filter() {
        let dir = project();
        let (_, out, _) = run(
            &dir,
            Format::Json,
            &["--native-unit", "m", "acme.plate::Plate.*"],
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let frames = v["frames"].as_array().unwrap();
        assert!(frames.iter().all(|f| f["kind"] == "datum"), "{out}");
        assert_eq!(frames[0]["transform"][2][3], 0.01, "{out}");
        let (code, text, _) = run(&dir, Format::Text, &[]);
        assert_eq!(code, EXIT_OK);
        assert!(text.contains("datum  acme.plate::Plate.top"), "{text}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/frames.rs::parse_args
    // frob:tests crates/cadetta-cli/src/cmd/frames.rs::run_frames
    #[test]
    fn usage_errors_exit_2() {
        let dir = project();
        for args in [
            &["--bogus"][..],
            &["--native-unit"],
            &["--native-unit", "parsecs"],
            &["--precision", "x"],
        ] {
            assert_eq!(run(&dir, Format::Text, args).0, EXIT_USAGE, "{args:?}");
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/frames.rs::run_frames
    #[test]
    fn json_output_validates_against_the_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/frames.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let dir = project();
        let (_, out, _) = run(&dir, Format::Json, &[]);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(validator.is_valid(&v), "{out}");
    }
}
