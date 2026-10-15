+++
id = "01M4TRMJMNWCMHCE2H3BJF002X"
title = "Selection-aware solver input evaluation for solve --variants"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T13:53:37Z"
updated = "2026-10-15T01:53:06Z"
scope = ["crates/cadetta-sema/src/eval.rs", "crates/cadetta-cli/src/cmd/solve.rs"]

[[links]]
kind = "blocked-by"
target = "01M4Y77276XRHD2NDXHJMCB8NQ"
+++

found while working ~1NJKGFE: Evaluator::eval_input cannot re-evaluate under a slot selection, so cmd/solve.rs variant_requests returns no variants
