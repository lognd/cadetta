+++
id = "01M4W5HM6SAY12P1J3R3G80Q7F"
title = "Per-instance checks for parameterized components (engaged, not_bottomed)"
type = "task"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-14T02:58:24Z"
updated = "2026-10-14T03:35:14Z"
scope = ["crates/cadetta-sema/**", "tests/conformance/**", "std/**", "docs/spec/language.md"]

[[acceptance]]
text = "Given a ScrewJoint instance, when cadetta check runs, then its named checks (Box/j.engaged ...) report verdicts"
bound = true
+++

found while working ~646D1T7: checks.rs skips parameterized components; instance members now have values, so checks can run per instance.
