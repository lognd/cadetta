+++
id = "01M4ZB40N2WD0VZYWZ7SNYB0FT"
title = "conformance emitters/sw-equations/naming-component-field fails: fixture uses dotted part path and out-of-range instance"
type = "bug"
category = "done"
outcome = "done"
priority = "high"
points = 1
reporter = "lognd"
created = "2026-10-15T08:33:35Z"
updated = "2026-10-15T08:37:38Z"
scope = ["crates/cadetta-cli/src/cmd/emit.rs", "tests/conformance/emitters/sw-equations/naming-component-field.*/**", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = 'sw-equations/naming-component-field emits "D1@Bore@Wheel.SLDPRT" = 10mm in both halves and adapters/star-mangle-drops-segment still passes'
bound = true
+++

Not a regression from naming_path: the gap entry was dropped by ~9WDH9XE but the fixture never passed. It binds Rover.wheel[2] (dot form, not the canonical Rover/wheel[2]) on part wheel[2], whose instances are 0..1.
