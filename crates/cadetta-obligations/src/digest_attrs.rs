//! Semantic versus presentational digests as seen by staleness.
//!
//! Presentational attributes (`display`, `emit`, `allow`, `lifecycle`) are excluded from digests
//! in `cadetta-ir`, so editing them never changes a closure digest and no obligation goes stale;
//! semantic attributes do. Specification: `docs/spec/ir.md` section 4.
//! frob:ticket 01M4SWN7NS97VV30F1A5W8KAEP

use cadetta_ir::digest::{DigestError, DigestGraph};
use cadetta_ir::model::Member;
use tracing::debug;

/// Current closure digest hex of `key` over `members`, as bound by obligations (obligations.md 2).
pub fn current_closure(members: &[Member], key: &str) -> Result<String, DigestError> {
    let mut g = DigestGraph::new();
    for m in members {
        g.add_member(m);
    }
    let d = g.closure(key)?;
    debug!(key, closure = %d.0, "current closure computed");
    Ok(d.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Evidence, ObligationKind, ObligationState, classify};
    use cadetta_ir::digest::PRESENTATIONAL_ATTRS;
    use cadetta_ir::model::MemberKind;
    use cadetta_ir::path::{Path, PathContext};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn mem(path: &str, deps: &[&str]) -> Member {
        let mut m = Member::new(p(path), MemberKind::Param);
        m.deps = deps.iter().map(|d| p(d)).collect();
        m
    }

    /// Classify an `ots` obligation (a non-recomputed kind, so staleness is observable).
    fn state_ots(recorded: &str, current: &str) -> ObligationState {
        classify(&Evidence {
            kind: ObligationKind::Ots,
            verdict: None,
            recorded: Some(recorded),
            current,
            suppressed: false,
        })
    }

    // frob:tests crates/cadetta-obligations/src/digest_attrs.rs::current_closure
    #[test]
    fn presentational_edits_stay_discharged_semantic_edits_go_stale() {
        let base = vec![mem("a.b::X.d", &[]), mem("a.b::X.e", &["a.b::X.d"])];
        let recorded = current_closure(&base, "a.b::X.e").unwrap();
        assert_eq!(state_ots(&recorded, &recorded), ObligationState::Discharged);

        for k in PRESENTATIONAL_ATTRS {
            let mut ms = base.clone();
            ms[0].attrs.insert(k.into(), "changed".into());
            let now = current_closure(&ms, "a.b::X.e").unwrap();
            assert_eq!(
                state_ots(&recorded, &now),
                ObligationState::Discharged,
                "{k}"
            );
        }
        for k in ["process", "default", "hole"] {
            let mut ms = base.clone();
            ms[0].attrs.insert(k.into(), "changed".into());
            let now = current_closure(&ms, "a.b::X.e").unwrap();
            assert_eq!(state_ots(&recorded, &now), ObligationState::Stale, "{k}");
        }
        let mut ms = base.clone();
        ms[0].role = Some("hole".into());
        let now = current_closure(&ms, "a.b::X.e").unwrap();
        assert_eq!(state_ots(&recorded, &now), ObligationState::Stale, "role");
    }
}
