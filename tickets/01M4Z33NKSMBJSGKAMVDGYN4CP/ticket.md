+++
id = "01M4Z33NKSMBJSGKAMVDGYN4CP"
title = "sema: evaluate a fit's .clearance as a member readable by checks"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T06:13:35Z"
updated = "2026-10-15T06:23:57Z"
idempotency_key = "a0fq-fitmember"
scope = ["crates/cadetta-sema/**", "docs/spec/language.md", "tests/conformance/known-gaps.toml", "changelog.d/01M4Z33NKSMBJSGKAMVDGYN4CP.fixed.md"]

[[acceptance]]
text = "The fit-clearance-member fixture passes and its known-gaps entry is removed"
bound = true
+++

Split from ~A0FQTVX. Fixture language/fit-clearance-member: check reports no C001 because fit analysis (sema/src/fits.rs, fitderive.rs) runs after member values and eval.rs cannot read it. Remove the known-gaps entry.
