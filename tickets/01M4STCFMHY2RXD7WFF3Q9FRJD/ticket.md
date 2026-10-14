+++
id = "01M4STCFMHY2RXD7WFF3Q9FRJD"
title = "Conformance run for lints"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:04:55Z"
updated = "2026-10-14T12:29:51Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan13:Conformance run for lints"
labels = ["creates:tests/conformance_lints.rs"]
scope = ["tests/conformance_lints.rs", "crates/cadetta-cli/Cargo.toml", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Given tests/conformance/lints/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/lints.md)"
bound = true
+++
