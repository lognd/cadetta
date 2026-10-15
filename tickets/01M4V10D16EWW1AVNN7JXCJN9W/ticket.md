+++
id = "01M4V10D16EWW1AVNN7JXCJN9W"
title = "IR schema accepts flip about z; spec allows x or y only"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:19:54Z"
updated = "2026-10-15T01:57:00Z"
labels = ["creates:changelog.d/01M4V10D16EWW1AVNN7JXCJN9W.fixed.md"]
scope = ["docs/schemas/**", "crates/cadetta-ir/src/schema.rs", "changelog.d/01M4V10D16EWW1AVNN7JXCJN9W.fixed.md"]

[[acceptance]]
text = "IR schema flip accepts x or y and rejects z"
bound = false
+++

found while working ~99NMCBG: ir.md 3 wire form: flip takes x|y; schema reuses Axis (x|y|z). Fixture op-flip-axis.
