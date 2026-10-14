+++
id = "01M4V436F7JRA8P832RH0EKNH2"
title = "Geometry diagnostics G001-G008 emitted by check"
type = "bug"
category = "done"
outcome = "done"
priority = "high"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T17:13:51Z"
updated = "2026-10-14T01:24:52Z"
scope = ["crates/cadetta-lint/**", "crates/cadetta-sema/**", "crates/cadetta-cli/**", "tests/conformance/**", "docs/**"]

[[links]]
kind = "blocked-by"
target = "01M4VQXPDA4TEKE455FNYB5TXC"

[[acceptance]]
text = "Given the geometry fixtures for G001-G008, when cadetta check runs, then each diagnostic is emitted with its catalog code, path and location (geometry.md 7, lints.md G) and its known-gaps entries are removed"
bound = true
+++

found while working ~ZFM3633. The geometry conformance fixtures g001-* .. g007-* (tests/conformance/geometry) expect G001..G007 placement and datum-mismatch diagnostics; the binary emits none, and a.face-style datum references of placed parts report E002.
