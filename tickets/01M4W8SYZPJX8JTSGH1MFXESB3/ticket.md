+++
id = "01M4W8SYZPJX8JTSGH1MFXESB3"
title = "N003 fires on std interface member tap_lead_in (found while working ~3G80Q7F; see tests/conformance/stdlib check-not-bottomed expect.json note)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-14T03:55:26Z"
updated = "2026-10-15T07:37:04Z"
scope = ["crates/cadetta-lint/src/rules/n.rs", "docs/spec/lints.md", "tests/conformance/stdlib/check-not-bottomed.expect.json", "changelog.d/01M4W8SYZPJX8JTSGH1MFXESB3.fixed.md"]

[[acceptance]]
text = "N003 does not fire on a dimensionless member whose name ends in a unit-like word (tap_lead_in = 3)"
bound = false
+++
