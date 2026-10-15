+++
id = "01M4Z33NNYYPECFPASHMRQXAZQ"
title = "sema: .drawn readable in checks (process-drawn-value)"
type = "task"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T06:13:36Z"
updated = "2026-10-15T06:13:36Z"
idempotency_key = "a0fq-drawn"
scope = ["crates/cadetta-sema/**"]

[[acceptance]]
text = "The process-drawn-value fixture passes and its known-gaps entry is removed"
bound = false
+++

Split from ~A0FQTVX. Fixture language/process-drawn-value: apply_chain in sema/src/eval.rs never reads .drawn; logic exists in process.rs (Processes::apply, ProcessOutcome::read). Wire it; remove the known-gaps entry.
