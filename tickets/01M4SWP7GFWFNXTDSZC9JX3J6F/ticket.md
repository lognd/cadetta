+++
id = "01M4SWP7GFWFNXTDSZC9JX3J6F"
title = "Conformance fixtures for emitters/csv"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:45:11Z"
updated = "2026-10-13T05:45:25Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan14:cf-csv"
labels = ["creates:tests/conformance/emitters/csv/**"]
scope = ["tests/conformance/emitters/csv/**"]

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = false

[[acceptance]]
text = "Given docs/spec/emitters/csv.md, when fixtures are written, then every normative rule and every diagnostic code has a must-fire and a must-not-fire fixture under tests/conformance/emitters/csv/ (emitters/csv.md)"
bound = false
+++
