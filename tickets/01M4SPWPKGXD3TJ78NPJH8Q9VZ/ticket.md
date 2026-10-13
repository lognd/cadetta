+++
id = "01M4SPWPKGXD3TJ78NPJH8Q9VZ"
title = "Kind and dimension checking"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T04:03:58Z"
persona = "design engineer"
capability = "have expressions checked for units and kinds"
outcome_text = "Q001, Q002, Q003 and Q005 stop wrong models"
labels = ["creates:crates/cadetta-sema/src/kindcheck.rs"]
scope = ["crates/cadetta-sema/src/kindcheck.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWPPPSEEFAHYTY5H3YD54"

[[acceptance]]
text = "Given a bare `5` assigned to a Length, when checked, then Q001 unitless-quantity is reported"
bound = false

[[acceptance]]
text = "Given `1mm + 1kg`, when checked, then Q002 dimension-mismatch is reported"
bound = false

[[acceptance]]
text = "Given `x: Energy = some_torque`, when checked, then Q003 is reported and `some_torque as Energy` is accepted"
bound = false

[[acceptance]]
text = "Given an ots parameter assigned from an expression, when checked, then D002 is signalled"
bound = false
+++
