+++
id = "01M50QY56TMJE762QGCWA7ZYV1"
title = "Dedupe negate_text: use cadetta_adapter::geometry_plan::negate_text in cadetta-cli"
type = "task"
category = "todo"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-15T21:36:49Z"
updated = "2026-10-16T00:26:10Z"
scope = ["crates/cadetta-cli/src/**"]

[[acceptance]]
text = "cadetta-cli uses the adapter negate_text; no local duplicate"
bound = false
+++

found while working ~Y25XAXV: cadetta-cli cmd/emit.rs negate_text duplicates the new adapter geometry_plan::negate_text
