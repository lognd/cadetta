+++
id = "01M4SWG6T58Q489WASV5DYGF7F"
title = "distance(), aligned() and G004"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:54Z"
updated = "2026-10-13T23:22:59Z"
persona = "design engineer"
capability = "prove that holes line up"
idempotency_key = "replan14:geo-aligned"
labels = ["creates:crates/cadetta-geometry/src/aligned.rs"]
scope = ["crates/cadetta-geometry/src/aligned.rs", "crates/cadetta-geometry/src/lib.rs", "tests/conformance/geometry/**", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocked-by"
target = "01M4SPWPT5XR3QCQJ7M1XP07MP"

[[links]]
kind = "blocked-by"
target = "01M4SWFXADMC8C5Y451JRVBYZM"

[[links]]
kind = "blocked-by"
target = "01M4SWG6N6DT76DRYYJHFB4656"

[[acceptance]]
text = "Given aligned(a, b, allowance) with equal counts, when evaluated, then every pair a[i], b[i] is measured in b's xy plane after conversion to b's datum, ignoring z (geometry.md 5)"
bound = true

[[acceptance]]
text = "Given patterns of different counts, when evaluated, then G004 is raised (geometry.md 5, 7)"
bound = true

[[acceptance]]
text = "Given the section 1 example with +-0.05mm tolerances, when checked, then the distance is in [0, 0.1414mm] and holds against 0.25mm; with +-0.2mm and +-0.1mm it is C002, not C001 (geometry.md 1)"
bound = true

[[acceptance]]
text = "Given distance(p, q), when both are in one datum, then the axial component is included (geometry.md 3)"
bound = false
+++
