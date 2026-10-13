+++
id = "01M4SPWQANZ48Y8HG2X84RNK2D"
title = "Generated JSON Schema committed to docs/schemas"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T04:03:53Z"
persona = "cadetta contributor"
capability = "generate the schema from the Rust types and commit it"
outcome_text = "the IR is a public contract with a compatibility check"
labels = ["creates:docs/schemas/cadetta.ir.1.json", "creates:crates/cadetta-ir/src/schema.rs"]
scope = ["docs/schemas/cadetta.ir.1.json", "crates/cadetta-ir/src/schema.rs"]

[[acceptance]]
text = "Given the Rust types, when the schema is generated, then it matches the committed file (CI check)"
bound = false

[[acceptance]]
text = "Given an optional field added, when diffed, then it is a minor change"
bound = false
+++
