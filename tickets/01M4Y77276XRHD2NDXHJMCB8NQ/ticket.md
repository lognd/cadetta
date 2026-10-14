+++
id = "01M4Y77276XRHD2NDXHJMCB8NQ"
title = "Reads through a part slot (wheel.load_rating) fail with E002; slot primary is never consulted"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-14T22:06:07Z"
updated = "2026-10-14T22:06:07Z"
scope = ["crates/cadetta-sema/src/geom.rs", "crates/cadetta-sema/src/eval.rs", "crates/cadetta-sema/src/resolve.rs"]

[[acceptance]]
text = "Reading a member through an interface-typed part slot evaluates against the slot's primary component (check load_ok: wheel.load_rating >= 440N with part wheel: Caster = Good yields a verdict, no E002)"
bound = false
+++

Found while working ~Q1ZBN9Y. geom.rs through_parts steps a part read to the slot's declared TYPE (interface Caster), whose members have no values, so analyze emits E002 'Caster has no member load_rating' for a slot declared 'part wheel[4]: Caster = Good alt [Bad]'. Repro: tests/conformance/solvers/variants-flag-solves-alternates.must-fire. The evaluator must resolve the slot to its primary Ctor (and, for variant evaluation, to a chosen alternate) before reading the member. BLOCKER for ~Q1ZBN9Y's variant half; prerequisite of the variant-evaluation ticket.
