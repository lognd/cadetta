+++
id = "01M4STAP0CCQTR0MM7TAMWC9E8"
title = "std.materials (14 variants and Materials table)"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T07:36:02Z"
persona = "design engineer"
capability = "pick a material and read cited properties"
idempotency_key = "replan13:std.materials (14 variants and Materials table)"
labels = ["creates:std/src/materials.cdt"]
scope = ["std/src/materials.cdt", "crates/cadetta-syntax/tests/std_materials.rs", "docs/spec/stdlib.md"]

[[links]]
kind = "blocks"
target = "01M4STAPCEF5EPEYX0MZDSBA7J"

[[links]]
kind = "blocks"
target = "01M4STAPPJR0478N5B1E751RZY"

[[acceptance]]
text = "Given the 14 Material variants in stdlib.md 3, when loaded, then each has its properties and a citation (stdlib.md 2)"
bound = true

[[acceptance]]
text = "Given the Materials table, when looked up by key, then the matching row is returned (stdlib.md 2)"
bound = true
+++
