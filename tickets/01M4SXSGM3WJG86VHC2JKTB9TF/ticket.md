+++
id = "01M4SXSGM3WJG86VHC2JKTB9TF"
title = "Conformance fixtures for packages"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T06:04:28Z"
updated = "2026-10-13T06:04:36Z"
persona = "spec author"
capability = "verify an implementation against packages.md"
labels = ["creates:tests/conformance/packages/**"]
scope = ["tests/conformance/packages/**"]

[[links]]
kind = "blocked-by"
target = "01M4STAQ2HJ1561N1M6VJHY70G"

[[acceptance]]
text = "Given docs/spec/packages.md, when fixtures are written, then every normative rule and every diagnostic code (E012-E017, U002) has a must-fire and a must-not-fire fixture under tests/conformance/packages/ (packages.md 7)"
bound = false

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = false
+++
