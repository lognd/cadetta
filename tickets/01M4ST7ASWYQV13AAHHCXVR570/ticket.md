+++
id = "01M4ST7ASWYQV13AAHHCXVR570"
title = "Functions, enums, match and standard functions"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T08:15:06Z"
persona = "design engineer"
capability = "write pure helper functions and exhaustive matches"
idempotency_key = "replan13:Functions, enums, match and standard functions"
labels = ["creates:crates/cadetta-sema/src/{func,enums}.rs"]
scope = ["crates/cadetta-sema/src/{func,enums}.rs", "crates/cadetta-sema/src/resolve.rs", "crates/cadetta-sema/src/lib.rs", "crates/cadetta-sema/src/eval.rs", "docs/spec/quantities.md"]

[[acceptance]]
text = "Given `fn name(a: Kind) -> Kind { let x = ...; expr }`, when evaluated, then it is pure and total; recursion is rejected (language.md 3.8)"
bound = true

[[acceptance]]
text = "Given a non-exhaustive `match`, when checked, then an error names the missing variants (language.md 3.8)"
bound = false

[[acceptance]]
text = 'Given the standard functions min max abs sqrt sin cos tan atan2 round_to ceil_to floor_to floor ceil round count and fmt, when evaluated, then each follows the cited rules; `fmt("{} x {}", thread, length)` renders by display rules (language.md 3.8)'
bound = false
+++
