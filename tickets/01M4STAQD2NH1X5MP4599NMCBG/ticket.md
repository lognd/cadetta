+++
id = "01M4STAQD2NH1X5MP4599NMCBG"
title = "Conformance fixtures for ir"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:57Z"
updated = "2026-10-13T16:19:41Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for ir"
labels = ["creates:tests/conformance/ir/**"]
scope = ["tests/conformance/ir/**", "docs/spec/ir.md"]

[[links]]
kind = "blocks"
target = "01M4STCEQRY59W2Q6BGZ6YE6N5"

[[acceptance]]
text = "Given docs/spec/ir.md, when fixtures are written, then every normative rule and every diagnostic code in ir has a must-fire and a must-not-fire fixture under tests/conformance/ir/ (ir.md)"
bound = false

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = false

[[acceptance]]
text = "Given ir.md 1.4 changes (Dim vector, geometry entities, path grammar, SLOT=ALT), when fixtures are rewritten, then each has fixtures (ir.md 2-3)"
bound = false
+++
