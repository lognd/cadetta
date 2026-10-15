//! frob:ticket 01M4V1X55R20C1PT8B56BCVN48
//! Compare a block's expectations with the facts a run observed.
//!
//! Generic keys have a checker below; any other key is compared for exact
//! equality with the executor's `observed[key]`, and a key nobody observes is a
//! failure, never a skip.

use serde_json::Value;

use crate::compare::compare;
use crate::exec::expected_members;
use crate::expect::Block;
use crate::run::{Facts, parse_json};

/// Keys that are inputs or documentation, not observations.
const IGNORED: &[&str] = &["note", "why", "args"];

/// Compare `facts` with `block`; one message per mismatch.
pub fn verify(block: &Block, facts: &Facts) -> Vec<String> {
    let mut problems = compare(block, &facts.produced, facts.exit);
    problems.extend(check_values(block, facts));
    for (key, want) in &block.extra {
        if IGNORED.contains(&key.as_str()) {
            continue;
        }
        let result = match key.as_str() {
            "output" | "outputs" => check_output(want, facts),
            "writes" => check_writes(want, facts),
            "file_written" => check_flag(want, !written(facts).is_empty(), key),
            "order" => check_order(want, facts),
            "stdout_contains" => check_contains(want, &facts.stdout, "stdout"),
            "stderr_contains" => check_contains(want, &facts.stderr, "stderr"),
            "message_contains" => check_contains(want, &facts.messages.join("\n"), "messages"),
            "message_excludes" => check_excludes(want, &facts.messages.join("\n"), "messages"),
            "results_order" => check_results_order(want, facts),
            "json_diag_schema" => check_diag_schema(want, facts),
            "config_error" => check_flag(want, facts.exit == 2, key),
            "observed" => check_observed_subset(want, facts),
            "excludes" => check_observed_excludes(want, facts),
            _ => check_observed(key, want, facts),
        };
        if let Err(m) = result {
            problems.push(m);
        }
    }
    problems
}

fn check_values(block: &Block, facts: &Facts) -> Vec<String> {
    let wanted = match expected_members(block) {
        Ok(w) => w,
        Err(m) => return vec![m],
    };
    wanted
        .into_iter()
        .filter_map(|(path, want)| match facts.members.get(&path) {
            Some(got) if *got == want => None,
            Some(got) => Some(format!("value {path}: got {got}, expected {want}")),
            None => Some(format!("value {path}: not in the evaluated IR")),
        })
        .collect()
}

/// Paths created or changed by the run.
fn written(facts: &Facts) -> Vec<String> {
    facts
        .after
        .iter()
        .filter(|(p, text)| facts.before.get(*p) != Some(text))
        .map(|(p, _)| p.clone())
        .collect()
}

fn strings(want: &Value) -> Vec<String> {
    match want {
        Value::Array(a) => a
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        Value::String(s) => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn check_flag(want: &Value, got: bool, key: &str) -> Result<(), String> {
    match want.as_bool() {
        Some(w) if w == got => Ok(()),
        Some(w) => Err(format!("{key}: expected {w}, observed {got}")),
        None => Err(format!("{key}: expectation is not a boolean")),
    }
}

fn check_contains(want: &Value, haystack: &str, what: &str) -> Result<(), String> {
    let missing: Vec<String> = strings(want)
        .into_iter()
        .filter(|s| !haystack.contains(s.as_str()))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("{what} lack {missing:?}"))
    }
}

fn check_excludes(want: &Value, haystack: &str, what: &str) -> Result<(), String> {
    let present: Vec<String> = strings(want)
        .into_iter()
        .filter(|s| haystack.contains(s.as_str()))
        .collect();
    if present.is_empty() {
        Ok(())
    } else {
        Err(format!("{what} contain {present:?}"))
    }
}

/// Positions of `needles` in `text`, each searched after the previous hit.
fn in_order(text: &str, needles: &[String]) -> bool {
    let mut from = 0;
    for n in needles {
        match text[from..].find(n.as_str()) {
            Some(i) => from += i + n.len(),
            None => return false,
        }
    }
    true
}

/// The text `order` is checked against: the files the run wrote, else stdout.
fn written_text(facts: &Facts) -> String {
    let files = written(facts);
    if files.is_empty() {
        return facts.stdout.clone();
    }
    files
        .iter()
        .filter_map(|p| facts.after.get(p))
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

fn check_order(want: &Value, facts: &Facts) -> Result<(), String> {
    let needles = strings(want);
    if in_order(&written_text(facts), &needles) {
        Ok(())
    } else {
        Err(format!("order: {needles:?} not found in this order"))
    }
}

fn check_writes(want: &Value, facts: &Facts) -> Result<(), String> {
    let mut wanted = strings(want);
    wanted.sort();
    let got = written(facts);
    if got == wanted {
        Ok(())
    } else {
        Err(format!("writes: observed {got:?}, expected {wanted:?}"))
    }
}

/// `output`/`outputs`: per file `contains`, `excludes`, `order`, `order_not`,
/// `ends_with`, `exists` and `unchanged`.
fn check_output(want: &Value, facts: &Facts) -> Result<(), String> {
    let Some(files) = want.as_object() else {
        return Err("output: expectation is not an object".into());
    };
    let mut problems = Vec::new();
    // A flat `contains`/`excludes` checks the entry's printed plan (`stdout`), not a file.
    if want.get("contains").is_some_and(Value::is_array)
        || want.get("excludes").is_some_and(Value::is_array)
    {
        if let Err(m) = check_contains(&want["contains"], &facts.stdout, "stdout") {
            problems.push(m);
        }
        if let Err(m) = check_excludes(&want["excludes"], &facts.stdout, "stdout") {
            problems.push(m);
        }
        return if problems.is_empty() {
            Ok(())
        } else {
            Err(format!("output: {}", problems.join("; ")))
        };
    }
    for (path, spec) in files {
        let text = facts.after.get(path);
        let wants_exists = spec["exists"].as_bool();
        if wants_exists == Some(false) {
            if text.is_some() {
                problems.push(format!("{path}: exists but should not"));
            }
            continue;
        }
        let Some(text) = text else {
            problems.push(format!("{path}: was not written"));
            continue;
        };
        if let Err(m) = check_contains(&spec["contains"], text, path) {
            problems.push(m);
        }
        if let Err(m) = check_excludes(&spec["excludes"], text, path) {
            problems.push(m);
        }
        if spec.get("order").is_some() && !in_order(text, &strings(&spec["order"])) {
            problems.push(format!("{path}: lines not in the expected order"));
        }
        if spec.get("order_not").is_some() && in_order(text, &strings(&spec["order_not"])) {
            problems.push(format!("{path}: lines appear in the forbidden order"));
        }
        if let Some(tail) = spec["ends_with"].as_str()
            && !text.ends_with(tail)
        {
            problems.push(format!("{path}: does not end with {tail:?}"));
        }
        if spec["unchanged"].as_bool() == Some(true) && facts.before.get(path) != Some(text) {
            problems.push(format!("{path}: changed but should be unchanged"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!("output: {}", problems.join("; ")))
    }
}

fn check_observed(key: &str, want: &Value, facts: &Facts) -> Result<(), String> {
    match facts.observed.get(key) {
        Some(got) if got == want => Ok(()),
        Some(got) => Err(format!("{key}: observed {got}, expected {want}")),
        None => Err(format!("this entry does not observe `{key}`")),
    }
}

/// `observed`: the expectation is a partial object; every wanted key must match recursively
/// and a wanted `{}` needs an empty actual object (reads, `binding` entry).
// frob:ticket 01M4VQXM0V12H5JBPHWW796HD7
fn check_observed_subset(want: &Value, facts: &Facts) -> Result<(), String> {
    let Some(got) = facts.observed.get("observed") else {
        return Err("this entry does not observe `observed`".into());
    };
    if subset(want, got) {
        Ok(())
    } else {
        Err(format!("observed: got {got}, expected (subset) {want}"))
    }
}

/// True when `want` is contained in `got`: objects by key, scalars and arrays by equality.
fn subset(want: &Value, got: &Value) -> bool {
    match (want, got) {
        (Value::Object(w), Value::Object(g)) => {
            (!w.is_empty() || g.is_empty())
                && w.iter()
                    .all(|(k, v)| g.get(k).is_some_and(|x| subset(v, x)))
        }
        _ => want == got,
    }
}

/// `excludes`: none of the strings may be a key or a string value anywhere in `observed`.
// frob:ticket 01M4VQXM0V12H5JBPHWW796HD7
fn check_observed_excludes(want: &Value, facts: &Facts) -> Result<(), String> {
    let got = facts.observed.get("observed").unwrap_or(&Value::Null);
    let mut tokens = Vec::new();
    collect_tokens(got, &mut tokens);
    let present: Vec<String> = strings(want)
        .into_iter()
        .filter(|s| tokens.contains(s))
        .collect();
    if present.is_empty() {
        Ok(())
    } else {
        Err(format!("observed contains excluded {present:?}"))
    }
}

/// Every key and string leaf of `v`.
fn collect_tokens(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                out.push(k.clone());
                collect_tokens(x, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| collect_tokens(x, out)),
        Value::String(s) => out.push(s.clone()),
        _ => {}
    }
}

/// Every record of the report's `diagnostics` list carries the wanted `schema` tag.
fn check_diag_schema(want: &Value, facts: &Facts) -> Result<(), String> {
    let report = parse_json(&facts.stdout).ok_or("json_diag_schema: stdout is not JSON")?;
    let list = report["diagnostics"]
        .as_array()
        .ok_or("json_diag_schema: report has no `diagnostics` list")?;
    match list.iter().find(|d| &d["schema"] != want) {
        Some(d) => Err(format!("json_diag_schema: record {d} lacks schema {want}")),
        None => Ok(()),
    }
}

/// `results_order`: the solver ids appear in this order in the files the run wrote.
fn check_results_order(want: &Value, facts: &Facts) -> Result<(), String> {
    let ids = strings(want);
    if ids.is_empty() {
        return Err("results_order: expectation lists no solver ids".into());
    }
    if in_order(&written_text(facts), &ids) {
        Ok(())
    } else {
        Err(format!("results_order: {ids:?} not found in this order"))
    }
}
