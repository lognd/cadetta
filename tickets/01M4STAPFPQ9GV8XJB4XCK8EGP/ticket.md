+++
id = "01M4STAPFPQ9GV8XJB4XCK8EGP"
title = "ScrewJoint arguments and checks"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T05:03:56Z"
persona = "design engineer"
capability = "declare a screw joint and have it checked"
idempotency_key = "replan13:ScrewJoint arguments and checks"
labels = ["creates:std/fasteners/screwjoint_checks.cdt"]
scope = ["std/fasteners/screwjoint_checks.cdt"]

[[acceptance]]
text = "Given head, clearance, counterbore, policy, tap Cut/Form/Vendor, depth_available, host and screw_cost, when passed, then each is checked as in stdlib.md 4"
bound = false

[[acceptance]]
text = "Given tap=Vendor, when evaluated, then host and screw_cost apply (stdlib.md 4)"
bound = false

[[acceptance]]
text = "Given an omitted `into`, when evaluated, then its default applies (stdlib.md 4)"
bound = false

[[acceptance]]
text = "Given a violated check, when evaluated, then a diagnostic names the check (stdlib.md 4)"
bound = false
+++
