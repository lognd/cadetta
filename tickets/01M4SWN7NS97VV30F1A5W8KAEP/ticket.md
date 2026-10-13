+++
id = "01M4SWN7NS97VV30F1A5W8KAEP"
title = "Semantic versus presentational digests"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:39Z"
updated = "2026-10-13T07:48:51Z"
persona = "design engineer"
capability = "avoid false staleness"
idempotency_key = "replan14:ob-attrs"
labels = ["creates:crates/cadetta-obligations/src/digest_attrs.rs"]
scope = ["crates/cadetta-obligations/src/digest_attrs.rs", "crates/cadetta-obligations/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQED1EN19NWQJE5NT0J4"

[[acceptance]]
text = "Given a change to display, emit, allow or lifecycle date text, when graphed, then no obligation goes stale; a change to process, role, rule roles or default does (ir.md 4)"
bound = true
+++
