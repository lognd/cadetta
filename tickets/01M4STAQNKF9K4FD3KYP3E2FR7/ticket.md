+++
id = "01M4STAQNKF9K4FD3KYP3E2FR7"
title = "Conformance fixtures for obligations"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:58Z"
updated = "2026-10-13T05:03:58Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for obligations"
labels = ["creates:tests/conformance/obligations/**"]
scope = ["tests/conformance/obligations/**"]

[[acceptance]]
text = "Given docs/spec/obligations.md, when fixtures are written, then every normative rule and every diagnostic code in obligations has a must-fire and a must-not-fire fixture under tests/conformance/obligations/ (obligations.md)"
bound = false

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = false
+++
