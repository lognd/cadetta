+++
id = "01M4STCCZ58C875BTMQ9E54BMT"
title = "Lints O001-O003 for obligations"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:04:52Z"
updated = "2026-10-13T12:42:29Z"
persona = "design engineer"
capability = "see stale, open and unattested obligations as diagnostics"
idempotency_key = "replan13:Lints O001-O003 for obligations"
labels = ["creates:crates/cadetta-lint/src/rules/obligation.rs"]
scope = ["crates/cadetta-lint/src/rules/obligation.rs", "crates/cadetta-lint/src/rules/mod.rs"]

[[links]]
kind = "blocks"
target = "01M4STCFMHY2RXD7WFF3Q9FRJD"

[[acceptance]]
text = "Given O001 to O003 in lints.md, when linted, then each fires per the catalog, O001 an error for roles required for check (lints.md O)"
bound = true

[[acceptance]]
text = "Given a current attestation, when linted, then none fires (lints.md O)"
bound = true
+++
