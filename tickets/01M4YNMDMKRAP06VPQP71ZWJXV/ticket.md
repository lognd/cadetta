+++
id = "01M4YNMDMKRAP06VPQP71ZWJXV"
title = "IR decoder so lint FILE runs all IR-only rules (D001, D002, F004, M, N, Q006, S)"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:18:04Z"
updated = "2026-10-15T02:27:15Z"
labels = ["creates:crates/cadetta-ir/src/decode.rs", "creates:crates/cadetta-cli/tests/ir_lint_rules.rs", "creates:changelog.d/01M4YNMDMKRAP06VPQP71ZWJXV.added.md"]
scope = ["crates/cadetta-ir/src/lib.rs", "crates/cadetta-cli/src/cmd/ir_lint.rs", "crates/cadetta-cli/src/pipeline.rs", "docs/spec/config-cli.md", "crates/cadetta-ir/src/decode.rs", "crates/cadetta-cli/tests/ir_lint_rules.rs", "changelog.d/01M4YNMDMKRAP06VPQP71ZWJXV.added.md"]

[[acceptance]]
text = "IR JSON decodes into cadetta_ir::model::Project and round-trips"
bound = true

[[acceptance]]
text = "lint FILE runs D001, D002, D003, F004, M, N, Q006 and S over the decoded model"
bound = true
+++

lint FILE (T 5ZFM9K6) only runs D003 via a skeleton decode; the value-reading rules need a full cadetta.ir/1 JSON decoder into cadetta_ir::model.
