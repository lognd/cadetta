+++
id = "01M4WE56Z5R66XFD87JQ07FCB2"
title = "Permanent S001 gate over the std tree"
type = "task"
category = "todo"
priority = "low"
points = 3
reporter = "Claude"
created = "2026-10-14T05:28:57Z"
updated = "2026-10-16T00:38:16Z"
labels = ["creates:crates/cadetta-cli/tests/std_s001_gate.rs"]
scope = ["crates/cadetta-cli/tests/std_s001_gate.rs"]

[[acceptance]]
text = "Given an uncited ots value in std/src, when the gate runs, then it fails"
bound = false
+++

Follow-up from ~E751RZY: std findings are filtered by is_std in pipeline.rs, so no standing check proves every std ots value and table is cited. Add a test or gate running S001 over std/src.
