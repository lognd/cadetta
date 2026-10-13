+++
id = "01M4SWK99ZRAYB4EQ355S4E2HA"
title = "IR entities Interface, Table, Waiver, Process, Part, Fit and Solver"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T05:43:35Z"
updated = "2026-10-13T08:28:38Z"
persona = "design engineer"
capability = "see every design entity in the IR"
idempotency_key = "replan14:ir-entities"
labels = ["creates:crates/cadetta-ir/src/entities.rs"]
scope = ["crates/cadetta-ir/src/entities.rs", "docs/spec/ir.md", "crates/cadetta-ir/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQ3VSX2CA3TZY4KQ2DZW"

[[links]]
kind = "blocked-by"
target = "01M4ST7ADMK0S7SRW6ETCS8ETC"

[[links]]
kind = "blocked-by"
target = "01M4ST7APCCWC8KASSNHK0FW70"

[[links]]
kind = "blocked-by"
target = "01M4ST8E8AQ98D0DFQ0DT50R91"

[[acceptance]]
text = "Given interfaces with bounds and datums, tables, waivers with targets_closure, and processes, when lowered, then each has its ir.md 3 entity (ir.md 3)"
bound = true

[[acceptance]]
text = "Given a Waiver with for_alt, when serialized, then it is keyed SLOT=ALT (ir.md 3)"
bound = true

[[acceptance]]
text = "Given a Check, when lowered, then verdicts hold primary and per_alternate keyed SLOT=ALT with conservative and waived_for (ir.md 3)"
bound = true
+++
