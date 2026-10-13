+++
id = "01M4STAS2X2KAHZYMFPT71T26K"
title = "Conformance fixtures for lints"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:59Z"
updated = "2026-10-13T16:47:50Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for lints"
labels = ["creates:tests/conformance/lints/**"]
scope = ["tests/conformance/lints/**", "docs/spec/lints.md"]

[[links]]
kind = "blocks"
target = "01M4STCFMHY2RXD7WFF3Q9FRJD"

[[acceptance]]
text = "Given docs/spec/lints.md, when fixtures are written, then every normative rule and every diagnostic code in lints has a must-fire and a must-not-fire fixture under tests/conformance/lints/ (lints.md)"
bound = false

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = false

[[acceptance]]
text = "Given the full catalog in lints.md, when fixtures are written, then every code in the catalog is covered (lints.md 3)"
bound = false

[[acceptance]]
text = "Given draft 1.4 revisions of the document, when fixtures are rewritten, then every rule added or changed in 1.4 has fixtures (docs/spec/README.md)"
bound = false
+++
