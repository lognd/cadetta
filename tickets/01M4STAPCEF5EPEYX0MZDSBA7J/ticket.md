+++
id = "01M4STAPCEF5EPEYX0MZDSBA7J"
title = "ScrewJoint derived members"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T05:05:21Z"
persona = "design engineer"
capability = "read derived joint geometry"
idempotency_key = "replan13:ScrewJoint derived members"
labels = ["creates:std/fasteners/screwjoint.cdt"]
scope = ["std/fasteners/screwjoint.cdt"]

[[links]]
kind = "blocks"
target = "01M4STAPFPQ9GV8XJB4XCK8EGP"

[[acceptance]]
text = "Given a ScrewJoint, when evaluated, then each derived member in stdlib.md 4 matches its formula (stdlib.md 3)"
bound = false

[[acceptance]]
text = "Given a hand-computed M5 case, when evaluated, then results match exactly (stdlib.md 3)"
bound = false
+++
