+++
id = "01M4STCFE8P4ABFFCAFEX8DC9C"
title = "Conformance run for solvers"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:04:55Z"
updated = "2026-10-14T12:19:57Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan13:Conformance run for solvers"
labels = ["creates:tests/conformance_solvers.rs"]
scope = ["tests/conformance_solvers.rs", "crates/cadetta-cli/Cargo.toml", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Given tests/conformance/solvers/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/solvers.md)"
bound = false
+++
