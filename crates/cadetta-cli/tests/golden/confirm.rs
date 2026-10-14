//! frob:ticket 01M4SXSGQQHR6RMBNJQJ3392AZ
//! Confirmation (opt-in): adopt the digests the implementation computes and mark a case confirmed.

use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::actual::{Scratch, copy_dir, run_cli};
use crate::compare::masked;

/// The placeholder digest in hand-evaluated files.
pub const TBD: &str = "blake3:TBD";

/// Every file under `dir` that holds `blake3:TBD`.
pub fn tbd_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return found;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            found.extend(tbd_files(&p));
        } else if fs::read_to_string(&p).is_ok_and(|t| t.contains(TBD)) {
            found.push(p);
        }
    }
    found.sort();
    found
}

/// Accept `regenerated` for a committed file only when it differs by digests alone.
pub fn adopt_digests(committed: &str, regenerated: &str) -> Result<String, String> {
    let digest = Regex::new(r"blake3:\S+").map_err(|e| e.to_string())?;
    let (a, b) = (
        masked(committed.as_bytes(), std::slice::from_ref(&digest)),
        masked(regenerated.as_bytes(), std::slice::from_ref(&digest)),
    );
    if a != b {
        return Err("regenerated file differs beyond digests".into());
    }
    if regenerated.contains(TBD) {
        return Err("regenerated file still carries blake3:TBD".into());
    }
    Ok(regenerated.to_owned())
}

/// Rewrite `status = "hand-evaluated"` as `confirmed`; other text is untouched.
pub fn set_confirmed(expected_toml: &str) -> Result<String, String> {
    let re = Regex::new(r#"(?m)^(status\s*=\s*)"hand-evaluated""#).map_err(|e| e.to_string())?;
    if !re.is_match(expected_toml) {
        return Err("no `status = \"hand-evaluated\"` line".into());
    }
    Ok(re.replace(expected_toml, r#"${1}"confirmed""#).into_owned())
}

/// Run `cadetta solve` in a scratch copy of the case.
pub fn solve_in(root: &Path) -> Result<(), String> {
    let run = run_cli(root, &["solve".into()])?;
    if run.exit == 0 {
        Ok(())
    } else {
        Err(format!("`cadetta solve` exit {}", run.exit))
    }
}

/// Confirm the case at `case`: `regenerate` fills a scratch copy, digests are adopted, status flipped.
pub fn confirm_with(
    case: &Path,
    regenerate: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<Vec<PathBuf>, String> {
    let targets = tbd_files(case);
    let scratch = Scratch::new()?;
    copy_dir(case, &scratch.0)?;
    regenerate(&scratch.0)?;
    let mut writes = Vec::new();
    for t in &targets {
        let rel = t.strip_prefix(case).map_err(|e| e.to_string())?;
        let fresh = fs::read_to_string(scratch.0.join(rel))
            .map_err(|e| format!("{}: {e}", rel.display()))?;
        let old = fs::read_to_string(t).map_err(|e| e.to_string())?;
        writes.push((
            t.clone(),
            adopt_digests(&old, &fresh).map_err(|e| format!("{}: {e}", rel.display()))?,
        ));
    }
    let toml_path = case.join("expected.toml");
    let toml = set_confirmed(&fs::read_to_string(&toml_path).map_err(|e| e.to_string())?)?;
    writes.push((toml_path, toml));
    let mut done = Vec::new();
    for (path, text) in writes {
        fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        tracing::info!(file = %path.display(), "golden: confirmed write");
        done.push(path);
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests crates/cadetta-cli/tests/golden/confirm.rs::adopt_requires_digest_only_difference
    #[test]
    fn adopt_requires_digest_only_difference() {
        let old = "{\"d\": \"blake3:TBD\", \"x\": 1}";
        assert!(adopt_digests(old, "{\"d\": \"blake3:ab12\", \"x\": 1}").is_ok());
        assert!(adopt_digests(old, "{\"d\": \"blake3:ab12\", \"x\": 2}").is_err());
        assert!(adopt_digests(old, old).is_err());
    }

    // frob:tests crates/cadetta-cli/tests/golden/confirm.rs::confirm_writes_digests_and_status_on_a_temp_copy
    #[test]
    fn confirm_writes_digests_and_status_on_a_temp_copy() {
        let tmp = Scratch::new().expect("scratch");
        let case = tmp.0.join("case");
        fs::create_dir_all(case.join("analysis")).expect("mkdir");
        fs::write(
            case.join("expected.toml"),
            "[case]\nspec = \"1.5\"\nstatus = \"hand-evaluated\"\n",
        )
        .expect("w");
        fs::write(case.join("analysis/r.json"), "{\"d\": \"blake3:TBD\"}").expect("w");
        let done = confirm_with(&case, |root| {
            fs::write(root.join("analysis/r.json"), "{\"d\": \"blake3:ff00\"}")
                .map_err(|e| e.to_string())
        })
        .expect("confirm");
        assert_eq!(done.len(), 2);
        assert_eq!(
            fs::read_to_string(case.join("analysis/r.json")).expect("r"),
            "{\"d\": \"blake3:ff00\"}"
        );
        assert!(
            fs::read_to_string(case.join("expected.toml"))
                .expect("t")
                .contains("status = \"confirmed\"")
        );
    }

    // frob:tests crates/cadetta-cli/tests/golden/confirm.rs::refuses_when_a_tbd_file_is_not_regenerated
    #[test]
    fn refuses_when_a_tbd_file_is_not_regenerated() {
        let tmp = Scratch::new().expect("scratch");
        let case = tmp.0.join("case");
        fs::create_dir_all(&case).expect("mkdir");
        fs::write(
            case.join("expected.toml"),
            "[case]\nstatus = \"hand-evaluated\"\n",
        )
        .expect("w");
        fs::write(case.join("r.json"), "blake3:TBD").expect("w");
        assert!(confirm_with(&case, |_| Ok(())).is_err());
        assert!(
            fs::read_to_string(case.join("expected.toml"))
                .expect("t")
                .contains("hand-evaluated")
        );
    }
}
