+++
id = "01M4SPWPKGXD3TJ78NPJH8Q9VZ"
title = "Kind and dimension checking"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T07:16:32Z"
persona = "design engineer"
capability = "have expressions checked for units and kinds"
outcome_text = "Q001, Q002, Q003 and Q005 stop wrong models"
labels = ["creates:crates/cadetta-sema/src/kindcheck.rs"]
scope = ["crates/cadetta-sema/src/kindcheck.rs", "crates/cadetta-sema/src/lib.rs", "docs/spec/quantities.md"]

[[links]]
kind = "blocks"
target = "01M4SPWPPPSEEFAHYTY5H3YD54"

[[acceptance]]
text = "Given a bare `5` assigned to a Length, when checked, then Q001 unitless-quantity is reported (quantities.md 2)"
bound = true

[[acceptance]]
text = "Given `1mm + 1kg`, when checked, then Q002 dimension-mismatch is reported"
bound = true

[[acceptance]]
text = "Given `x: Energy = some_torque`, when checked, then Q003 is reported and `some_torque as Energy` is accepted (quantities.md 4.1)"
bound = false

[[acceptance]]
text = "Given the angle cases of quantities.md 4.3, when checked, then Q008 fires exactly on the stated rows"
bound = false

[[acceptance]]
text = "Given a non-integer Count or a root of a non-divisible dimension, when checked, then Q009 and Q011 are reported (quantities.md 4.2, 4.1)"
bound = false

[[acceptance]]
text = "Given bare numbers as factors, exponents, dimensionless operands and Count/Ratio values, when checked, then no Q001 is reported (quantities.md 2)"
bound = false
+++
