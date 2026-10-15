+++
id = "01M4ZKRTB105N3QFA1RQ8C7WFF"
title = "U002 rows for unused private part and datum members"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T11:04:46Z"
updated = "2026-10-15T11:04:46Z"
scope = ["crates/cadetta-cli/src/lint_facts.rs"]
+++

found while working ~G2DFAFQ: item_facts emits Member rows only for priv param/let members; part/datum members are excluded to avoid false positives and need a use analysis.
