+++
id = "01M4VQXPDA4TEKE455FNYB5TXC"
title = "Geometry in check: evaluate datums, points, patterns and placements through cadetta-geometry and lower them to the IR"
type = "bug"
category = "in-progress"
priority = "high"
points = 8
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T23:00:22Z"
updated = "2026-10-13T23:46:50Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-geometry/src/**", "crates/cadetta-cli/src/pipeline.rs", "tests/conformance/geometry/**", "tests/conformance/known-gaps.toml", "tests/golden/**"]

[[acceptance]]
text = "Given a .cdt package with datums, points, patterns and placements, when cadetta check and cadetta ir run, then sema evaluates them by calling cadetta-geometry (no second implementation in sema) and the IR holds Datum, Point and Placement entities with their op Members and values (geometry.md 2-4, 6; ir.md 3)"
bound = false

[[acceptance]]
text = "Given p in D across placed parts, when evaluated in a check, then re-expression composes the placement tree as geometry.md 3 says, with G003 for an unplaced part"
bound = false

[[acceptance]]
text = "Given the geometry fixtures and golden cases that only needed evaluation, when the conformance corpus and golden tests run, then their known-gaps entries are removed and they pass"
bound = false
+++

found while working ~6BCVN48: geometry/* fixtures expect members like P.qx not present in the evaluated IR (14 fixtures).
