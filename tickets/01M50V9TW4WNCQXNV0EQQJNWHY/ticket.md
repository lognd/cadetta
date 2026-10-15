+++
id = "01M50V9TW4WNCQXNV0EQQJNWHY"
title = "Fix red conformance: geom-sign-write-and-read frame form, read-never-raises-q001 diagnostics_exclude"
type = "bug"
category = "todo"
priority = "high"
points = 2
reporter = "lognd"
created = "2026-10-15T22:35:38Z"
updated = "2026-10-15T22:35:38Z"
scope = ["crates/cadetta-adapter/src/geometry_plan.rs", "crates/cadetta-cli/tests/conformance/verify.rs"]

[[acceptance]]
text = "parse_source_form accepts the frame form and plans it as a datum frame binding (adapters.md 3.4)"
bound = false

[[acceptance]]
text = "the conformance verifier observes diagnostics_exclude: none of the listed codes may be produced"
bound = false
+++

Main conformance red: adapters/geom-sign-write-and-read (unknown source form frame) and adapters/read-never-raises-q001 (diagnostics_exclude unobserved).
