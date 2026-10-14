+++
id = "01M4SWN7GJW88YVBJ2XVR00EKX"
title = "impact --since and --worktree"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:39Z"
updated = "2026-10-14T13:30:08Z"
persona = "design engineer"
capability = "review a change before merging"
idempotency_key = "replan14:ob-since"
labels = ["creates:crates/cadetta-obligations/src/impact_since.rs", "creates:crates/cadetta-cli/src/cmd/impact_since.rs", "creates:docs/schemas/impact-since.json", "creates:bindings/python/python/cadetta/models/impact_since.py"]
scope = ["crates/cadetta-obligations/src/impact_since.rs", "crates/cadetta-obligations/src/lib.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/cmd/impact_swap_report.rs", "docs/spec/obligations.md", "docs/spec/config-cli.md", "crates/cadetta-cli/src/cmd/impact_since.rs", "docs/schemas/impact-since.json", "bindings/python/python/cadetta/project.py", "bindings/python/python/cadetta/queries.py", "bindings/python/scripts/gen_models.py", "bindings/python/tests/test_queries.py", "bindings/python/python/cadetta/models/impact_since.py"]

[[links]]
kind = "blocked-by"
target = "01M4ST8F7B9VYMFSCJD8MVBJNS"

[[acceptance]]
text = "Given two evaluations (git REV or worktree), when diffed, then changed nodes, stale obligations, flipped verdicts and CAD files to re-sync are reported (obligations.md 5; config-cli.md 3)"
bound = true
+++
