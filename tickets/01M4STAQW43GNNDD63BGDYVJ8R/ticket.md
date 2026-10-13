+++
id = "01M4STAQW43GNNDD63BGDYVJ8R"
title = "Conformance fixtures for stdlib"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:58Z"
updated = "2026-10-13T16:46:03Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for stdlib"
labels = ["creates:tests/conformance/stdlib/**"]
scope = ["tests/conformance/stdlib/**", "tests/conformance/known-gaps.toml", "docs/spec/stdlib.md"]

[[links]]
kind = "blocks"
target = "01M4STCFHFMG7999J9WDPK1BC3"

[[acceptance]]
text = "Given docs/spec/stdlib.md, when fixtures are written, then every normative rule and every diagnostic code in stdlib has a must-fire and a must-not-fire fixture under tests/conformance/stdlib/ (stdlib.md)"
bound = false

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = false
+++
