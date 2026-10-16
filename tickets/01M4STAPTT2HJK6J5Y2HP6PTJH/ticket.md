+++
id = "01M4STAPTT2HJK6J5Y2HP6PTJH"
title = "std 0.2.0 inch fasteners and inserts"
type = "story"
category = "done"
outcome = "done"
priority = "low"
points = 8
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:57Z"
updated = "2026-10-16T01:17:13Z"
persona = "design engineer"
capability = "use inch fasteners and inserts"
idempotency_key = "replan13:std 0.2.0 inch fasteners and inserts"
labels = ["creates:std/fasteners/inch.cdt"]
scope = ["std/fasteners/inch.cdt", "std/src/fasteners/inch.cdt", "std/src/fasteners.cdt", "std/src/lib.cdt", "std/cadetta.toml", "docs/spec/stdlib.md", "crates/cadetta-syntax/tests/std_fasteners_inch.rs"]

[[acceptance]]
text = "Given the inch fastener and insert scope in stdlib.md, when added, then each value is cited and the 0.2.0 version is set (stdlib.md 1)"
bound = true
+++
