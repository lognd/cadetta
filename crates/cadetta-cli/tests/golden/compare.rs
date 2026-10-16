//! frob:ticket 01M4SXSGQQHR6RMBNJQJ3392AZ
//! Compare a case's observations with its `expected.toml`.

use std::collections::BTreeMap;
use std::path::Path;

use cadetta_units::UnitTable;
use regex::Regex;
use serde_json::Value as Json;

use crate::actual::{Actual, produce};
use crate::expect::{ArtifactExp, Expected};
use crate::quantity::{Exact, parse_literal, range_of};

/// Which side is probably wrong when a hand-evaluated expectation disagrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suspect {
    /// The implementation (the case's run is not clean, or a known bug explains it).
    Implementation,
    /// The hand evaluation (the case's run is clean and still disagrees).
    HandEvaluation,
}

/// The result of one expectation.
#[derive(Debug)]
pub enum Finding {
    /// Equal.
    Match,
    /// Not equal.
    Mismatch(Suspect, String),
    /// Could not be compared.
    Unevaluable(String),
}

/// Counts of findings in one section.
#[derive(Debug, Default, Clone, Copy)]
pub struct Tally {
    /// Equal.
    pub matched: usize,
    /// Not equal.
    pub mismatched: usize,
    /// Not comparable yet.
    pub unevaluable: usize,
}

impl Tally {
    /// Add a finding.
    pub fn add(&mut self, f: &Finding) {
        match f {
            Finding::Match => self.matched += 1,
            Finding::Mismatch(..) => self.mismatched += 1,
            Finding::Unevaluable(_) => self.unevaluable += 1,
        }
    }
}

/// Everything compared for one case.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Values.
    pub values: Tally,
    /// Verdicts.
    pub verdicts: Tally,
    /// Diagnostics.
    pub diagnostics: Tally,
    /// Artifacts.
    pub artifacts: Tally,
    /// Mismatch and unevaluable detail lines.
    pub details: Vec<String>,
    /// Expectations found only through the `Rover.wheel` path form (~137W4EE).
    pub aliased: usize,
    /// Mismatches the implementation is suspected of.
    pub impl_suspect: usize,
    /// Mismatches the hand evaluation is suspected of.
    pub hand_suspect: usize,
}

impl Outcome {
    /// Mismatches in any section.
    pub fn mismatches(&self) -> usize {
        self.values.mismatched
            + self.verdicts.mismatched
            + self.diagnostics.mismatched
            + self.artifacts.mismatched
    }

    /// Unevaluable expectations in any section.
    pub fn unevaluable(&self) -> usize {
        self.values.unevaluable
            + self.verdicts.unevaluable
            + self.diagnostics.unevaluable
            + self.artifacts.unevaluable
    }

    fn record(&mut self, section: &str, what: &str, f: Finding) {
        match &f {
            Finding::Match => {}
            Finding::Mismatch(s, d) => {
                match s {
                    Suspect::Implementation => self.impl_suspect += 1,
                    Suspect::HandEvaluation => self.hand_suspect += 1,
                }
                self.details
                    .push(format!("  MISMATCH [{s:?} suspect] {section} {what}: {d}"));
            }
            Finding::Unevaluable(d) if d.starts_with("`cadetta ir`") => {}
            Finding::Unevaluable(d) => self
                .details
                .push(format!("  unevaluable {section} {what}: {d}")),
        }
        match section {
            "value" => self.values.add(&f),
            "verdict" => self.verdicts.add(&f),
            "diagnostic" => self.diagnostics.add(&f),
            _ => self.artifacts.add(&f),
        }
    }
}

/// `Rover/wheel` as sema writes it today: the first `/` becomes `.` (~137W4EE).
fn alias(path: &str) -> Option<String> {
    path.contains('/').then(|| path.replacen('/', ".", 1))
}

fn find<'a, T>(map: &'a BTreeMap<String, T>, path: &str) -> Option<(&'a T, bool)> {
    map.get(path)
        .map(|v| (v, false))
        .or_else(|| alias(path).and_then(|a| map.get(&a)).map(|v| (v, true)))
}

fn compare_exact(label: &str, want: &Exact, got: &Exact, suspect: Suspect) -> Option<Finding> {
    (want != got).then(|| Finding::Mismatch(suspect, format!("{label} expected {want}, got {got}")))
}

/// Compare everything against `expected`; `case_root` is the scratch copy, `source` the committed case.
pub fn compare(expected: &Expected, actual: &Actual, case_root: &Path, source: &Path) -> Outcome {
    let units = UnitTable::builtin();
    let mut out = Outcome::default();
    let suspect = if actual.clean() {
        Suspect::HandEvaluation
    } else {
        Suspect::Implementation
    };

    let mut members: BTreeMap<String, &Json> = BTreeMap::new();
    let mut checks: BTreeMap<String, &Json> = BTreeMap::new();
    if let Ok(ir) = &actual.ir {
        for c in ir
            .get("components")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
        {
            for m in c
                .get("members")
                .and_then(Json::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(p) = m.get("path").and_then(Json::as_str) {
                    members.insert(p.to_owned(), m);
                }
            }
        }
        for c in ir
            .get("checks")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(p) = c.get("path").and_then(Json::as_str) {
                checks.insert(p.to_owned(), c);
            }
        }
    }
    let ir_err = actual.ir.as_ref().err().cloned();

    for v in &expected.values {
        let what = v.path.clone();
        let f = (|| {
            if let Some(e) = &ir_err {
                return Finding::Unevaluable(e.clone());
            }
            let Some((member, aliased)) = find(&members, &v.path) else {
                return Finding::Unevaluable("path not in the IR".into());
            };
            let value = match &v.variant {
                None => member.get("value"),
                Some(k) => member.get("variants").and_then(|m| m.get(k)),
            };
            let Some(value) = value else {
                return Finding::Unevaluable("no value for this member/variant in the IR".into());
            };
            let got = match range_of(value) {
                Ok(r) => r,
                Err(e) => return Finding::Unevaluable(e),
            };
            let mut wants = vec![("nominal", &v.nominal, &got.nominal)];
            if let Some(m) = &v.min {
                wants.push(("min", m, &got.min));
            }
            if let Some(m) = &v.max {
                wants.push(("max", m, &got.max));
            }
            for (label, text, have) in wants {
                match parse_literal(&units, text) {
                    Err(e) => return Finding::Unevaluable(e),
                    Ok(want) => {
                        if let Some(f) = compare_exact(label, &want, have, suspect) {
                            return f;
                        }
                    }
                }
            }
            if aliased {
                out.aliased += 1;
            }
            Finding::Match
        })();
        out.record("value", &what, f);
    }

    for v in &expected.verdicts {
        let what = v.path.clone();
        let f = (|| {
            if let Some(e) = &ir_err {
                return Finding::Unevaluable(e.clone());
            }
            let Some((check, aliased)) = find(&checks, &v.path) else {
                return Finding::Unevaluable("check not in the IR".into());
            };
            let state = match &v.variant {
                None => check.pointer("/verdicts/primary"),
                Some(k) => check
                    .pointer("/verdicts/per_alternate")
                    .and_then(|m| m.get(k)),
            }
            .and_then(Json::as_str);
            let Some(state) = state else {
                return Finding::Unevaluable("no verdict for this variant in the IR".into());
            };
            let got = match state {
                "pass" => "holds",
                "fail" => "violates",
                "unknown" => "undecided",
                other => other,
            };
            if got != v.result {
                return Finding::Mismatch(suspect, format!("expected {}, got {got}", v.result));
            }
            if let Some(c) = v.conservative
                && check.get("conservative").and_then(Json::as_bool) != Some(c)
            {
                return Finding::Mismatch(suspect, format!("conservative expected {c}"));
            }
            if aliased {
                out.aliased += 1;
            }
            Finding::Match
        })();
        out.record("verdict", &what, f);
    }

    diagnostics(expected, actual, suspect, &mut out);

    for a in &expected.artifacts {
        let f = artifact(a, case_root, source, suspect);
        out.record("artifact", &a.expected, f);
    }
    out
}

fn diagnostics(expected: &Expected, actual: &Actual, suspect: Suspect, out: &mut Outcome) {
    let diags = match (&actual.diags, &actual.ir) {
        (Ok(d), Ok(_)) => d,
        (Err(e), _) | (_, Err(e)) => {
            for d in &expected.diagnostics {
                out.record("diagnostic", &d.code, Finding::Unevaluable(e.clone()));
            }
            return;
        }
    };
    let mut used = vec![false; diags.len()];
    for want in &expected.diagnostics {
        let hit = diags.iter().enumerate().find(|(i, d)| {
            // each actual diagnostic satisfies at most one expectation, so repeats need repeated entries
            !used[*i]
                && !d.waived
                && d.code == want.code
                && want
                    .path
                    .as_ref()
                    .is_none_or(|p| d.path.as_ref() == Some(p))
                && want
                    .variant
                    .as_ref()
                    .is_none_or(|v| d.alt.as_ref() == Some(v))
        });
        let what = format!("{} {}", want.code, want.path.as_deref().unwrap_or(""));
        let f = match hit {
            Some((i, _)) => {
                used[i] = true;
                Finding::Match
            }
            None => Finding::Mismatch(suspect, "expected diagnostic was not produced".into()),
        };
        out.record("diagnostic", &what, f);
    }
    for (i, d) in diags.iter().enumerate() {
        let closed = matches!(d.severity.as_str(), "error" | "warning");
        if closed && !d.waived && !used[i] {
            let what = format!("{} {}", d.code, d.path.as_deref().unwrap_or(""));
            let f = Finding::Mismatch(suspect, format!("unexpected {}", d.severity));
            out.record("diagnostic", &what, f);
        }
    }
}

/// Replace every mask match by `<masked>`.
pub fn masked(text: &[u8], masks: &[Regex]) -> Vec<u8> {
    let mut s = String::from_utf8_lossy(text).into_owned();
    for re in masks {
        s = re.replace_all(&s, "<masked>").into_owned();
    }
    s.into_bytes()
}

/// Compile an artifact's mask patterns.
pub fn mask_regexes(a: &ArtifactExp) -> Result<Vec<Regex>, String> {
    a.mask
        .iter()
        .map(|m| Regex::new(m).map_err(|e| format!("mask `{m}`: {e}")))
        .collect()
}

fn artifact(a: &ArtifactExp, root: &Path, source: &Path, suspect: Suspect) -> Finding {
    let masks = match mask_regexes(a) {
        Ok(m) => m,
        Err(e) => return Finding::Unevaluable(e),
    };
    let want = match std::fs::read(source.join(&a.expected)) {
        Ok(b) => b,
        Err(e) => return Finding::Unevaluable(format!("{}: {e}", a.expected)),
    };
    let got = match produce(root, &a.command, &a.produces) {
        Ok(b) => b,
        Err(e) => return Finding::Unevaluable(e),
    };
    if masked(&want, &masks) == masked(&got, &masks) {
        Finding::Match
    } else {
        Finding::Mismatch(
            suspect,
            format!("{} differs from {}", a.produces, a.expected),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actual::{Diag, split_command};
    use crate::expect::parse;

    fn actual_with(diags: Vec<Diag>) -> Actual {
        Actual {
            ir: Ok(serde_json::json!({"components": [], "checks": []})),
            diags: Ok(diags),
        }
    }

    fn diag(code: &str, severity: &str) -> Diag {
        Diag {
            code: code.into(),
            severity: severity.into(),
            path: None,
            alt: None,
            waived: false,
        }
    }

    // frob:tests crates/cadetta-cli/tests/golden/compare.rs::diagnostics_are_closed_for_errors_and_warnings
    #[test]
    fn diagnostics_are_closed_for_errors_and_warnings() {
        let exp = parse(
            "[case]\nspec=\"1.5\"\nstatus=\"hand-evaluated\"\n[[diagnostic]]\ncode=\"C002\"\n",
        )
        .expect("parse");
        let root = Path::new(".");
        let ok = compare(
            &exp,
            &actual_with(vec![diag("C002", "warning"), diag("I001", "info")]),
            root,
            root,
        );
        assert_eq!((ok.diagnostics.matched, ok.diagnostics.mismatched), (1, 0));
        let extra = compare(
            &exp,
            &actual_with(vec![diag("C002", "warning"), diag("E999", "error")]),
            root,
            root,
        );
        assert_eq!(extra.diagnostics.mismatched, 1);
        assert_eq!(
            extra.hand_suspect, 0,
            "an error makes the run unclean, so the implementation is suspect"
        );
    }

    // frob:tests crates/cadetta-cli/tests/golden/compare.rs::mismatch_names_the_suspect_side
    #[test]
    fn mismatch_names_the_suspect_side() {
        let toml = "[case]\nspec=\"1.5\"\nstatus=\"hand-evaluated\"\n[[verdict]]\npath=\"p::C.ok\"\nresult=\"violates\"\n";
        let exp = parse(toml).expect("parse");
        let ir = serde_json::json!({"components": [], "checks": [
            {"path": "p::C.ok", "verdicts": {"primary": "pass", "per_alternate": {}}, "conservative": false}]});
        let clean = Actual {
            ir: Ok(ir.clone()),
            diags: Ok(vec![]),
        };
        let o = compare(&exp, &clean, Path::new("."), Path::new("."));
        assert_eq!(
            (o.verdicts.mismatched, o.hand_suspect, o.impl_suspect),
            (1, 1, 0)
        );
        let dirty = Actual {
            ir: Ok(ir),
            diags: Ok(vec![diag("E002", "error")]),
        };
        let o = compare(&exp, &dirty, Path::new("."), Path::new("."));
        assert_eq!(
            (o.verdicts.mismatched, o.hand_suspect, o.impl_suspect),
            (1, 0, 2)
        );
    }

    // frob:tests crates/cadetta-cli/tests/golden/compare.rs::masks_replace_matches_in_both_files
    #[test]
    fn masks_replace_matches_in_both_files() {
        let re = vec![Regex::new(r"blake3:\S+").expect("re")];
        assert_eq!(
            masked(b"a blake3:abc b", &re),
            masked(b"a blake3:TBD b", &re)
        );
        assert_ne!(
            masked(b"a blake3:abc b", &re),
            masked(b"a blake3:abc c", &re)
        );
        assert_eq!(masked(b"x blake3:1", &re), b"x <masked>".to_vec());
    }

    // frob:tests crates/cadetta-cli/tests/golden/compare.rs::artifact_command_redirect_is_split
    #[test]
    fn artifact_command_redirect_is_split() {
        let (args, redirect) =
            split_command("cadetta impact --swap A B --format json > build/swap.json")
                .expect("split");
        assert_eq!(args, ["impact", "--swap", "A", "B", "--format", "json"]);
        assert_eq!(redirect.as_deref(), Some("build/swap.json"));
        assert!(split_command("make all").is_err());
    }
}
