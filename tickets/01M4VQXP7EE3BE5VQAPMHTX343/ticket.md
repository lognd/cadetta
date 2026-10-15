+++
id = "01M4VQXP7EE3BE5VQAPMHTX343"
title = "Q004 beyond-precision and Q006 exact-zero/trig info not raised"
type = "bug"
category = "todo"
priority = "medium"
points = 3
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:22Z"
updated = "2026-10-15T02:50:47Z"
scope = ["crates/cadetta-units/**", "tests/conformance/known-gaps.toml", "changelog.d/01M4VQXP7EE3BE5VQAPMHTX343.fixed.md", "crates/cadetta-cli/tests/quantity_zero_trig.rs"]

[[acceptance]]
text = "The listed fixtures pass."
bound = false
+++

found while working ~6BCVN48: quantities/q004, trig-exact-*, exact-zero-normalised, contract-q004-*, value-q004-beyond-precision fixtures expect Q004 error / Q006 info diagnostics that check never emits.
