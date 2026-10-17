//! frob:ticket 01M51SNBKJG1ZCJYKZ6VD4Q7XX
//! `update` and `obligations` entries: lock contents and obligation rows (packages.md 1, 6).
//!
//! Both stage the fixture package, run the CLI and observe the project's `cadetta.lock` and the
//! `cadetta.obligations/1` rows. The verifier compares `lock` and `rows` with [`check_lock`] and
//! [`check_rows`]; `lock_written` and `digest_changed` are compared for equality as observed.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::Job;
use crate::compare::Produced;
use crate::run::{Facts, parse_json, run_cli, stage};

/// The lock file, relative to the project root.
const LOCK: &str = "cadetta.lock";

/// The `package` records of lock text as name to `{name, version, source, digest}`.
fn package_records(text: &str) -> BTreeMap<String, Value> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            (words.next() == Some("package")).then_some(())?;
            let name = words.next()?.to_owned();
            let mut rec = json!({ "name": name });
            for w in words {
                if let Some((k, v)) = w.split_once('=') {
                    rec[k] = json!(v);
                }
            }
            Some((name, rec))
        })
        .collect()
}

/// An error code (`E` and three digits) leading the message of an `error:` line.
fn error_code(line: &str) -> Option<String> {
    let word = line.strip_prefix("error: ")?.split_whitespace().next()?;
    let digits = word.strip_prefix('E')?.trim_end_matches(':');
    (digits.len() == 3 && digits.bytes().all(|b| b.is_ascii_digit())).then(|| word.to_owned())
}

/// Diagnostics the CLI reports on stderr (`update` has no diagnostics report).
fn stderr_diagnostics(stderr: &str) -> Vec<Produced> {
    stderr
        .lines()
        .filter_map(error_code)
        .map(|code| Produced {
            code: code.trim_end_matches(':').to_owned(),
            severity: "error".into(),
            path: None,
        })
        .collect()
}

/// `update`: run `cadetta update [NAME]` and observe the lock before and after.
// frob:tests crates/cadetta-cli/tests/conformance/lockfix.rs::package_records
pub fn update(job: &Job) -> Result<Facts, String> {
    let scratch = stage(job.fixture)?;
    let mut args = job.entry.clone();
    if let Some(extra) = job.block.extra.get("args").and_then(Value::as_array) {
        args.extend(extra.iter().filter_map(|a| a.as_str().map(str::to_owned)));
    }
    let mut facts = run_cli(&scratch.0, &args)?;
    if facts.produced.is_empty() {
        facts.produced = stderr_diagnostics(&facts.stderr);
    }
    let before = package_records(facts.before.get(LOCK).map_or("", String::as_str));
    let after = package_records(facts.after.get(LOCK).map_or("", String::as_str));
    let changed: Vec<&String> = after
        .iter()
        .filter(|(n, rec)| before.get(*n).is_none_or(|b| b["digest"] != rec["digest"]))
        .map(|(n, _)| n)
        .collect();
    tracing::debug!(?changed, "lock digests differing from the start");
    facts.observed.insert(
        "lock_written".into(),
        json!(facts.before.get(LOCK) != facts.after.get(LOCK)),
    );
    facts
        .observed
        .insert("digest_changed".into(), json!(changed));
    facts
        .observed
        .insert("lock".into(), json!(after.values().collect::<Vec<_>>()));
    Ok(facts)
}

/// Split `PACKAGE ROLE by WHO on DATE` into its four fields.
fn upstream_parts(text: &str) -> Value {
    let w: Vec<&str> = text.split_whitespace().collect();
    match w.as_slice() {
        [package, role, "by", by, "on", date] => {
            json!({ "package": package, "role": role, "by": by, "date": date })
        }
        _ => json!({ "raw": text }),
    }
}

/// `obligations`: run `cadetta obligations` and observe its rows.
// frob:tests crates/cadetta-cli/tests/conformance/lockfix.rs::upstream_parts
pub fn obligations(job: &Job) -> Result<Facts, String> {
    let scratch = stage(job.fixture)?;
    let mut args = job.entry.clone();
    if let Some(extra) = job.block.extra.get("args").and_then(Value::as_array) {
        args.extend(extra.iter().filter_map(|a| a.as_str().map(str::to_owned)));
    }
    let mut facts = run_cli(&scratch.0, &args)?;
    let report = parse_json(&facts.stdout).ok_or("`cadetta obligations` printed no JSON")?;
    let rows: Vec<Value> = report["obligations"]
        .as_array()
        .ok_or("obligations report has no `obligations` list")?
        .iter()
        .map(|o| {
            let upstream: Vec<Value> = o["upstream"]
                .as_array()
                .map(|u| {
                    u.iter()
                        .filter_map(Value::as_str)
                        .map(upstream_parts)
                        .collect()
                })
                .unwrap_or_default();
            json!({ "id": o["id"], "role": o["role"], "upstream": upstream })
        })
        .collect();
    facts.observed.insert("rows".into(), json!(rows));
    Ok(facts)
}

/// `lock`: every wanted package record holds its listed fields; `absent` names have none.
pub fn check_lock(want: &Value, facts: &Facts) -> Result<(), String> {
    let got = facts.observed.get("lock").and_then(Value::as_array);
    let got = got.ok_or("this entry does not observe `lock`")?;
    let find = |n: &str| got.iter().find(|r| r["name"] == n);
    let mut problems = Vec::new();
    for pkg in want["packages"].as_array().into_iter().flatten() {
        let name = pkg["name"].as_str().unwrap_or_default();
        match find(name) {
            None => problems.push(format!("no lock record for `{name}`")),
            Some(rec) => {
                for (k, v) in pkg.as_object().into_iter().flatten() {
                    if rec[k] != *v {
                        problems.push(format!("`{name}` {k}: got {}, expected {v}", rec[k]));
                    }
                }
            }
        }
    }
    for name in want["absent"].as_array().into_iter().flatten() {
        if find(name.as_str().unwrap_or_default()).is_some() {
            problems.push(format!("lock still records {name}"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!("lock: {}", problems.join("; ")))
    }
}

/// `rows`: each wanted row is listed (with its role and upstream) or, when `listed` is false, absent.
pub fn check_rows(want: &Value, facts: &Facts) -> Result<(), String> {
    let got = facts.observed.get("rows").and_then(Value::as_array);
    let got = got.ok_or("this entry does not observe `rows`")?;
    let mut problems = Vec::new();
    for row in want.as_array().into_iter().flatten() {
        let id = row["id"].as_str().unwrap_or_default();
        let listed = row["listed"].as_bool().unwrap_or(true);
        match (got.iter().find(|r| r["id"] == id), listed) {
            (Some(_), false) => problems.push(format!("{id}: listed but should be discharged")),
            (None, true) => problems.push(format!("{id}: not listed")),
            (Some(r), true) => {
                if row.get("role").is_some_and(|w| *w != r["role"]) {
                    problems.push(format!(
                        "{id}: role {}, expected {}",
                        r["role"], row["role"]
                    ));
                }
                if let Some(up) = row.get("upstream")
                    && !r["upstream"].as_array().is_some_and(|l| l.contains(up))
                {
                    problems.push(format!("{id}: upstream {}, expected {up}", r["upstream"]));
                }
            }
            (None, false) => {}
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!("rows: {}", problems.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests crates/cadetta-cli/tests/conformance/lockfix.rs::package_records
    #[test]
    fn package_lines_become_records() {
        let r = package_records("attest x\npackage a.b version=1.0.0 source=path:b digest=ab\n");
        assert_eq!(r["a.b"]["source"], "path:b");
        assert_eq!(r.len(), 1);
    }

    // frob:tests crates/cadetta-cli/tests/conformance/lockfix.rs::upstream_parts
    #[test]
    fn upstream_text_is_split() {
        let u = upstream_parts("acme.shop review by a@x.org on 2026-10-13");
        assert_eq!(u["by"], "a@x.org");
        assert_eq!(u["date"], "2026-10-13");
    }

    // frob:tests crates/cadetta-cli/tests/conformance/lockfix.rs::error_code
    #[test]
    fn error_lines_yield_codes() {
        assert_eq!(
            error_code("error: E012 version-unsatisfied: x").as_deref(),
            Some("E012")
        );
        assert_eq!(error_code("error: `x` is not a dependency"), None);
    }
}
