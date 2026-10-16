+++
id = "01M51S49V6TVMESDJWKRE93MRS"
title = "Roll-up over misspelled member must report E002, not internal EVAL"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T07:16:54Z"
updated = "2026-10-16T10:34:38Z"
scope = ["crates/cadetta-sema/**", "tests/conformance/language/eval-rollup-no-contributor.*", "tests/conformance/language/e002-rollup-misspelled-member.*", "tests/conformance/stdlib/commercial-misspelling-not-a-member.*"]

[[acceptance]]
text = "A roll-up naming a member no part type declares reports E002, not EVAL"
bound = true
+++

Per docs/spec/stdlib.md Conformance decisions (~MEM04J7): misspelled roll-up member is E002 at the roll-up expression, never internal EVAL.
