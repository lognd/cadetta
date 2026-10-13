+++
id = "01M4ST8F01H7SYVGYX5G9PCJYT"
title = "Roles, policy and release gating"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T08:39:26Z"
persona = "design engineer"
capability = "declare which sign-offs gate check and release"
idempotency_key = "replan13:Roles, policy and release gating"
labels = ["creates:crates/cadetta-obligations/src/roles.rs"]
scope = ["crates/cadetta-obligations/src/roles.rs"]

[[acceptance]]
text = 'Given `[obligations.roles.R] covers = ["fit:*", "check:*"]`, when matched, then selectors like fit:*, check:* and ots:* cover the right nodes (obligations.md 4)'
bound = true

[[acceptance]]
text = 'Given stale or open obligations with required_for = "check", when gating, then the result fails; release roles fail only under `--release` (obligations.md 4)'
bound = true

[[acceptance]]
text = "Given a `require`, when waived without an attestation, then it is refused: attestation is the only way (obligations.md 4)"
bound = true

[[acceptance]]
text = "Given roles, when configured, then members and distinct_from_author are enforced (config-cli.md 2; obligations.md 4)"
bound = true
+++
