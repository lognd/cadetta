+++
id = "01M4SWP7KZ0KW16Q0KCYE821GA"
title = "Conformance run for emitters/json"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-14T12:32:42Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan14:cr-json"
labels = ["creates:tests/conformance_emitters_json.rs"]
scope = ["tests/conformance_emitters_json.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWP7EKGJHBGKHZG5RCYHAJ"

[[acceptance]]
text = "Given tests/conformance/emitters/json/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/emitters/json.md)"
bound = false
+++
