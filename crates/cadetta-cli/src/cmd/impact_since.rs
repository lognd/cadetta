//! frob:ticket 01M4SWN7GJW88YVBJ2XVR00EKX
//! `cadetta impact --since REV` / `--worktree`: the diff of two evaluations, REV (default
//! `HEAD`) against the working tree (obligations.md 5, config-cli.md 3).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use cadetta_ir::model::Project as Ir;
use cadetta_obligations::impact_since::{Change, ChangedNode, Diff, VerdictFlip, diff};
use serde_json::{Value as Json, json};

use super::impact::{Kind, downstream};
use super::impact_swap::report::file_of;
use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::Project;
use crate::{EXIT_OK, EXIT_USAGE, Globals};

/// Schema tag of the JSON report; the schema is `docs/schemas/impact-since.json`.
pub const SCHEMA: &str = "cadetta.impact-since/1";

const USAGE: &str = "usage: cadetta impact [--since REV] [--worktree]";

/// A CAD file with the bound paths whose value changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resync {
    /// The CAD file.
    pub file: String,
    /// Changed bound paths, sorted.
    pub paths: Vec<String>,
}

/// The whole report for one change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// The older side, as the user named it.
    pub since: String,
    /// Changed nodes, sorted by path.
    pub nodes: Vec<ChangedNode>,
    /// Obligations that become stale or may flip: id and effect, sorted.
    pub obligations: Vec<(String, &'static str)>,
    /// Verdicts that flipped, sorted by path.
    pub flips: Vec<VerdictFlip>,
    /// CAD files to re-sync, sorted by name.
    pub cad: Vec<Resync>,
}

/// CAD files bound to any of `moved` in `ir`, with the moved paths each holds.
fn cad_resync(ir: &Ir, moved: &BTreeSet<String>) -> Vec<Resync> {
    let mut files: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for b in &ir.bindings {
        let path = b.path.to_string();
        if !moved.contains(&path) {
            continue;
        }
        for l in &b.locators {
            files
                .entry(file_of(&l.locator, &b.adapter))
                .or_default()
                .insert(path.clone());
        }
    }
    files
        .into_iter()
        .map(|(file, paths)| Resync {
            file,
            paths: paths.into_iter().collect(),
        })
        .collect()
}

/// Diff the packages of two evaluations (matched by name) into one report.
pub fn build(since: &str, old: &[(String, Ir)], new: &[(String, Ir)]) -> Report {
    let mut report = Report {
        since: since.to_owned(),
        ..Report::default()
    };
    let mut obligations: BTreeMap<String, &'static str> = BTreeMap::new();
    for (name, new_ir) in new {
        let empty;
        let old_ir = match old.iter().find(|(n, _)| n == name) {
            Some((_, ir)) => ir,
            None => {
                empty = Ir::new(new_ir.flavour, new_ir.package.clone());
                &empty
            }
        };
        let d: Diff = diff(old_ir, new_ir);
        let mut moved: BTreeSet<String> = BTreeSet::new();
        for path in d.live_paths() {
            moved.insert(path.to_owned());
            // The node itself, then everything downstream of it.
            let Ok(impact) = downstream(new_ir, path) else {
                continue;
            };
            for dep in &impact.dependents {
                if dep.kind == Kind::Member {
                    moved.insert(dep.path.clone());
                }
            }
            for (id, effect) in impact.obligations {
                obligations.insert(id, effect);
            }
        }
        for n in d.nodes.iter().filter(|n| n.change != Change::Removed) {
            match n.kind {
                cadetta_obligations::impact_since::NodeKind::Solver => {
                    obligations.insert(format!("solver:{}", n.path), "stale");
                }
                cadetta_obligations::impact_since::NodeKind::Check => {
                    obligations.insert(format!("check:{}", n.path), "may_flip");
                }
                cadetta_obligations::impact_since::NodeKind::Member => {}
            }
        }
        for f in &d.flips {
            obligations.insert(format!("check:{}", f.path), "flipped");
        }
        report.cad.extend(cad_resync(new_ir, &moved));
        report.nodes.extend(d.nodes);
        report.flips.extend(d.flips);
    }
    report.nodes.sort_by(|a, b| a.path.cmp(&b.path));
    report.flips.sort_by(|a, b| a.path.cmp(&b.path));
    report.cad.sort_by(|a, b| a.file.cmp(&b.file));
    report.obligations = obligations.into_iter().collect();
    tracing::info!(
        nodes = report.nodes.len(),
        obligations = report.obligations.len(),
        flips = report.flips.len(),
        files = report.cad.len(),
        "impact since: report built"
    );
    report
}

/// Render the report as text or the `cadetta.impact-since/1` JSON document.
pub fn render(r: &Report, format: Format, out: &mut dyn Write) {
    if format == Format::Json {
        let doc = json!({
            "schema": SCHEMA,
            "since": r.since,
            "changed_nodes": r.nodes.iter().map(|n| json!({
                "path": n.path, "kind": n.kind.word(), "change": n.change.word(),
            })).collect::<Vec<Json>>(),
            "obligations": r.obligations.iter()
                .map(|(id, effect)| json!({"id": id, "effect": effect}))
                .collect::<Vec<Json>>(),
            "verdicts_flipped": r.flips.iter().map(|f| json!({
                "path": f.path, "old": f.old, "new": f.new,
            })).collect::<Vec<Json>>(),
            "cad_resync": r.cad.iter()
                .map(|c| json!({"file": c.file, "paths": c.paths}))
                .collect::<Vec<Json>>(),
        });
        let _ = writeln!(out, "{doc}");
        return;
    }
    if r.nodes.is_empty() && r.flips.is_empty() {
        let _ = writeln!(out, "since {}: no changes", r.since);
        return;
    }
    let _ = writeln!(out, "since {}", r.since);
    let _ = writeln!(out, "1. changed nodes: {}", r.nodes.len());
    for n in &r.nodes {
        let _ = writeln!(out, "  {} [{}] {}", n.path, n.kind.word(), n.change.word());
    }
    let _ = writeln!(out, "2. obligations affected: {}", r.obligations.len());
    for (id, effect) in &r.obligations {
        let _ = writeln!(out, "  {id}: {effect}");
    }
    let _ = writeln!(out, "3. verdicts flipped: {}", r.flips.len());
    for f in &r.flips {
        let _ = writeln!(out, "  {}: {} -> {}", f.path, f.old, f.new);
    }
    let _ = writeln!(out, "4. CAD files to re-sync: {}", r.cad.len());
    for c in &r.cad {
        let _ = writeln!(out, "  {} ({})", c.file, c.paths.join(", "));
    }
}

/// Run `git` with `args` in `dir`; its trimmed stdout, or the failure text.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !o.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).trim().to_owned())
}

/// A checkout of a revision in a scratch directory, removed on drop.
struct Checkout {
    /// The scratch directory holding the whole exported tree.
    scratch: PathBuf,
    /// Where the project root sits inside the exported tree.
    dir: PathBuf,
}

impl Checkout {
    /// Export `rev` of the repository containing `root` into a fresh scratch directory.
    fn export(root: &Path, rev: &str) -> Result<Checkout, String> {
        let top = PathBuf::from(git(root, &["rev-parse", "--show-toplevel"])?);
        let top = top.canonicalize().unwrap_or(top);
        let root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
        let scratch = std::env::temp_dir().join(format!("cadetta-since-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch)
            .map_err(|e| format!("cannot create {}: {e}", scratch.display()))?;
        let dir = scratch.join(root.strip_prefix(&top).unwrap_or(Path::new("")));
        let checkout = Checkout { scratch, dir };
        let mut archive = Command::new("git")
            .arg("-C")
            .arg(&top)
            .args(["archive", "--format=tar", rev])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot run git: {e}"))?;
        let stdout = archive.stdout.take().ok_or("git archive gave no output")?;
        let tar = Command::new("tar")
            .arg("-x")
            .arg("-C")
            .arg(&checkout.scratch)
            .stdin(Stdio::from(stdout))
            .status()
            .map_err(|e| format!("cannot run tar: {e}"))?;
        let archived = archive
            .wait_with_output()
            .map_err(|e| format!("git archive: {e}"))?;
        if !archived.status.success() {
            return Err(format!(
                "cannot read revision `{rev}`: {}",
                String::from_utf8_lossy(&archived.stderr).trim()
            ));
        }
        if !tar.success() {
            return Err("tar failed to unpack the revision".to_owned());
        }
        tracing::debug!(rev, dir = %checkout.dir.display(), "impact since: exported");
        Ok(checkout)
    }
}

impl Drop for Checkout {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

/// Analyze the project at `dir` into named IRs.
fn evaluate(
    dir: Option<&Path>,
    config: Option<&Path>,
    what: &str,
) -> Result<(Vec<(String, Ir)>, PathBuf), String> {
    let (project, members) = Project::open(dir, config).map_err(|e| format!("{what}: {e}"))?;
    let analysis = analyze(&project, &members);
    if analysis.halted {
        return Err(format!("{what}: a syntax error stopped analysis"));
    }
    let irs = analysis
        .packages
        .into_iter()
        .map(|p| (p.name, p.ir))
        .collect();
    Ok((irs, project.root))
}

/// Parse `[--since REV] [--worktree]` into the revision.
fn parse(args: &[String]) -> Result<String, String> {
    let mut rev = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--worktree" => {}
            "--since" => match it.next() {
                Some(r) if !r.starts_with("--") => rev = Some(r.clone()),
                _ => return Err(format!("`--since` needs a revision; {USAGE}")),
            },
            other => return Err(format!("unexpected `{other}`; {USAGE}")),
        }
    }
    Ok(rev.unwrap_or_else(|| "HEAD".to_owned()))
}

/// Run `cadetta impact --since REV` / `--worktree`; `args` are the words after the verb.
pub fn run_since(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str, code: u8| {
        tracing::error!(%message, "impact since failed");
        let _ = writeln!(err, "error: {message}");
        code
    };
    let rev = match parse(args) {
        Ok(r) => r,
        Err(m) => return fail(err, &m, EXIT_USAGE),
    };
    let (new, root) = match evaluate(
        globals.cwd.as_deref(),
        globals.config.as_deref(),
        "worktree",
    ) {
        Ok(o) => o,
        Err(m) => return fail(err, &m, EXIT_USAGE),
    };
    let checkout = match Checkout::export(&root, &rev) {
        Ok(c) => c,
        Err(m) => return fail(err, &m, EXIT_USAGE),
    };
    let old = match evaluate(Some(&checkout.dir), None, &rev) {
        Ok((irs, _)) => irs,
        Err(m) => return fail(err, &m, EXIT_USAGE),
    };
    render(&build(&rev, &old, &new), globals.format, out);
    EXIT_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{
        Binding, Check, Component, ComponentClass, Flavour, Locator, LocatorKind, LocatorMode,
        Member, MemberKind, PackageId, VerdictState, Verdicts,
    };
    use cadetta_ir::path::{Path as IrPath, PathContext};

    fn p(s: &str) -> IrPath {
        IrPath::parse(s, PathContext::General).unwrap()
    }

    fn ir(expr: &str, verdict: VerdictState) -> Ir {
        let mut ir = Ir::new(
            Flavour::Design,
            PackageId {
                name: "pkg".into(),
                version: "0.1.0".into(),
            },
        );
        let mut a = Member::new(p("pkg::X.a"), MemberKind::Param);
        a.expr = Some(expr.to_owned());
        let mut b = Member::new(p("pkg::X.b"), MemberKind::Param);
        b.deps = vec![p("pkg::X.a")];
        ir.components.push(Component {
            path: p("pkg::X.a"),
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members: vec![a, b],
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        });
        ir.checks.push(Check {
            path: p("pkg::X.ok"),
            expr: "pkg::X.b < 9mm".into(),
            verdicts: Verdicts {
                primary: Some(verdict),
                ..Verdicts::default()
            },
            conservative: false,
            waived_for: vec![],
        });
        ir.bindings.push(Binding {
            path: p("pkg::X.b"),
            adapter: "freecad".into(),
            locators: vec![Locator {
                locator: "Sheet@rover.FCStd".into(),
                kind: LocatorKind::Dimension,
                mode: LocatorMode::Value,
                native_unit: "mm".into(),
                ref_to: None,
                for_alternate: None,
                config: None,
                sign: None,
                component: None,
                implicit: None,
                tolerance: None,
            }],
        });
        ir
    }

    fn pair(a: Ir, b: Ir) -> Report {
        build("HEAD", &[("pkg".to_owned(), a)], &[("pkg".to_owned(), b)])
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_since.rs::build
    #[test]
    fn reports_nodes_obligations_flips_and_files() {
        let r = pair(ir("1mm", VerdictState::Pass), ir("2mm", VerdictState::Fail));
        let paths: Vec<_> = r.nodes.iter().map(|n| n.path.as_str()).collect();
        assert_eq!(paths, ["pkg::X.a", "pkg::X.ok"]);
        assert_eq!(r.obligations, [("check:pkg::X.ok".to_owned(), "flipped")]);
        assert_eq!(r.flips.len(), 1);
        assert_eq!(r.cad.len(), 1);
        assert_eq!(r.cad[0].file, "rover.FCStd");
        assert_eq!(r.cad[0].paths, ["pkg::X.b"]);
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_since.rs::render
    #[test]
    fn unchanged_is_said_plainly_and_json_validates() {
        let same = pair(ir("1mm", VerdictState::Pass), ir("1mm", VerdictState::Pass));
        let mut out = Vec::new();
        render(&same, Format::Text, &mut out);
        assert_eq!(String::from_utf8(out).unwrap(), "since HEAD: no changes\n");
        let schema: Json =
            serde_json::from_str(include_str!("../../../../docs/schemas/impact-since.json"))
                .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for r in [
            same,
            pair(ir("1mm", VerdictState::Pass), ir("2mm", VerdictState::Fail)),
        ] {
            let mut out = Vec::new();
            render(&r, Format::Json, &mut out);
            let v: Json = serde_json::from_slice(&out).unwrap();
            assert!(validator.is_valid(&v), "{v}");
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/impact_since.rs::parse
    #[test]
    fn parses_flags() {
        assert_eq!(parse(&[]).unwrap(), "HEAD");
        assert_eq!(parse(&["--worktree".into()]).unwrap(), "HEAD");
        assert_eq!(
            parse(&["--since".into(), "v1".into(), "--worktree".into()]).unwrap(),
            "v1"
        );
        assert!(parse(&["--since".into()]).is_err());
        assert!(parse(&["x".into()]).is_err());
    }
}
