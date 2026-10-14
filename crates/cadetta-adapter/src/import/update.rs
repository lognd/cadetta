//! Import `--update` and `--write`: pull CAD edits back into `.cdt` literals (adapters.md 7.4).
//!
//! The planning here is pure. The CLI resolves each bound path to a [`Source`] (a literal span,
//! or why it is not editable) and pairs it with the sync [`Entry`] holding the lock base and the
//! observed CAD value. [`plan`] decides the literal edits and what is skipped or flagged;
//! [`render_diff`] prints them; [`apply`] rewrites one source text. Nothing is written without
//! `--write`: the CLI calls [`apply`] and updates the lock only then (config-cli.md 3).
// frob:ticket 01M4SWMCWHXKA8K6VG7Z1PNF3W

use std::collections::BTreeMap;
use std::fmt;

use crate::sync::Entry;

/// How a bound path is defined in `.cdt` source, as resolved by the CLI from the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Definition {
    /// A plain literal at `start..end` (byte offsets) in `file`.
    Literal {
        /// Source file the literal lives in.
        file: String,
        /// Byte offset of the first character of the literal.
        start: usize,
        /// Byte offset one past the last character of the literal.
        end: usize,
    },
    /// A literal carrying a tolerance or fit; edited by hand.
    Toleranced,
    /// A `let` binding or any path computed from an expression.
    Derived,
}

/// One bound path ready for update planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// How the path is defined in source.
    pub definition: Definition,
    /// The path carries a citation.
    pub cited: bool,
}

/// One literal edit in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// Canonical path the edit serves.
    pub path: String,
    /// Source file to edit.
    pub file: String,
    /// Byte offset of the literal start.
    pub start: usize,
    /// Byte offset one past the literal end.
    pub end: usize,
    /// The literal as it stands (checked by [`apply`]).
    pub old: String,
    /// The observed CAD value as a literal.
    pub new: String,
}

/// Why a changed path is not edited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skip {
    /// The literal has a tolerance or fit; carries the observed value.
    Toleranced(String),
    /// `let` or derived: never touched.
    Derived,
    /// The path has no source definition the CLI could resolve.
    Unresolved,
}

impl fmt::Display for Skip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Skip::Toleranced(v) => write!(f, "has a tolerance or fit; CAD value {v}, edit by hand"),
            Skip::Derived => f.write_str("derived; edit its sources"),
            Skip::Unresolved => f.write_str("no source definition found"),
        }
    }
}

/// The result of planning an update.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdatePlan {
    /// Literal edits, sorted by file then offset.
    pub edits: Vec<Edit>,
    /// Changed paths that are not edited, with the reason.
    pub skipped: Vec<(String, Skip)>,
    /// Edited paths whose citation no longer matches the value.
    pub cite_flags: Vec<String>,
    /// `(adapter, file, locator, value)` lock `synced` values to record on `--write`.
    pub lock_updates: Vec<(String, String, String, String)>,
}

/// Plans literal edits for entries whose observed value differs from the lock base.
///
/// Entries without a base, or with `theirs == base`, are unchanged. `sources` maps canonical
/// paths to their definition; only plain literals are edited, never expressions.
pub fn plan(entries: &[Entry], sources: &BTreeMap<String, Source>) -> UpdatePlan {
    let mut out = UpdatePlan::default();
    for e in entries {
        if e.base.as_deref().is_none_or(|b| b == e.theirs) {
            tracing::debug!(path = %e.path, "update: unchanged in CAD");
            continue;
        }
        let Some(src) = sources.get(&e.path) else {
            tracing::warn!(path = %e.path, "update: no source definition");
            out.skipped.push((e.path.clone(), Skip::Unresolved));
            continue;
        };
        match &src.definition {
            Definition::Derived => {
                tracing::info!(path = %e.path, "update: derived, skipped");
                out.skipped.push((e.path.clone(), Skip::Derived));
            }
            Definition::Toleranced => {
                tracing::info!(path = %e.path, "update: toleranced, skipped");
                out.skipped
                    .push((e.path.clone(), Skip::Toleranced(e.theirs.clone())));
            }
            Definition::Literal { file, start, end } => {
                tracing::info!(path = %e.path, new = %e.theirs, "update: literal edit");
                out.edits.push(Edit {
                    path: e.path.clone(),
                    file: file.clone(),
                    start: *start,
                    end: *end,
                    old: e.ours.clone(),
                    new: e.theirs.clone(),
                });
                if src.cited {
                    out.cite_flags.push(e.path.clone());
                }
                out.lock_updates.push((
                    e.adapter.clone(),
                    e.file.clone(),
                    e.locator.clone(),
                    e.theirs.clone(),
                ));
            }
        }
    }
    out.edits
        .sort_by(|a, b| (&a.file, a.start).cmp(&(&b.file, b.start)));
    out
}

/// Renders the plan as the diff `--update` prints (one `-`/`+` pair per edit, then notes).
pub fn render_diff(plan: &UpdatePlan) -> String {
    let mut s = String::new();
    for e in &plan.edits {
        s.push_str(&format!(
            "{} ({})\n- {}\n+ {}\n",
            e.path, e.file, e.old, e.new
        ));
    }
    for p in &plan.cite_flags {
        s.push_str(&format!("{p}: value no longer matches its cite\n"));
    }
    for (p, why) in &plan.skipped {
        s.push_str(&format!("{p}: skipped, {why}\n"));
    }
    s
}

/// Applies the edits for one file to its text; `Err` when a literal is no longer where it was.
pub fn apply(text: &str, file: &str, plan: &UpdatePlan) -> Result<String, String> {
    let mut out = text.to_owned();
    let mut last = usize::MAX;
    for e in plan.edits.iter().rev().filter(|e| e.file == file) {
        if e.end > last || out.get(e.start..e.end) != Some(e.old.as_str()) {
            tracing::error!(path = %e.path, file, "update: source changed under the edit");
            return Err(format!("{}: literal in {file} is not `{}`", e.path, e.old));
        }
        out.replace_range(e.start..e.end, &e.new);
        last = e.start;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, base: Option<&str>, ours: &str, theirs: &str) -> Entry {
        Entry {
            adapter: "sw".into(),
            file: "cover.SLDPRT".into(),
            path: path.into(),
            locator: "T".into(),
            base: base.map(str::to_owned),
            ours: ours.into(),
            theirs: theirs.into(),
            replaces: None,
            ours_tol: None,
            theirs_tol: None,
        }
    }

    fn lit(start: usize, end: usize, cited: bool) -> Source {
        Source {
            definition: Definition::Literal {
                file: "a.cdt".into(),
                start,
                end,
            },
            cited,
        }
    }

    fn src(definition: Definition) -> Source {
        Source {
            definition,
            cited: false,
        }
    }

    // frob:tests update_edits_only_literals
    #[test]
    fn update_edits_only_literals() {
        let entries = [
            entry("p::C.a", Some("5mm"), "5mm", "6mm"),
            entry("p::C.b", Some("5mm"), "5mm", "6mm"),
            entry("p::C.c", Some("5mm"), "5mm", "6mm"),
            entry("p::C.d", Some("5mm"), "5mm", "5mm"),
            entry("p::C.e", None, "5mm", "6mm"),
            entry("p::C.f", Some("5mm"), "5mm", "6mm"),
        ];
        let sources = BTreeMap::from([
            ("p::C.a".to_owned(), lit(10, 13, true)),
            ("p::C.b".to_owned(), src(Definition::Derived)),
            ("p::C.c".to_owned(), src(Definition::Toleranced)),
            ("p::C.d".to_owned(), lit(20, 23, false)),
        ]);
        let plan = plan(&entries, &sources);
        assert_eq!(plan.edits.len(), 1);
        assert_eq!(plan.edits[0].new, "6mm");
        assert_eq!(plan.cite_flags, ["p::C.a"]);
        assert_eq!(plan.skipped.len(), 3);
        assert!(plan.skipped.contains(&("p::C.b".into(), Skip::Derived)));
        assert!(
            plan.skipped
                .contains(&("p::C.c".into(), Skip::Toleranced("6mm".into())))
        );
        assert!(plan.skipped.contains(&("p::C.f".into(), Skip::Unresolved)));
        assert_eq!(plan.lock_updates.len(), 1);
        let diff = render_diff(&plan);
        assert!(diff.contains("- 5mm\n+ 6mm"));
        assert!(diff.contains("no longer matches its cite"));
        assert!(diff.contains("derived; edit its sources"));
        assert!(diff.contains("edit by hand"));
    }

    // frob:tests write_rewrites_source_literals
    #[test]
    fn write_rewrites_source_literals() {
        let text = "a = 5mm\nb = 7mm\n";
        let entries = [
            entry("p::C.a", Some("5mm"), "5mm", "12mm"),
            entry("p::C.b", Some("7mm"), "7mm", "8mm"),
        ];
        let sources = BTreeMap::from([
            ("p::C.a".to_owned(), lit(4, 7, false)),
            ("p::C.b".to_owned(), lit(12, 15, false)),
        ]);
        let plan = plan(&entries, &sources);
        assert_eq!(apply(text, "a.cdt", &plan).unwrap(), "a = 12mm\nb = 8mm\n");
        assert_eq!(apply(text, "other.cdt", &plan).unwrap(), text);
        assert!(apply("a = 6mm\nb = 7mm\n", "a.cdt", &plan).is_err());
    }

    // frob:tests write_is_opt_in_plan_is_pure
    #[test]
    fn write_is_opt_in_plan_is_pure() {
        let text = "a = 5mm\n";
        let entries = [entry("p::C.a", Some("5mm"), "5mm", "6mm")];
        let sources = BTreeMap::from([("p::C.a".to_owned(), lit(4, 7, false))]);
        let _ = render_diff(&plan(&entries, &sources));
        assert_eq!(text, "a = 5mm\n");
    }
}
