+++
id = "01M4SWFX374CNSPDDD289Z9BCS"
title = "Datum declarations and Datum type"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:44Z"
updated = "2026-10-13T07:11:59Z"
persona = "design engineer"
capability = "declare datums on a component"
idempotency_key = "replan14:geo-datum"
labels = ["creates:crates/cadetta-geometry/src/datum.rs", "creates:crates/cadetta-geometry/Cargo.toml", "creates:crates/cadetta-geometry/src/lib.rs"]
scope = ["crates/cadetta-geometry/src/datum.rs", "crates/cadetta-geometry/Cargo.toml", "crates/cadetta-geometry/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWPCW1C0QC67GZG99EHTZ"

[[links]]
kind = "blocked-by"
target = "01M4SWFX0C2BXQGR8WQ2VN1G88"

[[acceptance]]
text = "Given datum site = top.offset(x = 40mm), when evaluated, then it is a frame built from a datum of the same component and ops apply left to right (geometry.md 2)"
bound = false

[[acceptance]]
text = "Given an offset or rotation argument, when lowered, then it is a Member with a canonical path such as Rail.site.offset.x (geometry.md 2; ir.md 2)"
bound = false

[[acceptance]]
text = "Given flip(x), when evaluated, then it equals rotate(x = 180deg) and reverses the normal (geometry.md 2)"
bound = false

[[acceptance]]
text = "Given a function parameter of type Datum, when called with a datum name, then it type-checks (geometry.md 2)"
bound = false
+++
