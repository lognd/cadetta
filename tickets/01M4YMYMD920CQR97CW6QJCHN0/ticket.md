+++
id = "01M4YMYMD920CQR97CW6QJCHN0"
title = "Evaluate variants for L001 alternate viability in check"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:06:10Z"
updated = "2026-10-15T02:50:35Z"
scope = ["crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/lint_lifts.rs", "docs/spec/lints.md", "changelog.d/01M4YMYMD920CQR97CW6QJCHN0.fixed.md"]

[[acceptance]]
text = "L001 treats an alternate that violates a check under its variant as non-viable"
bound = true
+++

found while working ~4H7PTZX: L001/L002 approximate alternate viability as resolves-and-not-end-of-life; real viability needs variant evaluation.
