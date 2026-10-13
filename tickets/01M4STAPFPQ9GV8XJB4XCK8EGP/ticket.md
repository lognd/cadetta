+++
id = "01M4STAPFPQ9GV8XJB4XCK8EGP"
title = "ScrewJoint arguments and checks"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T23:18:49Z"
persona = "design engineer"
capability = "declare a screw joint and have it checked"
idempotency_key = "replan13:ScrewJoint arguments and checks"
labels = ["creates:std/src/fasteners/screwjoint_checks.cdt"]
scope = ["std/src/fasteners/screwjoint_checks.cdt"]

[[links]]
kind = "blocks"
target = "01M4STAPPJR0478N5B1E751RZY"

[[acceptance]]
text = "Given head, clearance, counterbore, policy, tap Cut/Form/Vendor, depth_available, host and screw_cost, when passed, then each is checked as in stdlib.md 4"
bound = false

[[acceptance]]
text = "Given tap=Vendor, when evaluated, then host and screw_cost apply (stdlib.md 3)"
bound = false

[[acceptance]]
text = "Given an omitted `into`, when evaluated, then its default applies (stdlib.md 3)"
bound = false

[[acceptance]]
text = "Given a violated check, when evaluated, then a diagnostic names the check (stdlib.md 3)"
bound = false

[[acceptance]]
text = "Given a ScrewJoint, when evaluated, then its checks are named engaged (engagement >= min_engagement), not_bottomed (thread_depth > screw_length - grip), grip_positive, depth_ok (only with depth_available) and aligned (only with at and onto), and verdicts carry paths such as Mount/screws.engaged (stdlib.md 3)"
bound = false

[[acceptance]]
text = "Given a joint without depth_available or without at and onto, when evaluated, then no depth_ok or aligned check exists (stdlib.md 3)"
bound = false
+++
