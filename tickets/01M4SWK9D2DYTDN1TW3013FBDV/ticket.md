+++
id = "01M4SWK9D2DYTDN1TW3013FBDV"
title = "Canonical path grammar with indexes, [*] and placement paths"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T05:43:35Z"
updated = "2026-10-13T05:43:35Z"
persona = "design engineer"
capability = "address any node by path"
idempotency_key = "replan14:ir-paths"
labels = ["creates:crates/cadetta-ir/src/path.rs"]
scope = ["crates/cadetta-ir/src/path.rs"]

[[acceptance]]
text = "Given paths such as Bracket.holes[1].x, Rover/wheel[2].hub_bore and Rover/feet[i].place.0.x, when parsed, then they round-trip (ir.md 2)"
bound = false

[[acceptance]]
text = "Given [*], when it appears outside bindings, then it is rejected; in bindings it is accepted (ir.md 2)"
bound = false

[[acceptance]]
text = "Given datum paths Type.<datum>.offset.x and .rotate.x and generator members holes.pitch_x, when parsed, then they resolve to Members (ir.md 2)"
bound = false

[[acceptance]]
text = "Given a parameterized component, when a definition path is read, then the instance-only rule is enforced (ir.md 2; E011)"
bound = false
+++
