//! Import `--accept`: turn a reviewed proposal into bindings and the adoption marker
//! (adapters.md 7.4).
//!
//! The planning here is pure: it decides which proposal items become new binding tables and
//! which are already bound. Writing `bindings/<id>.toml`, `cad/imported/` and the `adopted` lock
//! record is the CLI's job. While an `adopted` record exists, [`adopted_r006`] reports every
//! native expression the next push will replace.
// frob:ticket 01M4SWMCT03T7KB7HZRJXRXM2Y

use std::collections::BTreeSet;

use super::propose::{Item, Proposal, Status};
use crate::sync::{Diag, Entry, R006};

/// What `--accept` will add, given what is already bound.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AcceptPlan {
    /// TOML text to append to the bindings file.
    pub appended: String,
    /// Canonical paths newly bound.
    pub added: Vec<String>,
    /// Canonical paths skipped because the path or locator is already bound.
    pub unchanged: Vec<String>,
}

/// Quotes `s` as a TOML basic string.
pub fn quote(s: &str) -> String {
    toml::Value::String(s.to_owned()).to_string()
}

/// The canonical path an item binds under: a datum or member `pkg::Comp.name`, a place
/// `pkg::Asm/part.place` (adapters.md 7.2).
pub fn canonical_path(package: &str, item: &Item) -> String {
    if item.kind == "instance" {
        format!("{package}::{}/{}.place", item.component, item.path)
    } else {
        format!("{package}::{}.{}", item.component, item.path)
    }
}

/// Renders one binding table for a canonical path and its proposal item.
pub fn binding_table(canonical: &str, item: &Item) -> String {
    let mut loc = format!(
        "{{ locator = {}, kind = {}, mode = \"value\"",
        quote(&item.locator),
        quote(&item.kind)
    );
    if let Some(unit) = &item.unit {
        loc.push_str(&format!(", native_unit = {}", quote(unit)));
    }
    loc.push_str(" }");
    format!("[{}]\nlocators = [ {loc} ]\n", quote(canonical))
}

/// Plans the binding additions for the non-skipped items; already bound paths or locators stay.
pub fn plan(
    proposal: &Proposal,
    package: &str,
    bound_paths: &mut BTreeSet<String>,
    bound_locators: &mut BTreeSet<String>,
) -> AcceptPlan {
    let mut out = AcceptPlan::default();
    for item in proposal.items.iter().filter(|i| i.status != Status::Skip) {
        let canonical = canonical_path(package, item);
        if bound_paths.contains(&canonical) || bound_locators.contains(&item.locator) {
            tracing::debug!(path = %canonical, "accept: already bound");
            out.unchanged.push(canonical);
            continue;
        }
        out.appended.push_str(&binding_table(&canonical, item));
        out.appended.push('\n');
        bound_paths.insert(canonical.clone());
        bound_locators.insert(item.locator.clone());
        out.added.push(canonical);
    }
    tracing::info!(
        added = out.added.len(),
        unchanged = out.unchanged.len(),
        "accept: planned"
    );
    out
}

/// R006 diagnostics for adopted files: each push entry replacing a native expression.
///
/// `adopted` holds `(adapter, file)` pairs from the lock; only entries whose value differs from
/// the observed one (`ours != theirs`) are pushed, so only those replace the expression.
pub fn adopted_r006(adopted: &BTreeSet<(String, String)>, entries: &[Entry]) -> Vec<Diag> {
    entries
        .iter()
        .filter(|e| adopted.contains(&(e.adapter.clone(), e.file.clone())) && e.ours != e.theirs)
        .filter_map(|e| {
            e.replaces.as_ref().map(|old| Diag {
                code: R006,
                message: format!(
                    "{} replaces native expression `{old}` (adopted file {})",
                    e.locator, e.file
                ),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(path: &str, locator: &str, status: Status) -> Item {
        Item {
            file: "cover.SLDPRT".into(),
            locator: locator.into(),
            kind: "global".into(),
            native_name: locator.into(),
            expression: None,
            value: "5".into(),
            unit: Some("mm".into()),
            unit_assumed: false,
            path: path.into(),
            component: "Cover".into(),
            part: None,
            property_type: None,
            status,
            reason: None,
        }
    }

    fn entry(replaces: Option<&str>, ours: &str, theirs: &str) -> Entry {
        Entry {
            adapter: "sw".into(),
            file: "cover.SLDPRT".into(),
            path: "p::Cover.t".into(),
            locator: "T".into(),
            base: None,
            ours: ours.into(),
            theirs: theirs.into(),
            replaces: replaces.map(str::to_owned),
            ours_tol: None,
            theirs_tol: None,
        }
    }

    // frob:tests plan_binds_new_and_skips_rest
    #[test]
    fn plan_binds_new_and_skips_rest() {
        let p = Proposal {
            adapter: "sw".into(),
            items: vec![
                item("t", "T", Status::Accept),
                item("u", "U", Status::Skip),
                item("w", "W", Status::Accept),
            ],
        };
        let mut paths = BTreeSet::from(["p::Cover.w".to_owned()]);
        let mut locs = BTreeSet::new();
        let plan = plan(&p, "p", &mut paths, &mut locs);
        assert_eq!(plan.added, ["p::Cover.t"]);
        assert_eq!(plan.unchanged, ["p::Cover.w"]);
        assert!(plan.appended.contains("[\"p::Cover.t\"]"));
        assert!(plan.appended.contains("native_unit = \"mm\""));
        assert!(!plan.appended.contains("\"U\""));
    }

    // frob:tests adopted_binding_fires_r006
    #[test]
    fn adopted_binding_fires_r006() {
        let adopted = BTreeSet::from([("sw".to_owned(), "cover.SLDPRT".to_owned())]);
        let d = adopted_r006(&adopted, &[entry(Some("=\"A\"*2"), "6", "5")]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, R006);
        assert!(d[0].message.contains("\"A\"*2"));
        assert!(adopted_r006(&adopted, &[entry(None, "6", "5")]).is_empty());
        assert!(adopted_r006(&adopted, &[entry(Some("=1"), "5", "5")]).is_empty());
        assert!(adopted_r006(&BTreeSet::new(), &[entry(Some("=1"), "6", "5")]).is_empty());
    }

    // frob:ticket 01M4VB64CJDA2CZ4WM4MNV70G6
    // frob:tests accept_writes_frame_and_instance_locators
    #[test]
    fn accept_writes_frame_and_instance_locators() {
        let mut frame = item("mount", "Mount", Status::Accept);
        frame.kind = "frame".into();
        frame.component = "Rail".into();
        let mut inst = item("bracket", "bracket<1>", Status::Accept);
        inst.kind = "instance".into();
        inst.component = "Drive".into();
        inst.part = Some("Bracket".into());
        let p = Proposal {
            adapter: "sw".into(),
            items: vec![frame, inst],
        };
        let out = plan(&p, "p", &mut BTreeSet::new(), &mut BTreeSet::new());
        assert_eq!(out.added, ["p::Rail.mount", "p::Drive/bracket.place"]);
        assert!(out.appended.contains(
            "locators = [ { locator = \"Mount\", kind = \"frame\", mode = \"value\", native_unit = \"mm\" } ]"
        ));
        assert!(out.appended.contains("[\"p::Drive/bracket.place\"]"));
        assert!(out.appended.contains(
            "{ locator = \"bracket<1>\", kind = \"instance\", mode = \"value\", native_unit = \"mm\" }"
        ));
    }
}
