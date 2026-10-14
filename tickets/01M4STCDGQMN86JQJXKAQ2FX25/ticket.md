+++
id = "01M4STCDGQMN86JQJXKAQ2FX25"
title = "Conformance run for language"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T05:04:53Z"
updated = "2026-10-14T05:56:24Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan13:Conformance run for language"
labels = ["creates:tests/conformance_language.rs"]
scope = ["tests/conformance_language.rs", "crates/cadetta-cli/Cargo.toml"]

[[acceptance]]
text = "Given tests/conformance/language/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/language.md)"
bound = true
+++
