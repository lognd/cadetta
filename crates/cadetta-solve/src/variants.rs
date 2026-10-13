//! Per-variant solver runs and readings (solvers.md 4.1).
//!
//! Decisions made here (written into solvers.md 4.1): a variant whose input
//! digest equals the primary's needs no result of its own and reads the
//! primary's (the inputs did not change, so nothing is borrowed); variants
//! with equal changed inputs share one solver run, and each gets its own
//! stored entry keyed (solver, variant); a variant with no stored result
//! reads unknown, never the primary's outputs.

// frob:ticket 01M4STANBCA3XCNHR3X1NJKGFE

use std::collections::BTreeMap;

use tracing::{debug, info};

use crate::protocol::{Request, Response};
use crate::results::{CurrentDigests, Lookup, ResultFile, SolveResult, input_digest};

/// A variant's solver request, built from the variant's own evaluated inputs.
#[derive(Clone, PartialEq, Debug)]
pub struct VariantRequest {
    /// The variant key `SLOT=ALT`.
    pub variant: String,
    /// The request this variant's inputs produce.
    pub request: Request,
}

/// One solver run covering every listed variant that shares an input set.
#[derive(Clone, PartialEq, Debug)]
pub struct SolveJob {
    /// The request to send.
    pub request: Request,
    /// Digest of the request's inputs.
    pub input_digest: String,
    /// The variants this run's result is stored for, sorted.
    pub variants: Vec<String>,
}

/// What a variant's outputs read as.
#[derive(Clone, PartialEq, Debug)]
pub enum VariantReading<'a> {
    /// Known outputs, from the variant's own result (or the primary's when inputs are unchanged).
    Outputs(&'a SolveResult),
    /// No result for the variant's inputs: unknown, never the primary's (S006).
    Unknown,
}

/// Plan one run per distinct changed input digest that still lacks a fresh result.
///
/// `base` carries the script and dep digests; its `input_digest` is the primary's.
pub fn plan(
    file: &ResultFile,
    solver: &str,
    base: &CurrentDigests,
    variants: &[VariantRequest],
) -> Vec<SolveJob> {
    let mut jobs: BTreeMap<String, SolveJob> = BTreeMap::new();
    for v in variants {
        let digest = input_digest(&v.request);
        if digest == base.input_digest {
            debug!(solver, variant = %v.variant, "variant inputs equal the primary's, no own result");
            continue;
        }
        let current = CurrentDigests {
            input_digest: digest.clone(),
            ..base.clone()
        };
        if matches!(
            file.lookup(solver, Some(&v.variant), &current),
            Lookup::Fresh(_)
        ) {
            debug!(solver, variant = %v.variant, "variant result is fresh, not running");
            continue;
        }
        let job = jobs.entry(digest.clone()).or_insert_with(|| SolveJob {
            request: v.request.clone(),
            input_digest: digest,
            variants: Vec::new(),
        });
        job.variants.push(v.variant.clone());
    }
    let mut jobs: Vec<SolveJob> = jobs.into_values().collect();
    for j in &mut jobs {
        j.variants.sort();
        j.variants.dedup();
    }
    info!(solver, runs = jobs.len(), "variant solve plan");
    jobs
}

/// Store `response` for every variant of `job`, one entry per (solver, variant).
pub fn store(
    file: &mut ResultFile,
    solver: &str,
    base: &CurrentDigests,
    job: &SolveJob,
    response: &Response,
) {
    for variant in &job.variants {
        info!(solver, %variant, digest = %job.input_digest, "storing variant result");
        file.upsert(SolveResult {
            solver: solver.to_string(),
            input_digest: job.input_digest.clone(),
            script_digest: base.script_digest.clone(),
            dep_digests: base.dep_digests.clone(),
            version: response.version.clone(),
            variant: Some(variant.clone()),
            inputs: job.request.inputs.clone(),
            outputs: response.outputs.clone(),
            diagnostics: response.diagnostics.clone(),
        });
    }
}

/// Read a solver's outputs under `variant`; unknown when it has no result and its inputs changed.
///
/// `variant_digest` is the digest of the variant's own request inputs.
pub fn read_variant<'a>(
    file: &'a ResultFile,
    solver: &str,
    variant: &str,
    base: &CurrentDigests,
    variant_digest: &str,
) -> VariantReading<'a> {
    let (key, digest) = if variant_digest == base.input_digest {
        (None, base.input_digest.as_str())
    } else {
        (Some(variant), variant_digest)
    };
    let current = CurrentDigests {
        input_digest: digest.to_string(),
        ..base.clone()
    };
    match file.lookup(solver, key, &current) {
        Lookup::Fresh(r) | Lookup::Stale(r) => VariantReading::Outputs(r),
        Lookup::Missing => {
            debug!(solver, variant, "no variant result, outputs unknown");
            VariantReading::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::InputValue;
    use crate::results::digest_bytes;

    const S: &str = "acme::drive_torque";

    fn req(lit: &str) -> Request {
        let inputs = BTreeMap::from([(
            "x".to_string(),
            InputValue {
                lit: lit.to_string(),
                si: 1.0,
                min: None,
                max: None,
            },
        )]);
        Request::new(S, inputs)
    }

    fn vr(variant: &str, lit: &str) -> VariantRequest {
        VariantRequest {
            variant: variant.to_string(),
            request: req(lit),
        }
    }

    fn base() -> CurrentDigests {
        CurrentDigests {
            input_digest: input_digest(&req("1mm")),
            script_digest: digest_bytes(b"script"),
            dep_digests: BTreeMap::new(),
        }
    }

    fn response() -> Response {
        Response {
            version: None,
            outputs: BTreeMap::from([("t".to_string(), "9N*m".to_string())]),
            diagnostics: vec![],
        }
    }

    // frob:tests crates/cadetta-solve/src/variants.rs::plan kind=unit
    #[test]
    fn one_result_per_distinct_input_digest() {
        let mut file = ResultFile::default();
        let vars = [
            vr("W=A", "2mm"),
            vr("W=B", "2mm"),
            vr("W=C", "3mm"),
            vr("V=D", "1mm"),
        ];
        let jobs = plan(&file, S, &base(), &vars);
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].variants.len() + jobs[1].variants.len(), 3);
        for j in &jobs {
            store(&mut file, S, &base(), j, &response());
        }
        // Three variant entries (two share a run); the unchanged one has none.
        assert_eq!(file.results.len(), 3);
        assert!(file.results.iter().all(|r| r.variant.is_some()));
        // Fresh entries are not planned again.
        assert!(plan(&file, S, &base(), &vars).is_empty());
    }

    // frob:tests crates/cadetta-solve/src/variants.rs::read_variant kind=unit
    #[test]
    fn missing_variant_result_is_unknown_not_primary() {
        let mut file = ResultFile::default();
        file.upsert(SolveResult {
            solver: S.into(),
            input_digest: base().input_digest,
            script_digest: base().script_digest,
            dep_digests: BTreeMap::new(),
            version: None,
            variant: None,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::from([("t".to_string(), "5N*m".to_string())]),
            diagnostics: vec![],
        });
        let changed = input_digest(&req("2mm"));
        assert_eq!(
            read_variant(&file, S, "W=A", &base(), &changed),
            VariantReading::Unknown
        );
        // Unchanged inputs read the primary's result.
        let same = base().input_digest;
        assert!(matches!(
            read_variant(&file, S, "W=A", &base(), &same),
            VariantReading::Outputs(r) if r.outputs["t"] == "5N*m"
        ));
        let jobs = plan(&file, S, &base(), &[vr("W=A", "2mm")]);
        store(&mut file, S, &base(), &jobs[0], &response());
        assert!(matches!(
            read_variant(&file, S, "W=A", &base(), &changed),
            VariantReading::Outputs(r) if r.outputs["t"] == "9N*m"
        ));
    }
}
