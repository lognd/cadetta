+++
id = "01M4TRMJMNWCMHCE2H3BJF002X"
title = "Selection-aware solver input evaluation for solve --variants"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T13:53:37Z"
updated = "2026-10-15T01:58:58Z"
scope = ["crates/cadetta-sema/src/eval.rs", "crates/cadetta-cli/src/cmd/solve.rs", "crates/cadetta-sema/src/geom.rs", "crates/cadetta-sema/src/resolve.rs", "crates/cadetta-sema/tests/interface_slots.rs", "docs/spec/solvers.md", "changelog.d/01M4TRMJMNWCMHCE2H3BJF002X.fixed.md"]

[[links]]
kind = "blocked-by"
target = "01M4Y77276XRHD2NDXHJMCB8NQ"

[[acceptance]]
text = "Reads through a part slot under a (slot, alt) selection reach the alternate instead of the primary"
bound = true

[[acceptance]]
text = "solve --variants evaluates solver inputs per alternate, runs once per distinct changed input set and stores results keyed SLOT=ALT"
bound = true
+++

found while working ~1NJKGFE: Evaluator::eval_input cannot re-evaluate under a slot selection, so cmd/solve.rs variant_requests returns no variants
