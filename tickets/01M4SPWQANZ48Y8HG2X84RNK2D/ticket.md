+++
id = "01M4SPWQANZ48Y8HG2X84RNK2D"
title = "Generated JSON Schema committed to docs/schemas"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T08:44:04Z"
persona = "cadetta contributor"
capability = "generate the schema from the Rust types and commit it"
outcome_text = "the IR is a public contract with a compatibility check"
labels = ["creates:docs/schemas/cadetta.ir.1.json", "creates:crates/cadetta-ir/src/schema.rs"]
scope = ["docs/schemas/cadetta.ir.1.json", "crates/cadetta-ir/src/schema.rs", "crates/cadetta-ir/Cargo.toml", "crates/cadetta-ir/src/lib.rs", "docs/spec/ir.md"]

[[links]]
kind = "blocks"
target = "01M4SPWS56AF9GE891GAM5QSWX"

[[links]]
kind = "blocks"
target = "01M4SPWT6N1HY3MHN544GZJYS9"

[[acceptance]]
text = "Given the Rust types, when the schema is generated, then it matches the committed file (CI check)"
bound = false

[[acceptance]]
text = "Given an optional field added, when diffed, then it is a minor change"
bound = false

[[acceptance]]
text = "Given a schema change, when classified, then adding optional fields is minor and anything else bumps cadetta.ir/N (ir.md 5)"
bound = false

[[acceptance]]
text = "Given draft 1.4, when schemas are generated, then docs/schemas also holds diag.json and the entities of ir.md 3 including Dim and Datum (ir.md 5; lints.md 1.1)"
bound = false
+++
