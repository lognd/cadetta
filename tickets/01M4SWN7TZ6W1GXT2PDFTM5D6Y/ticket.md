+++
id = "01M4SWN7TZ6W1GXT2PDFTM5D6Y"
title = "required_for and sync never discharged by equations"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:39Z"
updated = "2026-10-13T05:44:44Z"
persona = "design engineer"
capability = "gate releases on real sync"
idempotency_key = "replan14:ob-kinds"
labels = ["creates:crates/cadetta-obligations/src/kinds.rs"]
scope = ["crates/cadetta-obligations/src/kinds.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST8EMQTXAGZAWGE2M5A4BR"

[[acceptance]]
text = "Given [obligations.kinds.sync] required_for, when set, then it gates sync and solver obligations; sw-equations emission alone never discharges a sync obligation (obligations.md 2.3; config-cli.md 2)"
bound = false
+++
