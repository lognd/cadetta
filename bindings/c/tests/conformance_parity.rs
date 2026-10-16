//! Replay conformance `check`/`config`/`eval` fixtures through a C program
//! linked against the C ABI and assert its stdout equals the CLI's
//! `--format json` (bindings.md 1, 4). frob:ticket 01M4SV2T5ACY1EFVCXF3JQ1E7R
//!
//! Builds the library and CLI when absent; needs a C compiler (`$CC`, else `cc`).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const SINGLE_FILE_TOML: &str = "[package]\nname = \"acme.demo\"\nversion = \"0.1.0\"\nas_of = \"2026-10-13\"\n\n[lint]\nfail_on = \"error\"\n";

fn target_dir() -> PathBuf {
    // target/<profile>/deps/<test-exe> -> target/<profile>
    let exe = std::env::current_exe().expect("exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("profile dir")
        .to_path_buf()
}

fn stage(fixture: &Path, root: &Path) {
    if fixture.is_dir() {
        copy_dir(fixture, root);
    } else {
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("cadetta.toml"), SINGLE_FILE_TOML).unwrap();
        std::fs::copy(fixture, root.join("src/lib.cdt")).unwrap();
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dest = to.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &dest);
        } else {
            std::fs::copy(e.path(), dest).unwrap();
        }
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() && !p.to_string_lossy().contains(".must-") {
            collect(&p, out);
        } else if p.to_string_lossy().ends_with(".expect.json") {
            out.push(p);
        }
    }
}

/// One staged fixture: the same package copied for the C path and the CLI path.
struct Job {
    name: String,
    verb: &'static str,
    extra: Vec<String>,
    c_root: PathBuf,
    k_root: PathBuf,
}

/// Run `prog --format json --cwd ROOT VERB ARGS`; return exit and scrubbed stdout.
fn run(prog: &Path, job: &Job) -> (Option<i32>, String) {
    let root = if prog.file_name().is_some_and(|n| n == "parity") {
        &job.c_root
    } else {
        &job.k_root
    };
    let o = Command::new(prog)
        .args(["--format", "json", "--cwd"])
        .arg(root)
        .arg(job.verb)
        .args(&job.extra)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&o.stdout).replace(root.to_str().unwrap(), "<ROOT>");
    (o.status.code(), text)
}

#[test]
fn c_abi_matches_cli_json_over_the_corpus() {
    let target = target_dir();
    let cli = target.join("cadetta");
    let find_lib = || {
        ["libcadetta_capi.so", "libcadetta_capi.dylib"]
            .iter()
            .map(|n| target.join(n))
            .find(|p| p.exists())
    };
    if !cli.exists() || find_lib().is_none() {
        // Build the artifacts under test (a no-op when CI already did).
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let ok = Command::new(cargo)
            .args(["build", "-p", "cadetta-capi", "-p", "cadetta"])
            .status()
            .expect("run cargo build")
            .success();
        assert!(ok, "building cadetta and cadetta-capi failed");
    }
    let lib = find_lib().expect("libcadetta_capi built");

    let work = std::env::temp_dir().join(format!("capi-parity-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    let harness = work.join("parity");
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let built = Command::new(&cc)
        .arg(here.join("tests/parity.c"))
        .arg("-I")
        .arg(here.join("include"))
        .arg(&lib)
        .arg(format!("-Wl,-rpath,{}", target.display()))
        .arg("-o")
        .arg(&harness)
        .status()
        .expect("run C compiler");
    assert!(built.success(), "C harness failed to compile");

    let corpus = here.join("../../tests/conformance");
    let mut expects = Vec::new();
    collect(&corpus, &mut expects);
    let mut jobs: Vec<Job> = Vec::new();
    for ef in expects {
        let spec: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&ef).unwrap()).unwrap();
        if spec["kind"] != "source" {
            continue;
        }
        let entry = spec
            .get("entry")
            .and_then(|v| v.as_str())
            .unwrap_or("check");
        let verb = match entry.split_whitespace().next().unwrap_or("check") {
            "check" | "config" => "check",
            "eval" => "ir",
            _ => continue,
        };
        // Only plain entries: extra args per fixture are passed through.
        if entry.split_whitespace().count() != 1 {
            continue;
        }
        let stem = ef
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_end_matches(".expect.json")
            .to_owned();
        for side in ["must-fire", "must-not-fire"] {
            let mut fx = ef.with_file_name(format!("{stem}.{side}"));
            if !fx.is_dir() {
                fx = ef.with_file_name(format!("{stem}.{side}.cdt"));
            }
            if !fx.exists() {
                continue;
            }
            let block = &spec[side.replace('-', "_")];
            let extra: Vec<String> = block["args"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let id = format!("{}", jobs.len());
            let (c_root, k_root) = (work.join("c").join(&id), work.join("cli").join(&id));
            stage(&fx, &c_root);
            stage(&fx, &k_root);
            jobs.push(Job {
                name: format!("{stem}:{side}"),
                verb,
                extra,
                c_root,
                k_root,
            });
        }
    }
    let next = AtomicUsize::new(0);
    let mismatches = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                while let Some(job) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let (cc_code, c_out) = run(&harness, job);
                    let (cli_code, cli_out) = run(&cli, job);
                    if cc_code != cli_code || c_out != cli_out {
                        mismatches
                            .lock()
                            .unwrap()
                            .push(format!("{}: exit {cc_code:?} vs {cli_code:?}", job.name));
                    }
                }
            });
        }
    });
    let replayed = jobs.len();
    let mismatches = mismatches.into_inner().unwrap();
    let _ = std::fs::remove_dir_all(&work);
    assert!(replayed > 20, "replayed only {replayed} fixtures");
    assert!(
        mismatches.is_empty(),
        "C ABI differs from CLI: {mismatches:#?}"
    );
}
