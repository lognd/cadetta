+++
id = "01M4ST8FAX8HDAXH8NBC8VKPXW"
title = "impact --swap: CAD files, attestations and commercial deltas"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:44Z"
updated = "2026-10-13T05:02:44Z"
persona = "design engineer"
capability = "see what a swap costs and what must be re-synced and re-reviewed"
idempotency_key = "replan13:impact --swap: CAD files, attestations and commercial deltas"
labels = ["creates:crates/cadetta-cli/src/cmd/impact_swap_report.rs"]
scope = ["crates/cadetta-cli/src/cmd/impact_swap_report.rs"]

[[acceptance]]
text = "Given bound paths whose emitted value changes, when printed, then CAD files to re-sync list the paths and unchanged files are listed as not stale (obligations.md 5.1 section 5)"
bound = false

[[acceptance]]
text = "Given attestations whose covered closure digests change, when printed, then each is listed with the count of covered nodes affected (obligations.md 5.1 section 6)"
bound = false

[[acceptance]]
text = "Given the swap, when commercial deltas are printed, then they are diffs of `sum?(self, unit_cost)` per currency, `max?(self, lead_time)` and `mass?(self)` at the assembly named by the PART prefix, plus `moq` per changed part (obligations.md 5.1 section 7)"
bound = false

[[acceptance]]
text = "Given parts lacking a value, when printed, then they are listed (obligations.md 5.1)"
bound = false

[[acceptance]]
text = "Given money, percentages and quantities, when printed, then money uses the currency minor unit and percentages one decimal half away from zero (obligations.md 5.1)"
bound = false
+++
