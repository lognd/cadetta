+++
id = "01M4STCF3A4JQGPYSE7BBHK4BD"
title = "Conformance run for adapters and sw-equations"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:04:54Z"
updated = "2026-10-14T12:13:28Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan13:Conformance run for adapters and sw-equations"
labels = ["creates:tests/conformance_adapters.rs"]
scope = ["tests/conformance_adapters.rs", "crates/cadetta-cli/Cargo.toml", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Given tests/conformance/adapters/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/adapters.md)"
bound = false
+++
