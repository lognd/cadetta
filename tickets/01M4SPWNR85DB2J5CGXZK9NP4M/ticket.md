+++
id = "01M4SPWNR85DB2J5CGXZK9NP4M"
title = "Three-valued comparison of toleranced quantities"
type = "story"
category = "todo"
priority = "medium"
points = 2
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T04:03:58Z"
persona = "design engineer"
capability = "compare toleranced quantities as holds, violates or undecided"
outcome_text = "an undecided check is never counted as holding"
labels = ["creates:crates/cadetta-units/src/compare.rs"]
scope = ["crates/cadetta-units/src/compare.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWPT5XR3QCQJ7M1XP07MP"

[[acceptance]]
text = "Given intervals that overlap the threshold, when compared, then the verdict is undecided"
bound = false

[[acceptance]]
text = "Given an interval entirely above the threshold, when compared with >, then the verdict is holds"
bound = false
+++
