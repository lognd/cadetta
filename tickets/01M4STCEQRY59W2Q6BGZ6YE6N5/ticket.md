+++
id = "01M4STCEQRY59W2Q6BGZ6YE6N5"
title = "Conformance run for ir"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T05:04:54Z"
updated = "2026-10-13T05:04:54Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan13:Conformance run for ir"
labels = ["creates:tests/conformance_ir.rs"]
scope = ["tests/conformance_ir.rs"]

[[acceptance]]
text = "Given tests/conformance/ir/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/ir.md)"
bound = false
+++
