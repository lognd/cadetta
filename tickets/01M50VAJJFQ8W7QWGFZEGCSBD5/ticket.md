+++
id = "01M50VAJJFQ8W7QWGFZEGCSBD5"
title = "Fix red conformance: read-never-raises-q001 diagnostics_exclude unobserved"
type = "bug"
category = "in-progress"
priority = "high"
points = 1
reporter = "lognd"
created = "2026-10-15T22:36:02Z"
updated = "2026-10-15T23:36:30Z"
scope = ["crates/cadetta-cli/tests/conformance/verify.rs"]

[[acceptance]]
text = "the conformance verifier observes diagnostics_exclude: none of the listed codes may be produced"
bound = true
+++

Split from ~QQJNWHY; blocked on the MGXDNYP lease over crates/cadetta-cli/tests/**.
