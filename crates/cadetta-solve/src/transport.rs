//! Solver transports and the command template (solvers.md 3, config-cli.md 2).
//!
//! Decisions made here (written into solvers.md 3): the command template is a
//! list of arguments; `{script}`, `{in}` and `{out}` are the only placeholders
//! and are expanded inside each argument. `{in}` and `{out}` exist only under
//! the `file` transport; an unknown or unbound placeholder is a configuration
//! error. The solver runs with the project directory as its working directory.
//! An optional `timeout` (seconds) kills the child and reports a timeout.
//! Every failure names the solver; an unknown solver name is a usage error
//! (exit 2), every other failure is a solver error (S005).

// frob:ticket 01M4STAN28E01T9BY5M07ZD4V5

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace, warn};

use crate::protocol::{ProtocolError, Request, Response, parse_response};

/// How a request reaches the solver and the response comes back.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// Request on stdin, response on stdout.
    #[default]
    Stdio,
    /// Request and response in files named by `{in}` and `{out}`.
    File,
}

/// One `[solvers."PATH"]` table of `cadetta.toml`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverConfig {
    /// The command template: program then arguments, with placeholders.
    pub command: Vec<String>,
    /// The transport; defaults to `stdio`.
    #[serde(default)]
    pub transport: Transport,
    /// Seconds before the solver is killed; unset means no limit.
    #[serde(default)]
    pub timeout: Option<u64>,
}

/// Why a solver run failed; every variant but `UnknownSolver` is S005.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// No `[solvers.NAME]` table for the solver (usage error, exit 2).
    #[error("unknown solver `{solver}`: no [solvers.\"{solver}\"] table in cadetta.toml")]
    UnknownSolver {
        /// The solver path asked for.
        solver: String,
    },
    /// The command template is unusable.
    #[error("solver `{solver}`: bad command template: {reason}")]
    BadTemplate {
        /// The solver path.
        solver: String,
        /// What is wrong.
        reason: String,
    },
    /// The process could not be started or its pipes failed.
    #[error("solver `{solver}`: {context}: {source}")]
    Io {
        /// The solver path.
        solver: String,
        /// What was being done.
        context: String,
        /// The OS error.
        source: std::io::Error,
    },
    /// The process exited non-zero.
    #[error("solver `{solver}` exited with status {status}: {stderr}")]
    Exit {
        /// The solver path.
        solver: String,
        /// The exit status rendered (`code 3`, or `signal`).
        status: String,
        /// Trimmed stderr of the process.
        stderr: String,
    },
    /// The process ran past its timeout and was killed.
    #[error("solver `{solver}` timed out after {secs}s")]
    Timeout {
        /// The solver path.
        solver: String,
        /// The limit that was exceeded.
        secs: u64,
    },
    /// The response was rejected by the protocol layer.
    #[error("solver `{solver}`: {source}")]
    Protocol {
        /// The solver path.
        solver: String,
        /// The protocol failure.
        source: ProtocolError,
    },
}

impl TransportError {
    /// The process exit status the CLI uses: 2 for a usage error, else 1.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::UnknownSolver { .. } | Self::BadTemplate { .. } => 2,
            _ => 1,
        }
    }

    /// The catalog code reported: Q001 or S005 from the protocol, S005 otherwise.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Protocol { source, .. } => source.code(),
            _ => "S005",
        }
    }
}

/// Paths a template may reference.
#[derive(Clone, Debug, Default)]
pub struct Placeholders {
    /// The solver script (`{script}`).
    pub script: Option<PathBuf>,
    /// The request file (`{in}`), file transport only.
    pub input: Option<PathBuf>,
    /// The response file (`{out}`), file transport only.
    pub output: Option<PathBuf>,
}

/// Expand `{script}`, `{in}` and `{out}` in each template argument.
pub fn expand_command(
    solver: &str,
    template: &[String],
    ph: &Placeholders,
) -> Result<Vec<String>, TransportError> {
    let bad = |reason: String| TransportError::BadTemplate {
        solver: solver.to_string(),
        reason,
    };
    if template.is_empty() || template[0].is_empty() {
        return Err(bad("command is empty".into()));
    }
    let mut out = Vec::with_capacity(template.len());
    for arg in template {
        let mut s = String::new();
        let mut rest = arg.as_str();
        while let Some(i) = rest.find('{') {
            s.push_str(&rest[..i]);
            let tail = &rest[i + 1..];
            let Some(j) = tail.find('}') else {
                return Err(bad(format!("unclosed `{{` in `{arg}`")));
            };
            let name = &tail[..j];
            let val = match name {
                "script" => &ph.script,
                "in" => &ph.input,
                "out" => &ph.output,
                _ => return Err(bad(format!("unknown placeholder `{{{name}}}`"))),
            };
            let Some(p) = val else {
                return Err(bad(format!(
                    "placeholder `{{{name}}}` is not available here"
                )));
            };
            s.push_str(&p.to_string_lossy());
            rest = &tail[j + 1..];
        }
        s.push_str(rest);
        out.push(s);
    }
    trace!(solver, argv = ?out, "command expanded");
    Ok(out)
}

static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(solver: &str) -> Result<Self, TransportError> {
        let n = RUN_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cadetta-solve-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| io_err(solver, "creating scratch dir", e))?;
        Ok(Self(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn io_err(solver: &str, context: &str, source: std::io::Error) -> TransportError {
    TransportError::Io {
        solver: solver.to_string(),
        context: context.to_string(),
        source,
    }
}

/// Captured result of a finished child.
struct Finished {
    stdout: String,
}

/// Run `argv` in `cwd`, feeding `stdin`, enforcing `timeout`; fail on non-zero exit.
fn spawn_and_wait(
    solver: &str,
    argv: &[String],
    cwd: &Path,
    stdin: Option<String>,
    timeout: Option<u64>,
) -> Result<Finished, TransportError> {
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(cwd)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| io_err(solver, &format!("starting `{}`", argv[0]), e))?;
    debug!(solver, program = %argv[0], pid = child.id(), "solver started");

    let writer = stdin.map(|text| {
        let mut pipe = child.stdin.take().expect("stdin was piped");
        thread::spawn(move || {
            // A solver that exits without reading its input closes the pipe; that is its exit status to report.
            let _ = pipe.write_all(text.as_bytes());
        })
    });
    let mut out_pipe = child.stdout.take().expect("stdout was piped");
    let mut err_pipe = child.stderr.take().expect("stderr was piped");
    let out_t = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out_pipe.read_to_end(&mut b);
        b
    });
    let err_t = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err_pipe.read_to_end(&mut b);
        b
    });

    let start = Instant::now();
    let status = loop {
        match child
            .try_wait()
            .map_err(|e| io_err(solver, "waiting for the process", e))?
        {
            Some(s) => break s,
            None => {
                if let Some(secs) = timeout
                    && start.elapsed() >= Duration::from_secs(secs)
                {
                    warn!(solver, secs, "solver timed out; killing");
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(TransportError::Timeout {
                        solver: solver.to_string(),
                        secs,
                    });
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
    };
    if let Some(w) = writer {
        let _ = w.join();
    }
    let stdout = String::from_utf8_lossy(&out_t.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&err_t.join().unwrap_or_default()).into_owned();
    if !status.success() {
        let status = match status.code() {
            Some(c) => format!("code {c}"),
            None => "signal".to_string(),
        };
        warn!(solver, %status, "solver failed");
        return Err(TransportError::Exit {
            solver: solver.to_string(),
            status,
            stderr: stderr.trim().to_string(),
        });
    }
    Ok(Finished { stdout })
}

/// Run the solver `name` for `request` using its configured command and transport.
pub fn run_solver(
    configs: &BTreeMap<String, SolverConfig>,
    name: &str,
    script: &Path,
    request: &Request,
    project_dir: &Path,
) -> Result<Response, TransportError> {
    let Some(cfg) = configs.get(name) else {
        warn!(solver = name, "unknown solver");
        return Err(TransportError::UnknownSolver {
            solver: name.to_string(),
        });
    };
    info!(solver = name, transport = ?cfg.transport, "running solver");
    let mut ph = Placeholders {
        script: Some(script.to_path_buf()),
        ..Placeholders::default()
    };
    let text = match cfg.transport {
        Transport::Stdio => {
            let argv = expand_command(name, &cfg.command, &ph)?;
            spawn_and_wait(
                name,
                &argv,
                project_dir,
                Some(request.to_json()),
                cfg.timeout,
            )?
            .stdout
        }
        Transport::File => {
            let scratch = Scratch::new(name)?;
            let (inp, outp) = (scratch.0.join("in.json"), scratch.0.join("out.json"));
            std::fs::write(&inp, request.to_json())
                .map_err(|e| io_err(name, "writing the request file", e))?;
            ph.input = Some(inp);
            ph.output = Some(outp.clone());
            let argv = expand_command(name, &cfg.command, &ph)?;
            spawn_and_wait(name, &argv, project_dir, None, cfg.timeout)?;
            std::fs::read_to_string(&outp)
                .map_err(|e| io_err(name, "reading the response file", e))?
        }
    };
    parse_response(&text).map_err(|source| TransportError::Protocol {
        solver: name.to_string(),
        source,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    const OK: &str = r#"{"schema":"cadetta.solve/1","outputs":{"w":"3.2mm"},"diagnostics":[]}"#;

    /// Write the canned response to a file and return its path.
    fn fixture() -> String {
        // Written once per process: parallel tests must not truncate the file mid-read.
        static PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
        let p = PATH.get_or_init(|| {
            let p = std::env::temp_dir()
                .join(format!("cadetta-solve-fixture-{}.json", std::process::id()));
            std::fs::write(&p, OK).unwrap();
            p
        });
        p.to_string_lossy().into_owned()
    }

    fn cfg(
        cmd: &[&str],
        transport: Transport,
        timeout: Option<u64>,
    ) -> BTreeMap<String, SolverConfig> {
        let mut m = BTreeMap::new();
        m.insert(
            "p::s".to_string(),
            SolverConfig {
                command: cmd.iter().map(|s| s.to_string()).collect(),
                transport,
                timeout,
            },
        );
        m
    }

    fn run(c: &BTreeMap<String, SolverConfig>, name: &str) -> Result<Response, TransportError> {
        let req = Request::new(name, BTreeMap::new());
        run_solver(c, name, Path::new("script.sh"), &req, &std::env::temp_dir())
    }

    #[test]
    fn expands_placeholders_and_runs_stdio() {
        let script = format!("cat >/dev/null; cat {}", fixture());
        let c = cfg(&["sh", "-c", &script, "{script}"], Transport::Stdio, None);
        let r = run(&c, "p::s").unwrap();
        assert_eq!(r.outputs["w"], "3.2mm");
    }

    #[test]
    fn expands_placeholders_and_runs_file() {
        let c = cfg(
            &["sh", "-c", &format!("cat {} > \"$0\"", fixture()), "{out}"],
            Transport::File,
            None,
        );
        assert_eq!(run(&c, "p::s").unwrap().outputs["w"], "3.2mm");
    }

    #[test]
    fn file_transport_passes_request_in_file() {
        let c = cfg(
            &[
                "sh",
                "-c",
                &format!(
                    "grep -q cadetta.solve/1 \"$0\" && cat {} > \"$1\"",
                    fixture()
                ),
                "{in}",
                "{out}",
            ],
            Transport::File,
            None,
        );
        assert!(run(&c, "p::s").is_ok());
    }

    #[test]
    fn template_rejects_unknown_and_unavailable_placeholders() {
        let ph = Placeholders::default();
        let e = expand_command("p::s", &["x{nope}".into()], &ph).unwrap_err();
        assert!(e.to_string().contains("unknown placeholder"));
        let e = expand_command("p::s", &["{in}".into()], &ph).unwrap_err();
        assert!(e.to_string().contains("not available"));
        assert_eq!(e.exit_code(), 2);
    }

    #[test]
    fn failing_solver_error_names_solver() {
        let c = cfg(
            &["sh", "-c", "echo boom >&2; exit 3"],
            Transport::Stdio,
            None,
        );
        let e = run(&c, "p::s").unwrap_err();
        assert!(matches!(e, TransportError::Exit { .. }));
        let m = e.to_string();
        assert!(m.contains("p::s") && m.contains("code 3") && m.contains("boom"));
        assert_eq!(e.code(), "S005");
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn slow_solver_error_names_solver() {
        let c = cfg(&["sh", "-c", "sleep 5"], Transport::Stdio, Some(1));
        let t = Instant::now();
        let e = run(&c, "p::s").unwrap_err();
        assert!(matches!(e, TransportError::Timeout { secs: 1, .. }));
        assert!(e.to_string().contains("p::s"));
        assert!(t.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn unknown_solver_is_exit_2() {
        let c = cfg(&["true"], Transport::Stdio, None);
        let e = run(&c, "other::x").unwrap_err();
        assert!(matches!(e, TransportError::UnknownSolver { .. }));
        assert_eq!(e.exit_code(), 2);
        assert!(e.to_string().contains("other::x"));
    }

    #[test]
    fn bad_response_is_protocol_error() {
        let c = cfg(&["sh", "-c", "echo nope"], Transport::Stdio, None);
        let e = run(&c, "p::s").unwrap_err();
        assert!(matches!(e, TransportError::Protocol { .. }));
        assert_eq!(e.code(), "S005");
    }
}
