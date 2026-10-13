+++
id = "01M4SPWPAJ4ZGEGGVF63ZY806Z"
title = "Canonical formatter"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWG5A0FJNPKRPDDCJQZN8"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T04:03:52Z"
persona = "design engineer"
capability = "run a formatter that produces one canonical layout"
outcome_text = "layout never causes review noise or IR changes"
labels = ["creates:crates/cadetta-syntax/src/format.rs"]
scope = ["crates/cadetta-syntax/src/format.rs"]

[[acceptance]]
text = "Given any valid file, when formatted twice, then output is identical (idempotent)"
bound = false

[[acceptance]]
text = "Given `=` signs in a block, when formatted, then they are aligned with two-space indent and authored units preserved"
bound = false

[[acceptance]]
text = "Given formatted source, when lowered, then the IR digest is unchanged"
bound = false
+++
