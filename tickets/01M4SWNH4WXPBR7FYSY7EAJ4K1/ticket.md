+++
id = "01M4SWNH4WXPBR7FYSY7EAJ4K1"
title = "Fixed as-of date for expiry"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:49Z"
updated = "2026-10-13T06:04:57Z"
persona = "design engineer"
capability = "reproduce a historic result"
idempotency_key = "replan14:ob-asof"
labels = ["creates:crates/cadetta-obligations/src/as_of.rs"]
scope = ["crates/cadetta-obligations/src/as_of.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWSTRG1FFAJ0XM89ETBBN"

[[acceptance]]
text = "Given --as-of or [package] as_of, when expiry is judged, then that date is used and recorded in the JSON report header (config-cli.md 4)"
bound = false
+++
