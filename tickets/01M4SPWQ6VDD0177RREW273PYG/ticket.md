+++
id = "01M4SPWQ6VDD0177RREW273PYG"
title = "JSON serialization with string rationals"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T07:50:00Z"
persona = "cadetta contributor"
capability = "serialize the IR as cadetta.ir/1 JSON"
outcome_text = "adapters and tools exchange a stable format"
labels = ["creates:crates/cadetta-ir/src/json.rs"]
scope = ["crates/cadetta-ir/src/json.rs", "crates/cadetta-ir/src/lib.rs", "docs/spec/ir.md", "changelog.d/01M4SPWQ6VDD0177RREW273PYG.added.md"]

[[links]]
kind = "blocks"
target = "01M4SPWQANZ48Y8HG2X84RNK2D"

[[links]]
kind = "blocks"
target = "01M4SPWQH44JWFXZDNTGEBMMNX"

[[acceptance]]
text = 'Given a rational 127/5000, when serialized, then it is the string "127/5000" with den > 0 and reduced'
bound = true

[[acceptance]]
text = "Given any IR, when serialized, then no magnitude is a JSON float"
bound = true

[[acceptance]]
text = "Given an Inexact value, when serialized, then it is the only float and names its root operation path (ir.md 3)"
bound = true

[[acceptance]]
text = 'Given Exact, when serialized, then it is `{ r: "num/den", pi: int }` (ir.md 3)'
bound = true
+++
