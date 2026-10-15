+++
id = "01M4TA3CABXHEZFA7BXA8J3XJW"
title = "Wire I001, I004, U002, require and allow into cadetta check"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-15T01:03:46Z"
labels = ["creates:crates/cadetta-cli/src/lint_facts.rs", "creates:crates/cadetta-cli/tests/lint_entry_points.rs", "creates:changelog.d/01M4TA3CABXHEZFA7BXA8J3XJW.fixed.md"]
scope = ["crates/cadetta-lint", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/src/lib.rs", "crates/cadetta-cli/src/cmd/check_watch.rs", "docs/spec/lints.md", "crates/cadetta-cli/src/lint_facts.rs", "crates/cadetta-cli/tests/lint_entry_points.rs", "changelog.d/01M4TA3CABXHEZFA7BXA8J3XJW.fixed.md", "crates/cadetta-cli/tests/lsp.rs"]

[[acceptance]]
text = "I001 fires through check when an implementer lacks an interface member"
bound = true

[[acceptance]]
text = "I001 is quiet when the implementer has every member"
bound = true

[[acceptance]]
text = "I004 fires through check for a public parameterized root"
bound = true

[[acceptance]]
text = "U002 fires through check for an unused private item and not a used one"
bound = true

[[acceptance]]
text = "An #[allow] attribute suppresses its diagnostic through the engine"
bound = true

[[acceptance]]
text = "require paths are collected so requires are never suppressed"
bound = true
+++

found while working ~E631Y52. M, N, R, U002 facts, interface check_members/check_roots; waivers, requires, interfaces not lowered.
