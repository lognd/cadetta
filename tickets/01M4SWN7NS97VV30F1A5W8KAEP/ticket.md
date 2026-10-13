+++
id = "01M4SWN7NS97VV30F1A5W8KAEP"
title = "Semantic versus presentational digests"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:39Z"
updated = "2026-10-13T05:44:39Z"
persona = "design engineer"
capability = "avoid false staleness"
idempotency_key = "replan14:ob-attrs"
labels = ["creates:crates/cadetta-obligations/src/digest_attrs.rs"]
scope = ["crates/cadetta-obligations/src/digest_attrs.rs"]

[[acceptance]]
text = "Given a change to display, emit, allow or lifecycle date text, when graphed, then no obligation goes stale; a change to process, role, rule roles or default does (ir.md 4)"
bound = false
+++
