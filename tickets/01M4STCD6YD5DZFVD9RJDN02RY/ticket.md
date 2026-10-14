+++
id = "01M4STCD6YD5DZFVD9RJDN02RY"
title = "Conformance run for quantities"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T05:04:52Z"
updated = "2026-10-14T05:44:03Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan13:Conformance run for quantities"
labels = ["creates:tests/conformance_quantities.rs"]
scope = ["tests/conformance_quantities.rs", "crates/cadetta-cli/Cargo.toml"]

[[acceptance]]
text = "Given tests/conformance/quantities/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/quantities.md)"
bound = true
+++
