+++
id = "01M4STAPK7FN8DM11MV7GWXE3Z"
title = "std.retaining (Din471, Din472)"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T07:38:14Z"
persona = "design engineer"
capability = "use standard retaining rings"
idempotency_key = "replan13:std.retaining (Din471, Din472)"
labels = ["creates:std/src/retaining.cdt"]
scope = ["std/src/retaining.cdt", "crates/cadetta-syntax/tests/std_retaining.rs", "docs/spec/stdlib.md"]

[[links]]
kind = "blocks"
target = "01M4STAPPJR0478N5B1E751RZY"

[[acceptance]]
text = "Given Din471 and Din472 tables, when loaded, then every value is as published and cited (stdlib.md 2)"
bound = false

[[acceptance]]
text = "Given a groove and ring, when checked, then fit rules in stdlib.md 5 apply (stdlib.md 2)"
bound = false
+++
