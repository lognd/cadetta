+++
id = "01M4TZNG5H70B435540CN4M56T"
title = "Import CLI accept: use cadetta_adapter::import::accept::plan"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T15:56:28Z"
updated = "2026-10-15T02:57:40Z"
scope = ["crates/cadetta-cli/src/cmd/import.rs"]

[[acceptance]]
text = "cmd/import.rs accept uses accept::plan; local quote/binding_table removed; accept stays idempotent"
bound = false
+++

found while working JXRXM2Y: cmd/import.rs duplicates binding_table/quote and the planning loop now in adapter import/accept.rs. Replace with accept::plan.
