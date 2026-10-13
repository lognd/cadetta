+++
id = "01M4SWP7EKGJHBGKHZG5RCYHAJ"
title = "Conformance fixtures for emitters/json"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:45:11Z"
updated = "2026-10-13T17:37:30Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan14:cf-json"
labels = ["creates:tests/conformance/emitters/json/**"]
scope = ["tests/conformance/emitters/json/**"]

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = true

[[acceptance]]
text = "Given docs/spec/emitters/json.md, when fixtures are written, then every normative rule and every diagnostic code has a must-fire and a must-not-fire fixture under tests/conformance/emitters/json/ (emitters/json.md)"
bound = true
+++
