+++
id = "01M4SWP7J9W1908KF7HXSPV2K0"
title = "Conformance run for geometry"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:45:11Z"
updated = "2026-10-13T05:45:13Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan14:cr-geo"
labels = ["creates:tests/conformance_geometry.rs"]
scope = ["tests/conformance_geometry.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWP7CNYKZ9QM9F2ZFM3633"

[[acceptance]]
text = "Given tests/conformance/geometry/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/geometry.md)"
bound = false
+++
