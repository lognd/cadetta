+++
id = "01M4ST8ERHSJXGDXNH75ZCX6H7"
title = "Attestations and staleness from closure digests"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T07:56:32Z"
persona = "design engineer"
capability = "learn exactly which sign-offs went stale and why"
idempotency_key = "replan13:Attestations and staleness from closure digests"
labels = ["creates:crates/cadetta-obligations/src/staleness.rs"]
scope = ["crates/cadetta-obligations/src/staleness.rs", "crates/cadetta-obligations/src/lib.rs", "crates/cadetta-ir/src/digest.rs", "docs/spec/obligations.md"]

[[links]]
kind = "blocks"
target = "01M4ST8ET5NPMBSVXFVD4WN8XP"

[[links]]
kind = "blocks"
target = "01M4ST8F01H7SYVGYX5G9PCJYT"

[[links]]
kind = "blocks"
target = "01M4ST8F1W8T0BKARAMG0D49J2"

[[links]]
kind = "blocks"
target = "01M4ST8F3SDF95MVP07VXXNX0X"

[[links]]
kind = "blocks"
target = "01M4ST8F5G8E0FMC3KZJG8A95T"

[[links]]
kind = "blocks"
target = "01M4ST8F7B9VYMFSCJD8MVBJNS"

[[links]]
kind = "blocks"
target = "01M4STCCZ58C875BTMQ9E54BMT"

[[links]]
kind = "blocks"
target = "01M4STCF8FJ9HGB7EZ37MC24EF"

[[acceptance]]
text = "Given an attestation on a check, when anything the check reads changes however far upstream, then it becomes stale (obligations.md 2)"
bound = true

[[acceptance]]
text = "Given nodes unrelated to the attested set, when they change, then the attestation stays discharged (obligations.md 2)"
bound = true

[[acceptance]]
text = "Given staleness, when computed, then it is derived by comparing recorded closure digests with the current graph and never stored (obligations.md 2)"
bound = true

[[acceptance]]
text = "Given a swap of an interface-typed slot, when computed, then attestations covering instance paths of the slot become stale and attestations on other parts do not (ir.md 4)"
bound = true

[[acceptance]]
text = "Given a require evaluating holds with an attestation configured, when classified, then it is discharged only with a current attestation (obligations.md 2, 4)"
bound = true

[[acceptance]]
text = "Given an as-built obligation, when classified, then it is stale if the design changed after the inspection (obligations.md 2, OQ-13)"
bound = false

[[acceptance]]
text = "Given an attestation, when judged, then verdict and margin are recorded and dependency own digests are used (obligations.md 2)"
bound = false
+++
