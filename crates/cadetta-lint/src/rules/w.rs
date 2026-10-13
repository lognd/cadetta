//! Lints W001-W004: expired, stale, disallowed and unused waivers.
//!
//! The engine ([`crate::engine::apply`]) classifies each waiver and records
//! which expired, went stale or matched nothing; [`check`] turns that outcome
//! into catalog diagnostics and adds W003 (a waiver or `allow` of a fixed code,
//! or of a `require`, which is never suppressed). W005 is a separate rule.
//!
//! Specification: `docs/spec/lints.md` 2.1 to 2.3; `docs/spec/obligations.md` 4.

// frob:ticket 01M4SPWRDX98PHVNHPD14HYY8D

use cadetta_obligations::selector::Glob;
use tracing::{debug, trace};

use crate::diag::Diagnostic;
use crate::engine::{LintConfig, Outcome, Suppressions, explain, waiver_id};

/// Rule code: the waiver's `expires` date has passed (error).
pub const W001: &str = "W001";
/// Rule code: a target's closure digest changed since review (warning).
pub const W002: &str = "W002";
/// Rule code: waiver or allow of a fixed code or of a `require` (error).
pub const W003: &str = "W003";
/// Rule code: a waiver or allow suppressed nothing (warning).
pub const W004: &str = "W004";

/// Build one W diagnostic at the catalog severity, naming the waiver `id`.
fn emit(code: &str, cfg: &LintConfig, message: String) -> Diagnostic {
    let sev = cfg.severity(code).expect("W code is in the catalog");
    Diagnostic::new(code, sev, message)
}

/// Why `code` at `glob` (or exact `path`) may not be suppressed, if it may not.
fn disallowed(code: &str, hits_require: bool) -> Option<&'static str> {
    if explain(code).is_some_and(|c| c.fixed) {
        Some("names a fixed code")
    } else if hits_require {
        Some("targets a `require`, which is never suppressed")
    } else {
        None
    }
}

/// W003 diagnostics for waivers and allows that name fixed codes or `require`s; returns their ids too.
fn check_w003(sup: &Suppressions, cfg: &LintConfig) -> (Vec<Diagnostic>, Vec<String>) {
    let (mut out, mut ids) = (Vec::new(), Vec::new());
    for w in &sup.waivers {
        let glob = Glob::new(&w.glob).ok();
        let hits = glob
            .as_ref()
            .is_some_and(|g| sup.require_paths.iter().any(|p| g.matches(p)));
        if let Some(why) = disallowed(&w.code, hits) {
            let id = waiver_id(w);
            debug!(%id, why, "W003: waiver not allowed");
            out.push(emit(W003, cfg, format!("`{id}` {why}")));
            ids.push(id);
        }
    }
    for a in &sup.allows {
        let hits = sup.require_paths.contains(&a.path);
        if let Some(why) = disallowed(&a.code, hits) {
            let id = a.id();
            debug!(%id, why, "W003: allow not allowed");
            let mut d = emit(W003, cfg, format!("`{id}` {why}"));
            d.path = Some(a.path.clone());
            out.push(d);
            ids.push(id);
        }
    }
    (out, ids)
}

/// Run W001-W004 over a gated run and the suppressions that produced it.
pub fn check(outcome: &Outcome, sup: &Suppressions, cfg: &LintConfig) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for id in &outcome.expired {
        debug!(%id, "W001: waiver expired");
        out.push(emit(
            W001,
            cfg,
            format!("`{id}` expired and no longer suppresses"),
        ));
    }
    for id in &outcome.stale {
        debug!(%id, "W002: waiver stale");
        out.push(emit(
            W002,
            cfg,
            format!("`{id}` targets changed since review; re-review with `cadetta ack`"),
        ));
    }
    let (w003, disallowed_ids) = check_w003(sup, cfg);
    out.extend(w003);
    for id in &outcome.unused {
        if disallowed_ids.contains(id) {
            trace!(%id, "W004: skipped, already W003");
            continue;
        }
        debug!(%id, "W004: suppresses nothing");
        out.push(emit(W004, cfg, format!("`{id}` suppresses nothing")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Severity;
    use crate::engine::{Allow, apply};
    use cadetta_ir::entities::Waiver;
    use cadetta_ir::model::Digest;
    use cadetta_ir::variant_name::VariantName;

    fn diag(code: &str, path: &str) -> Diagnostic {
        let mut x = Diagnostic::new(code, Severity::Warning, "m");
        x.path = Some(path.into());
        x
    }

    fn run(diags: Vec<Diagnostic>, sup: &Suppressions) -> Vec<Diagnostic> {
        let cfg = LintConfig::new();
        let out = apply(diags, &cfg, sup);
        check(&out, sup, &cfg)
    }

    fn codes(d: &[Diagnostic]) -> Vec<&str> {
        d.iter().map(|x| x.code.as_str()).collect()
    }

    // frob:tests crates/cadetta-lint/src/rules/w.rs::check kind=unit
    #[test]
    fn w001_expired_waiver_is_error() {
        let mut w = Waiver::new("M001", "a::B.c", "ok");
        w.expires = Some("2026-01-01".into());
        let sup = Suppressions {
            waivers: vec![w],
            as_of: Some("2026-06-01".into()),
            ..Default::default()
        };
        let out = run(vec![diag("M001", "a::B.c")], &sup);
        assert_eq!(codes(&out), ["W001"]);
        assert_eq!(out[0].severity, Severity::Error);
    }

    // frob:tests crates/cadetta-lint/src/rules/w.rs::check kind=unit
    #[test]
    fn w003_fixed_code_waiver() {
        let sup = Suppressions {
            waivers: vec![Waiver::new("E002", "a::B.x", "ok")],
            ..Default::default()
        };
        let out = run(vec![diag("E002", "a::B.x")], &sup);
        assert_eq!(codes(&out), ["W003"], "no duplicate W004");
        assert_eq!(out[0].severity, Severity::Error);
    }

    // frob:tests crates/cadetta-lint/src/rules/w.rs::check kind=unit
    #[test]
    fn w003_require_waiver_and_allow() {
        let mut sup = Suppressions {
            waivers: vec![Waiver::new("C001", "a::R.*", "ok")],
            allows: vec![Allow::new("C001", "a::R.req", "why").unwrap()],
            ..Default::default()
        };
        sup.require_paths.insert("a::R.req".into());
        let out = run(vec![diag("C001", "a::R.req")], &sup);
        assert_eq!(codes(&out), ["W003", "W003"]);
    }

    // frob:tests crates/cadetta-lint/src/rules/w.rs::check kind=unit
    #[test]
    fn w004_unused_waiver_and_allow() {
        let sup = Suppressions {
            waivers: vec![Waiver::new("M001", "a::B.*", "ok")],
            allows: vec![Allow::new("M002", "a::B.c", "why").unwrap()],
            ..Default::default()
        };
        let out = run(vec![], &sup);
        assert_eq!(codes(&out), ["W004", "W004"]);
        assert_eq!(out[0].severity, Severity::Warning);
    }

    // frob:tests crates/cadetta-lint/src/rules/w.rs::check kind=unit
    #[test]
    fn glob_waiver_suppresses_matching_paths_only() {
        let sup = Suppressions {
            waivers: vec![Waiver::new("D001", "acme.parts::Bearing608.*", "ok")],
            ..Default::default()
        };
        let cfg = LintConfig::new();
        let out = apply(
            vec![
                diag("D001", "acme.parts::Bearing608.bore"),
                diag("D001", "acme.parts::Other.bore"),
            ],
            &cfg,
            &sup,
        );
        let waived: Vec<bool> = out.diagnostics.iter().map(|d| d.waived.is_some()).collect();
        assert_eq!(waived.iter().filter(|b| **b).count(), 1);
        assert!(check(&out, &sup, &cfg).is_empty(), "used, so no W004");
    }

    // frob:tests crates/cadetta-lint/src/rules/w.rs::check kind=unit
    #[test]
    fn w002_only_when_recorded_digest_differs() {
        let dg = |s: &str| Digest(s.into());
        let mut open = Waiver::new("D001", "a::B.*", "ok");
        let mut stale = open.clone();
        stale.targets_closure.insert("a::B.x".into(), dg("old"));
        let sup = |w: Waiver| Suppressions {
            waivers: vec![w],
            closures: [("a::B.x".to_owned(), dg("new"))].into(),
            ..Default::default()
        };
        // never reviewed: still suppresses, no W002 (review is O002)
        let out = run(vec![diag("D001", "a::B.x")], &sup(open.clone()));
        assert!(out.is_empty());
        open.targets_closure.insert("a::B.x".into(), dg("new"));
        assert!(run(vec![diag("D001", "a::B.x")], &sup(open)).is_empty());
        let out = run(vec![diag("D001", "a::B.x")], &sup(stale));
        assert_eq!(codes(&out), ["W002"]);
    }

    // frob:tests crates/cadetta-lint/src/rules/w.rs::check kind=unit
    #[test]
    fn alt_scoped_waiver_uses_same_states() {
        let mut w = Waiver::new("D001", "a::B.*", "ok");
        w.for_alt = Some(VariantName::parse("a::B/wheel=a::Colson2Series").unwrap());
        w.expires = Some("2026-01-01".into());
        let sup = Suppressions {
            waivers: vec![w],
            as_of: Some("2026-02-01".into()),
            ..Default::default()
        };
        assert_eq!(codes(&run(vec![], &sup)), ["W001"]);
    }

    // frob:tests crates/cadetta-lint/src/rules/w.rs::check kind=unit
    #[test]
    fn allow_has_no_expiry_or_stale_state() {
        let sup = Suppressions {
            allows: vec![Allow::new("M001", "a::B.c", "why").unwrap()],
            as_of: Some("2999-01-01".into()),
            ..Default::default()
        };
        let out = run(vec![diag("M001", "a::B.c")], &sup);
        assert!(out.is_empty());
    }
}
