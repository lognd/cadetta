+++
id = "01M51S49V6TVMESDJWKRE93MRS"
title = "Roll-up over misspelled member must report E002, not internal EVAL"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-16T07:16:54Z"
updated = "2026-10-16T07:16:54Z"
scope = ["crates/cadetta-sema/**"]
+++

Per docs/spec/stdlib.md Conformance decisions (~MEM04J7): misspelled roll-up member is E002 at the roll-up expression, never internal EVAL.
