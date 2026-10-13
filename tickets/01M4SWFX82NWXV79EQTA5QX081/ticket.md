+++
id = "01M4SWFX82NWXV79EQTA5QX081"
title = "Datum-typed Point2/Point3, Vec2/Vec3 arithmetic and G001"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 8
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:44Z"
updated = "2026-10-13T09:29:04Z"
persona = "design engineer"
capability = "keep positions in their datum"
idempotency_key = "replan14:geo-points"
labels = ["creates:crates/cadetta-geometry/src/point.rs"]
scope = ["crates/cadetta-geometry/src/point.rs", "crates/cadetta-geometry/src/lib.rs", "docs/spec/geometry.md"]

[[links]]
kind = "blocked-by"
target = "01M4ST7AFZGCN52434BQJRT5MM"

[[links]]
kind = "blocked-by"
target = "01M4SWFX374CNSPDDD289Z9BCS"

[[acceptance]]
text = "Given 'Point2 on top' and a literal Point2(5mm, 0mm) on top, when typed, then both carry datum top (geometry.md 3)"
bound = true

[[acceptance]]
text = "Given Point - Point, Point + Vec, Vec * s and array + vector, when evaluated, then results are in the same datum (geometry.md 3)"
bound = true

[[acceptance]]
text = "Given arithmetic across datums or array + point, when evaluated, then G001 is raised and nothing converts silently (geometry.md 3, 7)"
bound = true

[[acceptance]]
text = "Given each hand-listed coordinate, when lowered, then it is a Member with a canonical path such as Bracket.holes[1].x (geometry.md 3)"
bound = true
+++
