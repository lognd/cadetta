+++
id = "01M4ZKRTB105N3QFA1RQ8C7WFF"
title = "U002 rows for unused private part and datum members"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T11:04:46Z"
updated = "2026-10-15T17:55:33Z"
scope = ["crates/cadetta-cli/src/lint_facts.rs", "crates/cadetta-cli/tests/lint_entry_points.rs", "docs/spec/lints.md"]

[[acceptance]]
text = "An unused private part member yields a U002 Member row and a used one does not"
bound = true

[[acceptance]]
text = "An unused private datum member yields a U002 Member row and a read one does not"
bound = false
+++

found while working ~G2DFAFQ: item_facts emits Member rows only for priv param/let members; part/datum members are excluded to avoid false positives and need a use analysis.
