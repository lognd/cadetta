+++
id = "01M4ST8EPKBWRWHF51Y5NV3VK0"
title = "cadetta.lock format and persistence"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T05:05:17Z"
persona = "design engineer"
capability = "commit attestations and digests so the graph is rebuildable at any commit"
idempotency_key = "replan13:cadetta.lock format and persistence"
labels = ["creates:crates/cadetta-obligations/src/lock.rs"]
scope = ["crates/cadetta-obligations/src/lock.rs"]

[[links]]
kind = "blocks"
target = "01M4ST8ERHSJXGDXNH75ZCX6H7"

[[acceptance]]
text = "Given attestations, cited file digests, solver result digests and last-sync digests per adapter and file, when written, then entries are sorted and line-oriented (obligations.md 3)"
bound = false

[[acceptance]]
text = "Given the same state twice, when written, then the bytes are identical and merges of independent attestations do not conflict (obligations.md 3)"
bound = false

[[acceptance]]
text = "Given source plus the lock file at any commit, when loaded, then the obligation graph is rebuilt; nothing else writes state (obligations.md 2, 3)"
bound = false

[[acceptance]]
text = "Given an attestation, when stored, then it holds who, role, when, reason and the digest set (obligations.md 3)"
bound = false
+++
