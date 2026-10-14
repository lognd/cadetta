+++
id = "01M4SWP7NQ1KSHZ7TTSR6ZMFBF"
title = "Conformance run for emitters/csv"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-14T12:36:37Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan14:cr-csv"
labels = ["creates:tests/conformance_emitters_csv.rs"]
scope = ["tests/conformance_emitters_csv.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWP7GFWFNXTDSZC9JX3J6F"

[[acceptance]]
text = "Given tests/conformance/emitters/csv/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/emitters/csv.md)"
bound = false
+++
