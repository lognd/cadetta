+++
id = "01M4Z3FGQHHAPQ3N8VCDEY1G2X"
title = "language: wire I002/I003 and part alt slots into check (E002 on slot members, per-alternate viability)"
type = "task"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-15T06:20:04Z"
updated = "2026-10-15T06:50:58Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-cli/**", "crates/cadetta-lint/**", "tests/conformance/known-gaps.toml", "docs/spec/lints.md"]

[[acceptance]]
text = "An interface bound (load_rating >= 400N) is a readable member of the interface, so no E002"
bound = true

[[acceptance]]
text = "Per-alternate verdicts reach lowering so I002 is emitted at the variant key (i002-never-cross-product)"
bound = true
+++

Found in ~8NXQSDY. Fixtures i002-alternate-not-viable, i002-never-cross-product, i003-alternate-undecided: 'part wheel[4]: Caster = Good alt [Bad]' with 'check load_ok: wheel.load_rating >= 440N' yields E002 (unresolved wheel.load_rating) and I002/I003 never emitted (lint rules exist in cadetta-lint/src/rules/interface.rs but no per-alternate verdicts are fed from check). cadetta-variants API (slot, viability, evaluate_variants) is already adequate; the gap is wiring in sema/cli. Path for I002 is Rover/back=acme.demo::B2 (one alternate at a time).
