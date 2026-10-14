+++
id = "01M4STCFHFMG7999J9WDPK1BC3"
title = "Conformance run for stdlib"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:04:55Z"
updated = "2026-10-14T12:26:13Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan13:Conformance run for stdlib"
labels = ["creates:tests/conformance_stdlib.rs"]
scope = ["tests/conformance_stdlib.rs", "crates/cadetta-cli/Cargo.toml", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Given tests/conformance/stdlib/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/stdlib.md)"
bound = true
+++
