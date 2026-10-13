+++
id = "01M4ST8FEBCF9PC9SR0ZKTYT70"
title = "Query schemas impact.json and compare.json"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:44Z"
updated = "2026-10-13T05:44:39Z"
persona = "tool integrator"
capability = "consume query output from other tools such as frob"
idempotency_key = "replan13:Query schemas impact.json and compare.json"
labels = ["creates:docs/schemas/impact.json", "creates:docs/schemas/compare.json", "creates:crates/cadetta-obligations/src/schema.rs"]
scope = ["docs/schemas/impact.json", "docs/schemas/compare.json", "crates/cadetta-obligations/src/schema.rs"]

[[acceptance]]
text = "Given the Rust query types, when schemas are generated, then docs/schemas/impact.json and compare.json match the committed files (CI check) (obligations.md 5)"
bound = false

[[acceptance]]
text = "Given any query with `--format json`, when run, then its output validates against the matching schema (obligations.md 5)"
bound = false

[[acceptance]]
text = "Given the verbs obligations, why, swap, lock and diag, when emitting JSON, then each has a generated schema in docs/schemas (bindings.md 1; obligations.md 5)"
bound = false
+++
