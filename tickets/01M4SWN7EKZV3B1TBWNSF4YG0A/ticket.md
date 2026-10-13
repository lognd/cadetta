+++
id = "01M4SWN7EKZV3B1TBWNSF4YG0A"
title = "why --stale"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:39Z"
updated = "2026-10-13T05:44:44Z"
persona = "design engineer"
capability = "see why something went stale"
idempotency_key = "replan14:ob-stale"
labels = ["creates:crates/cadetta-obligations/src/why_stale.rs"]
scope = ["crates/cadetta-obligations/src/why_stale.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST8F5G8E0FMC3KZJG8A95T"

[[acceptance]]
text = "Given a stale obligation, when why --stale runs, then it prints the nearest changed upstream node with old and new values (obligations.md 5; config-cli.md 3)"
bound = false
+++
