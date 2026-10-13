+++
id = "01M4STAQ6562JJ6XGWKZ48GXKX"
title = "Conformance fixtures for quantities"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:57Z"
updated = "2026-10-13T16:14:13Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for quantities"
labels = ["creates:tests/conformance/quantities/**"]
scope = ["tests/conformance/quantities/**"]

[[links]]
kind = "blocks"
target = "01M4STCD6YD5DZFVD9RJDN02RY"

[[acceptance]]
text = "Given docs/spec/quantities.md, when fixtures are written, then every normative rule and every diagnostic code in quantities has a must-fire and a must-not-fire fixture under tests/conformance/quantities/ (quantities.md)"
bound = false

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = false
+++
