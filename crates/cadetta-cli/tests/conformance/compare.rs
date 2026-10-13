//! frob:ticket 01M4V0NSCWCW8W0P7DG628M8AP
//! Diagnostic matching under the closed-list rules of the README.

use crate::expect::{Block, ExpectedDiag};

/// A diagnostic the run produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Produced {
    /// Diagnostic code.
    pub code: String,
    /// Lowercase severity word.
    pub severity: String,
    /// Canonical path, if any.
    pub path: Option<String>,
}

fn describe(code: &str, severity: &str, path: &Option<String>) -> String {
    match path {
        Some(p) => format!("{code} ({severity}) at {p}"),
        None => format!("{code} ({severity})"),
    }
}

/// The exit status a block expects: explicit, else nonzero iff an error is listed.
fn exit_ok(block: &Block, exit: i32) -> Result<(), String> {
    match block.exit {
        Some(want) if want != exit => Err(format!("exit status {exit}, expected {want}")),
        Some(_) => Ok(()),
        None => {
            let wants_error = block.diagnostics.iter().any(|d| d.severity == "error");
            match (wants_error, exit) {
                (true, 0) => Err("exit status 0, expected nonzero".into()),
                (false, 0) => Ok(()),
                (false, n) => Err(format!("exit status {n}, expected 0")),
                (true, _) => Ok(()),
            }
        }
    }
}

/// Compare `produced` and `exit` with `block`; returns one message per mismatch.
///
/// Errors and warnings are a closed list; info is compared only when listed.
/// Order is irrelevant and duplicates count.
pub fn compare(block: &Block, produced: &[Produced], exit: i32) -> Vec<String> {
    let mut problems = Vec::new();
    let mut used = vec![false; produced.len()];
    let mut unmatched: Vec<&ExpectedDiag> = Vec::new();
    // Path-bearing expectations claim their exact match first.
    let mut order: Vec<&ExpectedDiag> = block.diagnostics.iter().collect();
    order.sort_by_key(|d| d.path.is_none());
    for want in order {
        let hit = produced.iter().enumerate().position(|(i, p)| {
            !used[i]
                && p.code == want.code
                && p.severity == want.severity
                && want
                    .path
                    .as_ref()
                    .is_none_or(|w| p.path.as_ref() == Some(w))
        });
        match hit {
            Some(i) => used[i] = true,
            None => unmatched.push(want),
        }
    }
    for want in unmatched {
        problems.push(format!(
            "missing {}",
            describe(&want.code, &want.severity, &want.path)
        ));
    }
    for (i, p) in produced.iter().enumerate() {
        if !used[i] && (p.severity == "error" || p.severity == "warning") {
            problems.push(format!("extra {}", describe(&p.code, &p.severity, &p.path)));
        }
    }
    if let Err(m) = exit_ok(block, exit) {
        problems.push(m);
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn want(code: &str, sev: &str, path: Option<&str>) -> ExpectedDiag {
        ExpectedDiag {
            code: code.into(),
            severity: sev.into(),
            path: path.map(str::to_owned),
        }
    }
    fn got(code: &str, sev: &str, path: Option<&str>) -> Produced {
        Produced {
            code: code.into(),
            severity: sev.into(),
            path: path.map(str::to_owned),
        }
    }

    #[test]
    fn closed_list_and_duplicates() {
        let block = Block {
            diagnostics: vec![want("Q006", "warning", None), want("Q006", "warning", None)],
            ..Block::default()
        };
        let one = [got("Q006", "warning", Some("a::B.c"))];
        assert_eq!(compare(&block, &one, 0), ["missing Q006 (warning)"]);
        let extra = [
            got("Q006", "warning", None),
            got("Q006", "warning", None),
            got("W004", "warning", None),
            got("N001", "info", None),
        ];
        assert_eq!(compare(&block, &extra, 0), ["extra W004 (warning)"]);
    }

    #[test]
    fn path_must_match_when_given_and_exit_defaults() {
        let block = Block {
            diagnostics: vec![want("C001", "error", Some("a::B.x"))],
            ..Block::default()
        };
        let wrong = [got("C001", "error", Some("a::B.y"))];
        let msgs = compare(&block, &wrong, 1);
        assert_eq!(msgs.len(), 2, "{msgs:?}");
        assert!(compare(&block, &[got("C001", "error", Some("a::B.x"))], 1).is_empty());
        assert_eq!(
            compare(&block, &[got("C001", "error", Some("a::B.x"))], 0),
            ["exit status 0, expected nonzero"]
        );
    }
}
