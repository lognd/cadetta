+++
id = "01M4STAQNKF9K4FD3KYP3E2FR7"
title = "Conformance fixtures for obligations"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 8
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:58Z"
updated = "2026-10-13T16:34:38Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for obligations"
labels = ["creates:tests/conformance/obligations/**"]
scope = ["tests/conformance/obligations/**", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocks"
target = "01M4STCF8FJ9HGB7EZ37MC24EF"

[[acceptance]]
text = "Given docs/spec/obligations.md, when fixtures are written, then every normative rule and every diagnostic code in obligations has a must-fire and a must-not-fire fixture under tests/conformance/obligations/ (obligations.md)"
bound = true

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = true

[[acceptance]]
text = "Given draft 1.4 revisions of the document, when fixtures are rewritten, then every rule added or changed in 1.4 has fixtures (docs/spec/README.md)"
bound = true
+++
