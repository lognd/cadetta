+++
id = "01M4STAPCEF5EPEYX0MZDSBA7J"
title = "ScrewJoint derived members"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T13:49:22Z"
persona = "design engineer"
capability = "read derived joint geometry"
idempotency_key = "replan13:ScrewJoint derived members"
labels = ["creates:std/src/fasteners/screwjoint.cdt"]
scope = ["std/src/fasteners/screwjoint.cdt", "crates/cadetta-sema/tests/**", "docs/spec/stdlib.md", "crates/cadetta-syntax/tests/std_fasteners_screwjoint.rs"]

[[links]]
kind = "blocks"
target = "01M4STAPFPQ9GV8XJB4XCK8EGP"

[[links]]
kind = "blocked-by"
target = "01M4SXRV8HCNXHVFX87BTFJC7S"

[[acceptance]]
text = "Given a ScrewJoint, when evaluated, then each derived member in stdlib.md 4 matches its formula (stdlib.md 3)"
bound = true

[[acceptance]]
text = "Given a hand-computed M5 case, when evaluated, then results match exactly (stdlib.md 3)"
bound = true

[[acceptance]]
text = "Given ScrewJoint, when its members are listed from another package, then the public members are exactly those in stdlib.md 3 and its other intermediate values are priv, still evaluated and shown by why and impact (stdlib.md intro; packages.md 3)"
bound = true

[[acceptance]]
text = "Given a dependent package that reads a priv ScrewJoint member, when checked, then E016 fires (packages.md 3)"
bound = true
+++
