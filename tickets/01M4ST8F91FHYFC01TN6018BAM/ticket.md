+++
id = "01M4ST8F91FHYFC01TN6018BAM"
title = "impact --swap: verdict, changed values, flips and suggested edit"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T13:34:22Z"
persona = "design engineer"
capability = "preview a part swap before editing"
idempotency_key = "replan13:impact --swap: verdict, changed values, flips and suggested edit"
labels = ["creates:crates/cadetta-cli/src/cmd/impact_swap.rs"]
scope = ["crates/cadetta-cli/src/cmd/impact_swap.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs"]

[[links]]
kind = "blocks"
target = "01M4ST8FAX8HDAXH8NBC8VKPXW"

[[links]]
kind = "blocks"
target = "01M4ST8FEBCF9PC9SR0ZKTYT70"

[[acceptance]]
text = "Given `impact --swap PART NEW`, when run, then it is the variant evaluation of PART=NEW diffed against the primary evaluation (obligations.md 5.1)"
bound = true

[[acceptance]]
text = "Given the outcome, when printed, then section 1 verdict is ok, ok with warnings or blocked, and a violated require cannot be waived (obligations.md 5.1)"
bound = true

[[acceptance]]
text = "Given changed values, when printed, then direct (interface members) come before derived, with old and new values, a `let` included (obligations.md 5.1)"
bound = true

[[acceptance]]
text = "Given flipped checks, fits, requires and bounds, when printed, then they are sorted by canonical path (obligations.md 5.1)"
bound = true

[[acceptance]]
text = 'Given a NEW not in the alt list, when printed, then the output says "not yet approved" and NEW must still implement the interface (obligations.md 5.1)'
bound = true

[[acceptance]]
text = "Given the suggested edit, when printed, then it is the new `part` line with NEW removed from alt and the old primary appended (obligations.md 5.1)"
bound = true
+++
