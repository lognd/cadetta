+++
id = "01M4SWP7J9W1908KF7HXSPV2K0"
title = "Conformance run for geometry"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:45:11Z"
updated = "2026-10-14T12:32:12Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan14:cr-geo"
labels = ["creates:tests/conformance_geometry.rs"]
scope = ["tests/conformance_geometry.rs", "crates/cadetta-cli/Cargo.toml", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocked-by"
target = "01M4SWP7CNYKZ9QM9F2ZFM3633"

[[acceptance]]
text = "Given tests/conformance/geometry/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/geometry.md)"
bound = true
+++
