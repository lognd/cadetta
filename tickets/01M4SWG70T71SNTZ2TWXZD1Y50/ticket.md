+++
id = "01M4SWG70T71SNTZ2TWXZD1Y50"
title = "Lint G005 bolted-float-unlocated"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:54Z"
updated = "2026-10-13T05:41:54Z"
persona = "design engineer"
capability = "see when a tight check ignores bolt float"
idempotency_key = "replan14:geo-g005"
labels = ["creates:crates/cadetta-lint/src/rules/g005.rs", "creates:tests/lints/G005/**"]
scope = ["crates/cadetta-lint/src/rules/g005.rs", "tests/lints/G005/**"]

[[acceptance]]
text = "Given a check tighter than a joint's float that reads through a placement carrying it, when linted, then G005 warning fires (geometry.md 5.2, 7)"
bound = false

[[acceptance]]
text = "Given locating = true or a looser check, when linted, then G005 does not fire (geometry.md 5.2)"
bound = false
+++
