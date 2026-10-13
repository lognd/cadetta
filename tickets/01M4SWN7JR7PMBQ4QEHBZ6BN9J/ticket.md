+++
id = "01M4SWN7JR7PMBQ4QEHBZ6BN9J"
title = "swap verb"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:39Z"
updated = "2026-10-13T14:18:34Z"
persona = "design engineer"
capability = "swap a part safely"
idempotency_key = "replan14:ob-swap"
labels = ["creates:crates/cadetta-obligations/src/swap.rs"]
scope = ["crates/cadetta-obligations/src/swap.rs", "crates/cadetta-cli/src/cmd/swap.rs", "crates/cadetta-cli/src/cmd/ack.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST8EPKBWRWHF51Y5NV3VK0"

[[links]]
kind = "blocked-by"
target = "01M4ST8F91FHYFC01TN6018BAM"

[[acceptance]]
text = "Given swap SLOT ALT --reason, when the preview is not blocked, then source is edited and a swap lock record is written; blocked writes nothing and exits 1 (obligations.md 3, 5.1; config-cli.md 3)"
bound = false
+++
