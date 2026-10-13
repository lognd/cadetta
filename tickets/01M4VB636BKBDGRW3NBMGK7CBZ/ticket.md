+++
id = "01M4VB636BKBDGRW3NBMGK7CBZ"
title = "Placement: place A at B, point offsets, elementwise arrays, unified op paths"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:46Z"
updated = "2026-10-13T22:36:07Z"
persona = "design engineer"
capability = "place parts with a verb that does not suggest contact and place arrays without index plumbing"
outcome_text = "placements read the way parts sit"
idempotency_key = "d15:place-at"
scope = ["crates/cadetta-syntax/src/**", "crates/cadetta-sema/src/lower.rs", "crates/cadetta-sema/src/point.rs", "crates/cadetta-geometry/src/**", "crates/cadetta-ir/src/path.rs", "crates/cadetta-ir/src/geometry.rs", "crates/cadetta-ir/src/schema.rs", "docs/schemas/cadetta.ir.1.json", "tests/golden/**", "tests/conformance/geometry/**", "tests/conformance/known-gaps.toml", "tests/conformance/stdlib/**", "crates/cadetta-syntax/tests/**"]

[[links]]
kind = "blocked-by"
target = "01M4VB630NZKXA6Q9D7253FATR"

[[acceptance]]
text = "Given place A at B, when evaluated, then datum A's frame equals B's with no flip; place a.b on c and a trailing for clause are E001 pointing at the new form (geometry.md 4; language.md 5a)"
bound = true

[[acceptance]]
text = "Given D.offset(p) with p typed on D as the first op, when evaluated, then it equals D.offset(x = p.x, y = p.y[, z]) with Members whose expression is the point; p on another datum, or the point form after another op, is G001 (geometry.md 4)"
bound = true

[[acceptance]]
text = "Given place feet.top at base.underside.offset(base.foot_sites), when evaluated, then instance i is placed at point i; mismatched counts, an anchor with no array operand, or an array anchor on an indexed target is G004 (geometry.md 4)"
bound = true

[[acceptance]]
text = "Given placement ops, when lowered, then Member paths are Assembly/part.place.offset.x, place.offset[1].x and place.rotate.z, array placements lower per instance with integer indexes, and the Op enum has no at variant (ir.md 2, 3)"
bound = true

[[acceptance]]
text = "Given a part placed twice or an array partly placed instance by instance, when checked, then G006 and G007 fire as geometry.md 4 states"
bound = false

[[acceptance]]
text = "Given the golden corpus and geometry fixtures, when run, then they use place at and pass"
bound = false
+++
