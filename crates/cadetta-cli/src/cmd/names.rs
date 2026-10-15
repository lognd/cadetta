// frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
//! `cadetta names [ADAPTER-ID] [--module PATH] [GLOB...]`: list every in-scope bindable path with
//! its kind, emitted value, citation, doc, the name the adapter would create and its binding
//! state (config-cli.md 3, adapters.md 3, 3.1 and 3.5). Names never launches CAD: with no read
//! adapter available the state of an unbound path is `unknown`, not `unbound`.

use std::collections::HashMap;
use std::io::Write;

use cadetta_adapter::bindings::{
    BindingSet, R013, implicit_text, kind_text, mangle, mangle_all, mode_text,
};
use cadetta_adapter::builtin::sw_equations::DEFAULT_PRECISION;
use cadetta_adapter::policy::resolve_point;
use cadetta_adapter::scope::Scope as AdapterScope;
use cadetta_ir::model::{Flavour, Locator, LocatorKind, Member, MemberKind, Project as Ir};
use cadetta_ir::path::Path;
use cadetta_units::UnitTable;
use serde::Serialize;

use crate::cmd::emit::{
    DEFAULT_NATIVE_UNIT, EmitDiag, attr_point, citation_text, config_point, diag, kind_of,
    locators_for, parse_args, scope_candidates, written_for,
};
use crate::cmd::ir::package_id;
use crate::config::{Adapter, BindMode};
use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::{PackageDir, Project};
use crate::scope::Scope as RunScope;
use crate::{EXIT_FAIL, EXIT_OK, EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta names [ADAPTER-ID] [--module PATH] [GLOB...]";

/// The binding state of one path (config-cli.md 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// An explicit locator in the bindings file.
    Explicit,
    /// Bound by scope or by discovery, never written to the file.
    Implicit,
    /// The adapter read CAD and found nothing for the path.
    Unbound,
    /// The adapter could not read CAD on this machine.
    Unknown,
}

/// One locator of a listed path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NameLocator {
    /// The native locator text.
    pub locator: String,
    /// Locator kind: global, dimension, property, frame or instance.
    pub kind: String,
    /// Locator mode: value or ref.
    pub mode: String,
    /// Why the binding is implicit: global, link, frame or reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit: Option<String>,
}

/// One listed path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NameRow {
    /// Canonical path.
    pub path: String,
    /// Member kind: param, datum or place.
    pub kind: String,
    /// The value emit would write at the resolved emit point; null when none is writable.
    pub emit: Option<String>,
    /// The authored value text when the value carries a tolerance, else null.
    pub tolerance: Option<String>,
    /// Citation, as emit prints it.
    pub citation: Option<String>,
    /// Doc comment text.
    pub doc: Option<String>,
    /// The name the adapter creates for the path.
    pub created_name: String,
    /// explicit, implicit, unbound or unknown.
    pub state: State,
    /// For an implicit state, why: global, link, frame or reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Every locator known for the path, explicit first.
    pub locators: Vec<NameLocator>,
}

/// The names of one adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdapterNames {
    /// Adapter id.
    pub id: String,
    /// The bindings file, relative to the package directory.
    pub bindings: String,
    /// Rows sorted by canonical path.
    pub names: Vec<NameRow>,
}

/// The `names.json` document (config-cli.md 3).
#[derive(Debug, Default, Serialize)]
pub struct NamesReport {
    /// One entry per selected adapter.
    pub adapters: Vec<AdapterNames>,
    /// Diagnostics (R013 name collision, Q004 unwritable value).
    pub diagnostics: Vec<EmitDiag>,
    /// An invalid bindings file: stops the run with exit 2 (adapters.md 3); not reported.
    #[serde(skip)]
    pub config_error: Option<String>,
}

/// State of a path from its locators; `can_read` says whether an adapter could read CAD.
pub fn state_of(locators: &[Locator], can_read: bool) -> (State, Option<&'static str>) {
    if locators.iter().any(|l| l.implicit.is_none()) {
        return (State::Explicit, None);
    }
    if let Some(why) = locators.iter().find_map(|l| l.implicit) {
        return (State::Implicit, Some(implicit_text(why)));
    }
    if can_read {
        (State::Unbound, None)
    } else {
        (State::Unknown, None)
    }
}

fn member_kind_text(kind: MemberKind) -> &'static str {
    match kind {
        MemberKind::Param => "param",
        MemberKind::Datum => "datum",
        MemberKind::Place => "place",
        MemberKind::Let => "let",
        MemberKind::Bound => "bound",
    }
}

fn row(
    member: &Member,
    created: &str,
    locators: &[Locator],
    adapter: &Adapter,
    cfg_point: cadetta_adapter::policy::EmitPoint,
    diagnostics: &mut Vec<EmitDiag>,
) -> Option<NameRow> {
    let value = member.value.as_ref()?;
    let (state, reason) = state_of(locators, false);
    let unit = locators
        .iter()
        .map(|l| l.native_unit.as_str())
        .find(|u| !u.is_empty())
        .map(str::to_owned)
        .or_else(|| adapter.native_unit.clone())
        .unwrap_or_else(|| DEFAULT_NATIVE_UNIT.to_owned());
    let point = resolve_point(&[attr_point(member)], cfg_point);
    let precision = adapter.precision.map_or(DEFAULT_PRECISION, u32::from);
    let emit = match written_for(
        &UnitTable::builtin(),
        value,
        member,
        &unit,
        kind_of(value),
        point,
        precision,
    ) {
        Ok(w) => w,
        Err((code, message)) => {
            tracing::warn!(path = %member.path, code, %message, "names: value not writable");
            diagnostics.push(diag(code, message, Some(&member.path)));
            None
        }
    };
    let mut ordered: Vec<&Locator> = locators.iter().collect();
    ordered.sort_by_key(|l| l.implicit.is_some());
    Some(NameRow {
        path: member.path.to_string(),
        kind: member_kind_text(member.kind).to_owned(),
        emit,
        tolerance: value.tol.is_some().then(|| value.authored.clone()),
        citation: citation_text(member),
        doc: member.doc.clone(),
        created_name: created.to_owned(),
        state,
        reason: reason.map(str::to_owned),
        locators: ordered
            .into_iter()
            .map(|l| NameLocator {
                locator: l.locator.clone(),
                kind: kind_text(l.kind).to_owned(),
                mode: mode_text(l.mode).to_owned(),
                implicit: l.implicit.map(|i| implicit_text(i).to_owned()),
            })
            .collect(),
    })
}

/// Names of one adapter in one package, appended to `report`.
fn adapter_names(
    pkg: &PackageDir,
    ir: &Ir,
    adapter: &Adapter,
    run_scope: &RunScope,
    report: &mut NamesReport,
) {
    let bindings = adapter.bindings_path();
    let bindings_file = pkg.dir.join(&bindings);
    let mut set = match std::fs::read_to_string(&bindings_file) {
        Ok(text) => match BindingSet::load_str(adapter.id(), &text) {
            Ok(set) => set,
            Err(e) => {
                tracing::error!(file = %bindings_file.display(), error = %e, "names: invalid bindings");
                report.config_error = Some(format!("{}: {e}", bindings_file.display()));
                return;
            }
        },
        Err(_) => BindingSet::default(),
    };
    let scope = AdapterScope::new(adapter.modules.clone(), adapter.paths.clone());
    let bindable: Vec<&Member> = ir
        .components
        .iter()
        .flat_map(|c| &c.members)
        .filter(|m| {
            !m.analysis
                && m.value.is_some()
                && matches!(
                    m.kind,
                    MemberKind::Param | MemberKind::Datum | MemberKind::Place
                )
                && scope.accepts(&m.path.to_string())
        })
        .collect();
    // A path bound to an explicit global creates no name (adapters.md 3.1).
    let creating: Vec<Path> = bindable
        .iter()
        .filter(|m| {
            locators_for(&set, &m.path).is_none_or(|ls| {
                !ls.iter()
                    .any(|l| l.kind == LocatorKind::Global && l.implicit.is_none())
            })
        })
        .map(|m| m.path.clone())
        .collect();
    let collided = match mangle_all(&creating, &adapter.prefix) {
        Ok(_) => false,
        Err(e) => {
            tracing::warn!(adapter = %adapter.id(), error = %e, "names: native name collision (R013)");
            report.diagnostics.push(diag(R013, e.to_string(), None));
            true
        }
    };
    if adapter.bind == BindMode::Scope && !collided {
        let package = pkg.config.package.as_ref().and_then(|p| p.name.as_deref());
        let candidates = scope_candidates(ir, &scope);
        if let Err(e) = set.bind_scope(
            &candidates,
            package.unwrap_or_default(),
            &adapter.prefix,
            false,
        ) {
            report.diagnostics.push(diag(R013, e.to_string(), None));
        }
    }
    let cfg_point = config_point(pkg.config.emit.point);
    let mut names: Vec<NameRow> = bindable
        .into_iter()
        .filter(|m| run_scope.contains(&m.path.to_string()))
        .filter_map(|m| {
            let locators = locators_for(&set, &m.path).unwrap_or_default();
            let created = mangle(&m.path, &adapter.prefix);
            row(
                m,
                &created,
                locators,
                adapter,
                cfg_point,
                &mut report.diagnostics,
            )
        })
        .collect();
    names.sort_by(|a, b| a.path.cmp(&b.path));
    tracing::info!(adapter = %adapter.id(), n = names.len(), "names: listed");
    report.adapters.push(AdapterNames {
        id: adapter.id().to_owned(),
        bindings,
        names,
    });
}

fn render_text(report: &NamesReport) -> String {
    let mut s = String::new();
    for a in &report.adapters {
        s.push_str(&format!("{} ({})\n", a.id, a.bindings));
        for n in &a.names {
            let state = match &n.reason {
                Some(r) => format!("implicit ({r})"),
                None => format!("{:?}", n.state).to_lowercase(),
            };
            s.push_str(&format!(
                "  {}  {}  {}  emit={}  name={}\n",
                n.path,
                n.kind,
                state,
                n.emit.as_deref().unwrap_or("-"),
                n.created_name
            ));
        }
    }
    for d in &report.diagnostics {
        match &d.path {
            Some(p) => s.push_str(&format!("{}: {} [{p}]\n", d.code, d.message)),
            None => s.push_str(&format!("{}: {}\n", d.code, d.message)),
        }
    }
    if s.is_empty() {
        s.push_str("no adapters configured\n");
    }
    s
}

/// Run `cadetta names`; `args` are the words after the verb. Returns the exit code.
pub fn run_names(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "names failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let parsed = match parse_args(args) {
        Ok(p) if p.adapters.len() <= 1 => p,
        Ok(_) => return fail(err, &format!("at most one adapter id; {USAGE}")),
        Err(message) => return fail(err, &format!("{message}; {USAGE}")),
    };
    let run_scope = match RunScope::new(&parsed.modules, &parsed.globs) {
        Ok(s) => s,
        Err(e) => return fail(err, &format!("{e}; {USAGE}")),
    };
    let (project, scope) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok(opened) => opened,
        Err(e) => return fail(err, &e.to_string()),
    };
    if let Some(id) = parsed.adapters.first() {
        let known = scope.iter().any(|i| {
            project.packages[*i]
                .config
                .adapters
                .iter()
                .any(|a| a.id() == id)
        });
        if !known {
            return fail(err, &format!("no adapter `{id}` in cadetta.toml"));
        }
    }
    let analysis = analyze(&project, &scope);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    let by_name: HashMap<_, _> = analysis
        .packages
        .iter()
        .map(|p| (p.name.clone(), p))
        .collect();
    let mut report = NamesReport::default();
    for index in &scope {
        let pkg = &project.packages[*index];
        let id = package_id(pkg);
        let ir = by_name
            .get(&id.name)
            .map_or_else(|| Ir::new(Flavour::Design, id.clone()), |p| p.ir.clone());
        for adapter in &pkg.config.adapters {
            if parsed.adapters.first().is_some_and(|a| a != adapter.id()) {
                continue;
            }
            adapter_names(pkg, &ir, adapter, &run_scope, &mut report);
            if let Some(message) = report.config_error.take() {
                return fail(err, &message);
            }
        }
    }
    let failed = report
        .diagnostics
        .iter()
        .any(|d| d.code == "Q004" || d.code == R013);
    let rendered = match globals.format {
        Format::Json => serde_json::to_string_pretty(&report).map_err(|e| e.to_string()),
        _ => Ok(render_text(&report)),
    };
    match rendered {
        Ok(text) => {
            let _ = writeln!(out, "{}", text.trim_end());
        }
        Err(e) => return fail(err, &format!("could not render the report: {e}")),
    }
    if failed { EXIT_FAIL } else { EXIT_OK }
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
            let p = std::env::temp_dir().join(format!("cadetta-names-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn project(source: &str, bindings: &str, adapter_extra: &str) -> Dir {
        let dir = Dir::new();
        let toml = format!(
            "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"sw-equations\"\nid = \"equations\"\n{adapter_extra}"
        );
        std::fs::write(dir.0.join("cadetta.toml"), toml).unwrap();
        std::fs::create_dir_all(dir.0.join("src")).unwrap();
        std::fs::write(dir.0.join("src/lib.cdt"), source).unwrap();
        std::fs::create_dir_all(dir.0.join("bindings")).unwrap();
        std::fs::write(dir.0.join("bindings/equations.toml"), bindings).unwrap();
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
        let code = run_names(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    const SOURCE: &str =
        "model CoverPlate {\n  thickness: Length = 6mm\n  offset: Length = 2mm\n}\n";
    const BINDINGS: &str = "[\"acme.plate::CoverPlate.thickness\"]\nlocators = [ { locator = \"Plate_Thk\", kind = \"global\", native_unit = \"mm\" } ]\n";

    fn json(dir: &Dir, args: &[&str]) -> serde_json::Value {
        let (_, out, _) = run(dir, Format::Json, args);
        serde_json::from_str(&out).unwrap()
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/names.rs::run_names
    #[test]
    fn lists_sorted_paths_with_explicit_and_unknown_states() {
        let dir = project(SOURCE, BINDINGS, "");
        let v = json(&dir, &[]);
        let names = &v["adapters"][0]["names"];
        assert_eq!(names[0]["path"], "acme.plate::CoverPlate.offset");
        assert_eq!(names[0]["state"], "unknown");
        assert_eq!(names[1]["path"], "acme.plate::CoverPlate.thickness");
        assert_eq!(names[1]["state"], "explicit");
        assert_eq!(names[1]["locators"][0]["locator"], "Plate_Thk");
        assert_eq!(names[1]["emit"], "6mm");
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/names.rs::run_names
    #[test]
    fn scope_binding_is_implicit_global() {
        let dir = project(SOURCE, BINDINGS, "bind = \"scope\"\n");
        let v = json(&dir, &["equations"]);
        let off = &v["adapters"][0]["names"][0];
        assert_eq!(off["state"], "implicit");
        assert_eq!(off["reason"], "global");
        assert_eq!(off["locators"][0]["implicit"], "global");
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/names.rs::state_of
    #[test]
    fn unbound_only_when_the_adapter_could_read() {
        assert_eq!(state_of(&[], true), (State::Unbound, None));
        assert_eq!(state_of(&[], false), (State::Unknown, None));
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/names.rs::run_names
    #[test]
    fn text_output_and_glob_filter() {
        let dir = project(SOURCE, BINDINGS, "");
        let (code, out, _) = run(&dir, Format::Text, &["acme.plate::CoverPlate.thick*"]);
        assert_eq!(code, EXIT_OK, "{out}");
        assert!(
            out.contains("CoverPlate.thickness") && !out.contains("offset"),
            "{out}"
        );
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/names.rs::run_names
    #[test]
    fn usage_errors_exit_2() {
        let dir = project(SOURCE, BINDINGS, "");
        assert_eq!(run(&dir, Format::Text, &["nope"]).0, EXIT_USAGE);
        assert_eq!(
            run(&dir, Format::Text, &["equations", "other"]).0,
            EXIT_USAGE
        );
        assert_eq!(run(&dir, Format::Text, &["--bogus"]).0, EXIT_USAGE);
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/names.rs::run_names
    #[test]
    fn colliding_created_names_raise_r013() {
        let src = "model Part {\n  width = 1mm\n}\n";
        let dir = project(src, "", "");
        std::fs::write(dir.0.join("src/b.cdt"), src.replace("1mm", "2mm")).unwrap();
        let (code, out, _) = run(&dir, Format::Json, &[]);
        assert_eq!(code, EXIT_FAIL, "{out}");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["diagnostics"][0]["code"], "R013");
    }

    // frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
    // frob:tests crates/cadetta-cli/src/cmd/names.rs::run_names
    #[test]
    fn json_validates_against_names_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/names.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for extra in ["", "bind = \"scope\"\n"] {
            let dir = project(SOURCE, BINDINGS, extra);
            let v = json(&dir, &[]);
            assert!(validator.is_valid(&v), "{v}");
        }
    }

    // frob:ticket 01M4YQQPSNQ7TP3QFRWAYFH870
    // frob:tests crates/cadetta-cli/src/cmd/names.rs::run_names
    #[test]
    fn invalid_bindings_exit_two_naming_the_file() {
        let bad = "[\"acme.plate::CoverPlate.thickness\"]\nlocators = [ { locator = \"\", kind = \"global\" } ]\n";
        let dir = project(SOURCE, bad, "");
        let (code, out, e) = run(&dir, Format::Json, &[]);
        assert_eq!(code, EXIT_USAGE, "{out} {e}");
        assert!(e.contains("equations.toml"), "{e}");
        assert!(!out.contains("R005"), "{out}");
    }
}
