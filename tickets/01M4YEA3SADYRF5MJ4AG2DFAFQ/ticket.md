+++
id = "01M4YEA3SADYRF5MJ4AG2DFAFQ"
title = "U002 and I004 facts: resolution-based use, members, parameterized and is_root"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T00:10:07Z"
updated = "2026-10-15T11:04:36Z"
scope = ["crates/cadetta-cli/src/lint_facts.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/lint_entry_points.rs", "docs/spec/lints.md"]

[[links]]
kind = "blocked-by"
target = "01M4TA3CABXHEZFA7BXA8J3XJW"

[[acceptance]]
text = "U002 uses name resolution, so a shadowing mention does not count as use"
bound = true

[[acceptance]]
text = "U002 reports an unused private param/let member of a component"
bound = true

[[acceptance]]
text = "I004 fires only for a model/assembly with a param lacking a default that no part instantiates"
bound = true
+++

Follow-up of ~A8J3XJW. U002 'used' is a lexical identifier count; priv Member rows are not produced; ItemFacts.parameterized/is_root are unset so I004 uses check_roots only. Drive these from sema name resolution.
