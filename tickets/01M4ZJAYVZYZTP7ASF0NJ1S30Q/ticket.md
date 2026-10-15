+++
id = "01M4ZJAYVZYZTP7ASF0NJ1S30Q"
title = "Ack for waiver: and sync: obligations"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T10:39:43Z"
updated = "2026-10-15T11:39:51Z"
scope = ["crates/cadetta-cli/src/cmd/ack.rs", "crates/cadetta-cli/src/universe.rs", "docs/spec/obligations.md"]

[[acceptance]]
text = "cadetta ack waiver:GLOB records the waiver at its targets' closure digest and removes it from the open obligations"
bound = true
+++

found while working TNGHA7Q: ack_candidates emits no Subject::Waiver and sync/waiver rows have no attest path
