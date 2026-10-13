+++
id = "01M4ST8ET5NPMBSVXFVD4WN8XP"
title = "Citation digests and source acknowledgement"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T08:33:56Z"
persona = "design engineer"
capability = "know when a datasheet changed under a value"
idempotency_key = "replan13:Citation digests and source acknowledgement"
labels = ["creates:crates/cadetta-obligations/src/citation.rs"]
scope = ["crates/cadetta-obligations/src/citation.rs", "crates/cadetta-obligations/src/lib.rs", "docs/spec/obligations.md"]

[[acceptance]]
text = "Given a local-file citation, when acknowledged with `--role source`, then its digest is recorded in the lock (obligations.md 2, 3)"
bound = true

[[acceptance]]
text = "Given the cited file digest or the citation text changing, when checked, then the S004 citation-changed fact is recorded and the obligation is stale (obligations.md 2, lints.md S)"
bound = false

[[acceptance]]
text = 'Given a literal citation `@"ASTM A276"`, when checked, then no digest or reachability check applies (language.md 3.9)'
bound = false
+++
