//! Lints R007-R013, the adapter and binding rules (lints.md 3 R).
//!
//! Pure functions over facts an adapter response or the binding resolver
//! supplies. R004 and R008-R010 come from the adapter and pass through
//! unchanged; the engine gates R008 (error on sync, info on check).
//!
//! Specification: `docs/spec/lints.md` 3 R, `docs/spec/adapters.md` 3,
//! `docs/spec/geometry.md` 6.

// frob:ticket 01M4SWKAH8CP5VJYR93A9M9EPG

use std::collections::BTreeMap;

use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Severity};
use crate::rules::r::NativeRef;

/// R007 code.
pub const CODE_R007: &str = "R007";
/// R008 code.
pub const CODE_R008: &str = "R008";
/// R011 code.
pub const CODE_R011: &str = "R011";
/// R012 code (reserved: unreachable from source, see lints.md).
pub const CODE_R012: &str = "R012";
/// R013 code.
pub const CODE_R013: &str = "R013";

/// The diagnostic location for a native locator.
fn loc(n: &NativeRef) -> Location {
    Location::Native {
        adapter: n.adapter.clone(),
        file: n.file.clone(),
        locator: n.locator.clone(),
    }
}

/// A diagnostic an adapter reported, to be passed through unchanged.
#[derive(Clone, Debug)]
pub struct AdapterReport {
    /// The rule code the adapter named.
    pub code: String,
    /// The adapter's message text.
    pub message: String,
    /// The native locator it concerns, if any.
    pub native: Option<NativeRef>,
}

/// R004 and R008-R010: forward adapter-reported diagnostics unchanged; other codes are dropped.
pub fn pass_through(reports: &[AdapterReport]) -> Vec<Diagnostic> {
    reports
        .iter()
        .filter_map(|r| {
            let sev = match r.code.as_str() {
                "R004" | "R008" | "R009" | "R010" => Severity::Error,
                other => {
                    trace!(code = other, "adapter report is not a pass-through code");
                    return None;
                }
            };
            debug!(code = %r.code, "adapter diagnostic passed through");
            let mut d = Diagnostic::new(r.code.clone(), sev, r.message.clone());
            d.location = r.native.as_ref().map(loc);
            Some(d)
        })
        .collect()
}

/// A bound coordinate with where its native sketch sits.
#[derive(Clone, Debug)]
pub struct CoordFact {
    /// Canonical IR path of the bound coordinate.
    pub path: String,
    /// The bound native locator.
    pub native: NativeRef,
    /// True when the sketch lies on the datum's plane.
    pub on_datum_plane: bool,
    /// True when the dimension is measured from the sketch origin.
    pub from_origin: bool,
}

/// R007: a bound coordinate's sketch is off its datum plane or not measured from the origin.
pub fn check_r007(facts: &[CoordFact]) -> Vec<Diagnostic> {
    facts
        .iter()
        .filter(|f| !f.on_datum_plane || !f.from_origin)
        .map(|f| {
            let why = if f.on_datum_plane {
                "not measured from the sketch origin"
            } else {
                "sketch is not on the datum's plane"
            };
            debug!(path = %f.path, why, "R007: binding/datum mismatch");
            let mut d = Diagnostic::new(
                CODE_R007,
                Severity::Warning,
                format!("coordinate `{}`: {why}", f.path),
            );
            d.path = Some(f.path.clone());
            d.location = Some(loc(&f.native));
            d
        })
        .collect()
}

/// An assembly slot: the file bound to its primary versus the file instantiated.
#[derive(Clone, Debug)]
pub struct SlotFact {
    /// Slot path.
    pub slot: String,
    /// File bound to the slot's primary.
    pub primary_file: String,
    /// File the assembly actually holds.
    pub held_file: String,
    /// The assembly locator.
    pub native: NativeRef,
}

/// R011: the assembly holds a file other than the slot primary's.
pub fn check_r011(slots: &[SlotFact]) -> Vec<Diagnostic> {
    slots
        .iter()
        .filter(|s| s.primary_file != s.held_file)
        .map(|s| {
            debug!(slot = %s.slot, "R011: component mismatch");
            let mut d = Diagnostic::new(
                CODE_R011,
                Severity::Warning,
                format!(
                    "slot `{}` holds `{}` but its primary is bound to `{}`",
                    s.slot, s.held_file, s.primary_file
                ),
            );
            d.path = Some(s.slot.clone());
            d.location = Some(loc(&s.native));
            d
        })
        .collect()
}

/// A `[*]` binding with the values emitted for each matched instance.
#[derive(Clone, Debug)]
pub struct StarBinding {
    /// The binding path pattern.
    pub path: String,
    /// The binding's locator.
    pub native: NativeRef,
    /// Emitted value per instance.
    pub values: Vec<String>,
}

/// R012 (reserved, dormant): a `[*]` binding over instances whose emitted values differ.
///
/// Unreachable from source today since part-array instances share arguments; kept for the
/// future language feature named in adapters.md 3.
pub fn check_r012(bindings: &[StarBinding]) -> Vec<Diagnostic> {
    bindings
        .iter()
        .filter(|b| b.values.windows(2).any(|w| w[0] != w[1]))
        .map(|b| {
            debug!(path = %b.path, "R012: divergent binding");
            let mut d = Diagnostic::new(
                CODE_R012,
                Severity::Error,
                format!(
                    "binding `{}` spans instances with differing values: {}",
                    b.path,
                    b.values.join(", ")
                ),
            );
            d.path = Some(b.path.clone());
            d.location = Some(loc(&b.native));
            d
        })
        .collect()
}

/// A path's claim on a native name.
#[derive(Clone, Debug)]
pub struct NameClaim {
    /// The native name the path would create (or the part named by a reference property).
    pub name: String,
    /// The IR path making the claim.
    pub path: String,
    /// True when the user bound it explicitly (adapters.md 3.1).
    pub explicit: bool,
}

/// R013 (fixed error): two distinct paths would create the same native name, unless all are explicit.
pub fn check_r013(claims: &[NameClaim]) -> Vec<Diagnostic> {
    let mut by_name: BTreeMap<&str, Vec<&NameClaim>> = BTreeMap::new();
    for c in claims {
        by_name.entry(&c.name).or_default().push(c);
    }
    let mut out = Vec::new();
    for (name, group) in by_name {
        let mut paths: Vec<&str> = group.iter().map(|c| c.path.as_str()).collect();
        paths.sort_unstable();
        paths.dedup();
        if paths.len() < 2 || group.iter().all(|c| c.explicit) {
            continue;
        }
        debug!(name, "R013: native name collision");
        let mut d = Diagnostic::new(
            CODE_R013,
            Severity::Error,
            format!(
                "paths {} would create the same native name `{name}`; bind one explicitly",
                paths.join(", ")
            ),
        );
        d.path = Some(paths[0].to_owned());
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nref() -> NativeRef {
        NativeRef {
            adapter: "sw".into(),
            file: "a.sldasm".into(),
            locator: "L".into(),
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/r_adapter.rs::pass_through
    #[test]
    fn adapter_codes_pass_through_unchanged() {
        let rep = |c: &str| AdapterReport {
            code: c.into(),
            message: format!("msg {c}"),
            native: Some(nref()),
        };
        let d = pass_through(&[
            rep("R004"),
            rep("R008"),
            rep("R009"),
            rep("R010"),
            rep("R001"),
        ]);
        let codes: Vec<_> = d.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(codes, ["R004", "R008", "R009", "R010"]);
        assert_eq!(d[1].message, "msg R008");
        assert_eq!(d[1].severity, Severity::Error);
    }

    // frob:tests crates/cadetta-lint/src/engine.rs::LintConfig.severity
    #[test]
    fn r008_is_error_on_sync_and_info_on_check() {
        use crate::engine::{LintConfig, Mode};
        let mut cfg = LintConfig::default();
        cfg.mode = Mode::Check;
        assert_eq!(cfg.severity("R008"), Some(Severity::Info));
        cfg.mode = Mode::Sync;
        assert_eq!(cfg.severity("R008"), Some(Severity::Error));
    }

    // frob:tests crates/cadetta-lint/src/rules/r_adapter.rs::check_r011
    #[test]
    fn r011_fires_on_swapped_file() {
        let s = |held: &str| SlotFact {
            slot: "asm::s".into(),
            primary_file: "a.sldprt".into(),
            held_file: held.into(),
            native: nref(),
        };
        assert_eq!(check_r011(&[s("b.sldprt")])[0].code, "R011");
        assert!(check_r011(&[s("a.sldprt")]).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/r_adapter.rs::check_r012
    #[test]
    fn r012_fires_on_differing_values() {
        let b = |v: &[&str]| StarBinding {
            path: "p[*].d".into(),
            native: nref(),
            values: v.iter().map(|s| (*s).into()).collect(),
        };
        let d = check_r012(&[b(&["1", "2"])]);
        assert_eq!(d[0].code, "R012");
        assert_eq!(d[0].severity, Severity::Error);
        assert!(check_r012(&[b(&["1", "1"]), b(&[])]).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/r_adapter.rs::check_r013
    #[test]
    fn r013_fires_on_shared_native_name() {
        let c = |p: &str, e: bool| NameClaim {
            name: "D1".into(),
            path: p.into(),
            explicit: e,
        };
        let d = check_r013(&[c("a::x", false), c("b::x", true)]);
        assert_eq!(d[0].code, "R013");
        assert_eq!(d[0].severity, Severity::Error);
        assert!(check_r013(&[c("a::x", true), c("b::x", true)]).is_empty());
        assert!(check_r013(&[c("a::x", false), c("a::x", false)]).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/r_adapter.rs::check_r007
    #[test]
    fn r007_warns_off_plane_or_off_origin() {
        let f = |p, o| CoordFact {
            path: "m::x".into(),
            native: nref(),
            on_datum_plane: p,
            from_origin: o,
        };
        let d = check_r007(&[f(false, true), f(true, false), f(true, true)]);
        assert_eq!(d.len(), 2);
        assert!(
            d.iter()
                .all(|d| d.code == "R007" && d.severity == Severity::Warning)
        );
    }
}
