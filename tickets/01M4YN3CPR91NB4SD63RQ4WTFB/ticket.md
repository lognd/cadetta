+++
id = "01M4YN3CPR91NB4SD63RQ4WTFB"
title = "Q002 affine multiply: also reject Temperature values held in variables/params"
type = "task"
category = "todo"
priority = "low"
points = 3
reporter = "lognd"
created = "2026-10-15T02:08:46Z"
updated = "2026-10-15T02:35:37Z"
idempotency_key = "q002-affine-vars"
scope = ["crates/cadetta-sema/src/kindcheck.rs", "docs/spec/quantities.md", "changelog.d/01M4YN3CPR91NB4SD63RQ4WTFB.fixed.md", "tests/conformance/quantities/q00*"]

[[acceptance]]
text = "Temperature held in a variable multiplied by a number reports Q002; fixtures q002-affine-multiply and q003-temperature-delta pass"
bound = true
+++

~0YYT3TP rejects multiply/divide only on direct degC/degF literals; a Temperature held in a variable or parameter passes. Needs affine tracking on values/kinds. Also replay fixtures q002-affine-multiply and q003-temperature-delta (not run by the implementer).
