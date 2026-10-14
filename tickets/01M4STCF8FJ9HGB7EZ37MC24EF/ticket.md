+++
id = "01M4STCF8FJ9HGB7EZ37MC24EF"
title = "Conformance run for obligations"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:04:55Z"
updated = "2026-10-14T06:05:45Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan13:Conformance run for obligations"
labels = ["creates:tests/conformance_obligations.rs", "creates:crates/cadetta-cli/Cargo.toml"]
scope = ["tests/conformance_obligations.rs", "crates/cadetta-cli/Cargo.toml"]

[[acceptance]]
text = "Given tests/conformance/obligations/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/obligations.md)"
bound = true
+++
