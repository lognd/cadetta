+++
id = "01M4Y66FVYYDJH0FJ16F1T08H3"
title = "Remove span-heuristic diagnostic attribution in the check pipeline"
type = "chore"
category = "done"
outcome = "done"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-14T21:48:19Z"
updated = "2026-10-16T01:38:38Z"
scope = ["crates/cadetta-cli", "crates/cadetta-sema", "changelog.d", "crates/cadetta-cli/**"]

[[acceptance]]
text = "Every diagnostic reaching the pipeline carries a ModuleKey"
bound = true

[[acceptance]]
text = "SourceIndex::attribute is removed"
bound = true
+++

Follow-up of ~6KASY82. SourceIndex::attribute still guesses a file from the span for diagnostics with module None (lowering, lint and other producers). Stamp a ModuleKey on every remaining producer, then delete attribute and make file_for require a module.
