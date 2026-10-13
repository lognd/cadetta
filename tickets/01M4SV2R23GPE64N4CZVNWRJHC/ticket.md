+++
id = "01M4SV2R23GPE64N4CZVNWRJHC"
title = "Conformance fixtures for bindings"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:17:04Z"
updated = "2026-10-13T17:48:09Z"
persona = "spec author"
capability = "verify an implementation against the spec"
labels = ["creates:tests/conformance/bindings/**"]
scope = ["tests/conformance/bindings/**"]

[[links]]
kind = "blocked-by"
target = "01M4STAQ2HJ1561N1M6VJHY70G"

[[acceptance]]
text = "Given docs/spec/bindings.md, when fixtures are written, then every normative rule and every diagnostic code in bindings has a must-fire and a must-not-fire fixture under tests/conformance/bindings/ (bindings.md)"
bound = true

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = true

[[acceptance]]
text = "Given draft 1.4 revisions of the document, when fixtures are rewritten, then every rule added or changed in 1.4 has fixtures (docs/spec/README.md)"
bound = true
+++
