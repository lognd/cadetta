+++
id = "01M4VB64CJDA2CZ4WM4MNV70G6"
title = "Import proposes datums from frames and placements from instances"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:47Z"
updated = "2026-10-15T08:58:38Z"
persona = "design engineer"
capability = "adopt an existing assembly without retyping its frames and component positions"
outcome_text = "positions are derived, not transcribed"
idempotency_key = "d15:import-geometry"
scope = ["crates/cadetta-adapter/src/import/**", "tests/conformance/adapters/**"]

[[links]]
kind = "blocked-by"
target = "01M4SWMCYW919R5JKSV2T1ZYBN"

[[links]]
kind = "blocked-by"
target = "01M4VTJ6W0X6YHWEEHT645FHJZ"

[[acceptance]]
text = "Given observed frames, when import proposes, then each becomes a datum built from origin with one offset and the fewest rotates, whole multiples of 90deg exact (adapters.md 7.2)"
bound = true

[[acceptance]]
text = "Given observed instances, when import proposes, then each becomes a flagged place anchored at the parent's origin with the read transform, and --accept writes explicit frame and instance locators (adapters.md 7.2)"
bound = true
+++
