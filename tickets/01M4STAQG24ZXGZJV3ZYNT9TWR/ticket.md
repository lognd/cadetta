+++
id = "01M4STAQG24ZXGZJV3ZYNT9TWR"
title = "Conformance fixtures for adapters"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 8
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:57Z"
updated = "2026-10-13T16:37:53Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for adapters"
labels = ["creates:tests/conformance/adapters/**"]
scope = ["tests/conformance/adapters/**", "tests/conformance/known-gaps.toml", "docs/spec/adapters.md"]

[[links]]
kind = "blocks"
target = "01M4STCF3A4JQGPYSE7BBHK4BD"

[[acceptance]]
text = "Given docs/spec/adapters.md, when fixtures are written, then every normative rule and every diagnostic code in adapters has a must-fire and a must-not-fire fixture under tests/conformance/adapters/ (adapters.md)"
bound = true

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = true

[[acceptance]]
text = "Given draft 1.4 revisions of the document, when fixtures are rewritten, then every rule added or changed in 1.4 has fixtures (docs/spec/README.md)"
bound = true
+++
