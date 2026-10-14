//! Default-implementation precedence (stdlib.md 4, config-cli.md 2).
// frob:ticket 01M4SWP84AG2TGPYHCHTGVJGFZ
//!
//! For an interface, the default implementation is the first hit, highest
//! first: the component's own `#![default]`, the file's `#![default]`,
//! the project `[defaults]` table, then the `std` default.

use std::collections::BTreeMap;

use tracing::{debug, trace};

/// Where a resolved default came from, highest precedence first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    /// `#![default(Iface = Impl)]` on the component.
    Component,
    /// `#![default(Iface = Impl)]` at file level.
    File,
    /// The project `cadetta.toml` `[defaults]` table.
    Project,
    /// The `std` default (for example `IsoDefault`).
    Std,
}

/// The default implementation for an interface and the layer that supplied it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution {
    /// The implementation as written in its layer (a name or a qualified path).
    pub value: String,
    /// The layer that won.
    pub source: Source,
}

/// The default layers in play for one interface lookup; each maps interface to implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct Layers<'a> {
    /// The component's own `#![default]` entries.
    pub component: Option<&'a BTreeMap<String, String>>,
    /// The file's `#![default]` entries.
    pub file: Option<&'a BTreeMap<String, String>>,
    /// The project `[defaults]` table.
    pub project: Option<&'a BTreeMap<String, String>>,
    /// The `std` default for the interface, when one exists.
    pub std: Option<&'a str>,
}

/// Resolve the default for `iface`: component, file, project, then std.
pub fn resolve_default(iface: &str, layers: &Layers<'_>) -> Option<Resolution> {
    let tables = [
        (Source::Component, layers.component),
        (Source::File, layers.file),
        (Source::Project, layers.project),
    ];
    for (source, table) in tables {
        match table.and_then(|t| t.get(iface)) {
            Some(value) => {
                debug!(iface, ?source, value = %value, "default resolved");
                return Some(Resolution {
                    value: value.clone(),
                    source,
                });
            }
            None => trace!(iface, ?source, "no default in layer"),
        }
    }
    match layers.std {
        Some(value) => {
            debug!(iface, value, "default falls back to std");
            Some(Resolution {
                value: value.to_string(),
                source: Source::Std,
            })
        }
        None => {
            debug!(iface, "no default in any layer");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(v: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("FastenerPolicy".to_string(), v.to_string())])
    }

    // frob:tests crates/cadetta-sema/src/defaults.rs::resolve_default kind=unit
    #[test]
    fn precedence_is_component_file_project_std() {
        let (c, f, p) = (table("C"), table("F"), table("P"));
        let mut l = Layers {
            component: Some(&c),
            file: Some(&f),
            project: Some(&p),
            std: Some("IsoDefault"),
        };
        let got = |l: &Layers<'_>| {
            let r = resolve_default("FastenerPolicy", l).unwrap();
            (r.value, r.source)
        };
        let want = |v: &str, s| (v.to_string(), s);
        assert_eq!(got(&l), want("C", Source::Component));
        l.component = None;
        assert_eq!(got(&l), want("F", Source::File));
        l.file = None;
        assert_eq!(got(&l), want("P", Source::Project));
        l.project = None;
        assert_eq!(got(&l), want("IsoDefault", Source::Std));
    }

    // frob:tests crates/cadetta-sema/src/defaults.rs::resolve_default kind=unit
    #[test]
    fn other_interfaces_and_empty_layers_resolve_to_nothing() {
        let c = table("C");
        let l = Layers {
            component: Some(&c),
            ..Layers::default()
        };
        assert_eq!(resolve_default("Other", &l), None);
        assert_eq!(resolve_default("FastenerPolicy", &Layers::default()), None);
    }
}
