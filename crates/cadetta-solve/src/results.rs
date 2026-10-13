//! Result files `<stem>.out.json` keyed by input digest (solvers.md 4).
//!
//! Decisions made here (written into solvers.md 4): digests are
//! `blake3:<hex>` over the canonical request JSON (sorted keys, no
//! whitespace) until the IR digest scheme lands; a recorded result is fresh
//! only when input, script and every dep digest match, otherwise it is stale
//! and its outputs still evaluate; results are keyed by (solver path,
//! variant) and a file is rewritten sorted by that key. Digests live only in
//! this file, never in `cadetta.lock`.

// frob:ticket 01M4STAN5C20ACNDRARX5MRDDP

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::protocol::{InputValue, Request, SolverDiagnostic};

/// The schema tag of a result file.
pub const RESULT_SCHEMA: &str = "cadetta.solve-result/1";

/// Why a result file could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum ResultsError {
    /// Filesystem failure.
    #[error("{path}: {source}")]
    Io {
        /// The file involved.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The file is not a valid `cadetta.solve-result/1` document.
    #[error("{path}: invalid result file: {reason}")]
    Invalid {
        /// The file involved.
        path: PathBuf,
        /// What is wrong.
        reason: String,
    },
}

/// One stored solve result.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SolveResult {
    /// Canonical solver path, e.g. `acme.drive::DriveModule.drive_torque`.
    pub solver: String,
    /// Digest of the request's input values.
    pub input_digest: String,
    /// Digest of the solver script.
    pub script_digest: String,
    /// Digest of each dependency, by path.
    pub dep_digests: BTreeMap<String, String>,
    /// The solver's own version, when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// `SLOT=ALT` when this result is for a variant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// The inputs the solver was given.
    pub inputs: BTreeMap<String, InputValue>,
    /// Output literals by name.
    pub outputs: BTreeMap<String, String>,
    /// Solver diagnostics.
    pub diagnostics: Vec<SolverDiagnostic>,
}

/// The digests a result is compared against to decide freshness.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CurrentDigests {
    /// Digest of the current input values.
    pub input_digest: String,
    /// Digest of the current script.
    pub script_digest: String,
    /// Digest of each current dependency.
    pub dep_digests: BTreeMap<String, String>,
}

/// What a lookup found.
#[derive(Clone, PartialEq, Debug)]
pub enum Lookup<'a> {
    /// All digests match: use the cached result, do not run the solver.
    Fresh(&'a SolveResult),
    /// A result exists but a digest differs: S003; outputs still evaluate.
    Stale(&'a SolveResult),
    /// No result for this solver and variant: outputs unknown (S006 for variants).
    Missing,
}

/// The contents of one `<stem>.out.json`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ResultFile {
    /// Always [`RESULT_SCHEMA`].
    pub schema: String,
    /// Results sorted by solver path then variant.
    pub results: Vec<SolveResult>,
}

impl Default for ResultFile {
    fn default() -> Self {
        Self {
            schema: RESULT_SCHEMA.to_string(),
            results: Vec::new(),
        }
    }
}

/// Digest arbitrary bytes as `blake3:<hex>`.
pub fn digest_bytes(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

/// Digest a request's canonical JSON (struct and map keys are already sorted).
pub fn input_digest(request: &Request) -> String {
    digest_bytes(request.to_json().as_bytes())
}

/// The result file beside `script`: `drive_torque.py` gives `drive_torque.out.json`.
pub fn out_path(script: &Path) -> PathBuf {
    script.with_extension("out.json")
}

impl ResultFile {
    /// Read `path`; a missing file is an empty result set.
    pub fn load(path: &Path) -> Result<Self, ResultsError> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!(path = %path.display(), "no result file yet");
                return Ok(Self::default());
            }
            Err(source) => {
                return Err(ResultsError::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };
        let file: Self = serde_json::from_str(&text).map_err(|e| ResultsError::Invalid {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
        if file.schema != RESULT_SCHEMA {
            warn!(path = %path.display(), schema = %file.schema, "wrong result schema");
            return Err(ResultsError::Invalid {
                path: path.to_path_buf(),
                reason: format!("expected schema `{RESULT_SCHEMA}`, found `{}`", file.schema),
            });
        }
        debug!(path = %path.display(), n = file.results.len(), "loaded results");
        Ok(file)
    }

    /// Write the file sorted by solver path then variant, pretty-printed, ending in a newline.
    pub fn save(&mut self, path: &Path) -> Result<(), ResultsError> {
        self.sort();
        let mut text = serde_json::to_string_pretty(self).expect("results are plain data");
        text.push('\n');
        std::fs::write(path, text).map_err(|source| ResultsError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        info!(path = %path.display(), n = self.results.len(), "wrote results");
        Ok(())
    }

    /// Insert or replace the result for its (solver, variant) key.
    pub fn upsert(&mut self, result: SolveResult) {
        match self
            .results
            .iter_mut()
            .find(|r| r.solver == result.solver && r.variant == result.variant)
        {
            Some(slot) => {
                debug!(solver = %result.solver, "replacing stored result");
                *slot = result;
            }
            None => {
                debug!(solver = %result.solver, "adding stored result");
                self.results.push(result);
            }
        }
        self.sort();
    }

    /// Find the result for `solver` and `variant` and classify it against `current`.
    pub fn lookup(
        &self,
        solver: &str,
        variant: Option<&str>,
        current: &CurrentDigests,
    ) -> Lookup<'_> {
        let Some(r) = self
            .results
            .iter()
            .find(|r| r.solver == solver && r.variant.as_deref() == variant)
        else {
            debug!(solver, ?variant, "no stored result");
            return Lookup::Missing;
        };
        if r.input_digest == current.input_digest
            && r.script_digest == current.script_digest
            && r.dep_digests == current.dep_digests
        {
            debug!(solver, ?variant, "stored result is fresh");
            Lookup::Fresh(r)
        } else {
            debug!(solver, ?variant, "stored result is stale");
            Lookup::Stale(r)
        }
    }

    fn sort(&mut self) {
        self.results.sort_by(|a, b| {
            (a.solver.as_str(), a.variant.as_deref())
                .cmp(&(b.solver.as_str(), b.variant.as_deref()))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(solver: &str, lit: &str) -> Request {
        let mut inputs = BTreeMap::new();
        inputs.insert(
            "x".to_string(),
            InputValue {
                lit: lit.to_string(),
                si: 1.0,
                min: None,
                max: None,
            },
        );
        Request::new(solver, inputs)
    }

    fn result(solver: &str, variant: Option<&str>, r: &Request) -> SolveResult {
        SolveResult {
            solver: solver.to_string(),
            input_digest: input_digest(r),
            script_digest: digest_bytes(b"script"),
            dep_digests: BTreeMap::new(),
            version: None,
            variant: variant.map(str::to_string),
            inputs: r.inputs.clone(),
            outputs: BTreeMap::from([("t".to_string(), "5N*m".to_string())]),
            diagnostics: vec![],
        }
    }

    fn current(r: &Request) -> CurrentDigests {
        CurrentDigests {
            input_digest: input_digest(r),
            script_digest: digest_bytes(b"script"),
            dep_digests: BTreeMap::new(),
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cadetta-results-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // frob:tests crates/cadetta-solve/src/results.rs::ResultFile.lookup kind=unit
    #[test]
    fn unchanged_inputs_are_fresh() {
        let r = req("acme.drive::drive_torque", "300mm");
        let mut f = ResultFile::default();
        f.upsert(result(&r.solver, None, &r));
        assert!(matches!(
            f.lookup(&r.solver, None, &current(&r)),
            Lookup::Fresh(_)
        ));
    }

    // frob:tests crates/cadetta-solve/src/results.rs::ResultFile.lookup kind=unit
    #[test]
    fn changed_inputs_are_stale_but_readable() {
        let r = req("acme.drive::drive_torque", "300mm");
        let mut f = ResultFile::default();
        f.upsert(result(&r.solver, None, &r));
        let changed = req("acme.drive::drive_torque", "310mm");
        match f.lookup(&r.solver, None, &current(&changed)) {
            Lookup::Stale(s) => assert_eq!(s.outputs["t"], "5N*m"),
            other => panic!("expected stale, got {other:?}"),
        }
        let mut cur = current(&r);
        cur.dep_digests.insert("acme::d".into(), "blake3:00".into());
        assert!(matches!(f.lookup(&r.solver, None, &cur), Lookup::Stale(_)));
        assert_eq!(
            f.lookup(&r.solver, Some("A=B"), &current(&r)),
            Lookup::Missing
        );
    }

    // frob:tests crates/cadetta-solve/src/results.rs::ResultFile.save kind=unit
    #[test]
    fn file_is_sorted_and_named_beside_script() {
        assert_eq!(
            out_path(Path::new("p/drive_torque.py")),
            PathBuf::from("p/drive_torque.out.json")
        );
        let r = req("s", "1mm");
        let mut f = ResultFile::default();
        f.upsert(result("b::z", None, &r));
        f.upsert(result("a::y", Some("S=B"), &r));
        f.upsert(result("a::y", Some("S=A"), &r));
        f.upsert(result("a::y", None, &r));
        let dir = tmp("sorted");
        let p = dir.join("x.out.json");
        f.save(&p).unwrap();
        let back = ResultFile::load(&p).unwrap();
        assert_eq!(back, f);
        assert_eq!(back.schema, "cadetta.solve-result/1");
        let order: Vec<_> = back
            .results
            .iter()
            .map(|r| (r.solver.as_str(), r.variant.as_deref()))
            .collect();
        assert_eq!(
            order,
            vec![
                ("a::y", None),
                ("a::y", Some("S=A")),
                ("a::y", Some("S=B")),
                ("b::z", None)
            ]
        );
        assert!(
            ResultFile::load(&dir.join("none.out.json"))
                .unwrap()
                .results
                .is_empty()
        );
    }

    // frob:tests crates/cadetta-solve/src/results.rs::ResultFile.upsert kind=unit
    #[test]
    fn shared_script_solvers_never_collide() {
        let r = req("s", "1mm");
        let mut f = ResultFile::default();
        f.upsert(result("acme.drive::DriveModule.drive_torque", None, &r));
        f.upsert(result("acme.drive::drive_torque", None, &r));
        assert_eq!(f.results.len(), 2);
        f.upsert(result("acme.drive::drive_torque", None, &r));
        assert_eq!(f.results.len(), 2);
    }

    // frob:tests crates/cadetta-solve/src/results.rs::out_path kind=unit
    #[test]
    fn digests_live_only_in_the_out_file() {
        // The lock file is written by other crates; this module only ever
        // targets `<stem>.out.json` and never a lock path.
        let p = out_path(Path::new("a/b.py"));
        assert!(p.to_string_lossy().ends_with(".out.json"));
        assert!(!p.to_string_lossy().contains("cadetta.lock"));
        let r = req("s", "1mm");
        assert!(input_digest(&r).starts_with("blake3:"));
    }
}
