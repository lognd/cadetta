+++
id = "01M4SPWNR85DB2J5CGXZK9NP4M"
title = "Three-valued comparison over affine forms"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T06:22:46Z"
persona = "design engineer"
capability = "compare toleranced quantities as holds, violates or undecided"
outcome_text = "an undecided check is never counted as holding"
labels = ["creates:crates/cadetta-units/src/compare.rs"]
scope = ["crates/cadetta-units/src/compare.rs", "crates/cadetta-units/src/lib.rs", "docs/spec/quantities.md"]

[[links]]
kind = "blocks"
target = "01M4SPWPT5XR3QCQJ7M1XP07MP"

[[links]]
kind = "blocks"
target = "01M4STCD6YD5DZFVD9RJDN02RY"

[[acceptance]]
text = "Given an affine form entirely above the threshold, when compared with `>`, then the verdict is holds (quantities.md 5.8)"
bound = false

[[acceptance]]
text = "Given a form that straddles the threshold, when compared, then the verdict is undecided (quantities.md 5.8)"
bound = false

[[acceptance]]
text = "Given a conservative evaluation that is undecided, when reported, then the verdict says so for C002 (quantities.md 5.4, 5.8)"
bound = false

[[acceptance]]
text = "Given an inexact operand whose difference is within 1e-9 relative, when compared, then the verdict is undecided with an inexactness note (quantities.md 5.8)"
bound = false

[[acceptance]]
text = "Given `==` on raw toleranced values, when compared, then C003 suggests the accessor form; `a.nominal == b.nominal` is exact (quantities.md 5.8, C003)"
bound = false
+++
