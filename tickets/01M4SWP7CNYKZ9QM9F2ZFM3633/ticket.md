+++
id = "01M4SWP7CNYKZ9QM9F2ZFM3633"
title = "Conformance fixtures for geometry"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:45:11Z"
updated = "2026-10-13T17:07:31Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan14:cf-geo"
labels = ["creates:tests/conformance/geometry/**"]
scope = ["tests/conformance/geometry/**", "tests/conformance/known-gaps.toml", "docs/spec/geometry.md"]

[[acceptance]]
text = "Given docs/spec/geometry.md, when fixtures are written, then every normative rule and every diagnostic code has a must-fire and a must-not-fire fixture under tests/conformance/geometry/ (geometry.md)"
bound = false

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = false
+++
