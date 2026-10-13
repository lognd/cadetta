+++
id = "01M4ST8EEGK712KZT44DEDBSEG"
title = "Viability, undecided results and alternate waivers"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4ST8E5XX0TX0MBND07JQME1"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T08:57:25Z"
persona = "design engineer"
capability = "know which alternates are safe to swap in"
idempotency_key = "replan13:Viability, undecided results and alternate waivers"
labels = ["creates:crates/cadetta-variants/src/viability.rs"]
scope = ["crates/cadetta-variants/src/viability.rs"]

[[links]]
kind = "blocks"
target = "01M4STCCK2B2XG83N4BRS5GR7B"

[[acceptance]]
text = "Given a variant where no check, fit, bound or require violates, when evaluated, then the alternate is viable (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given undecided results under a variant, when evaluated, then the alternate stays viable with the I003 fact (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given a violating variant, when evaluated, then the alternate stays listed as not viable (I002) naming the failure and is reported as `C001 for alt NAME` (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given `waive C002 on Rover.load_ok for alt Colson2Series`, when evaluated, then it applies to that variant only (language.md 3.2, lints.md 2)"
bound = false

[[acceptance]]
text = "Given a violated `require` under a variant, when waived, then the alternate is still non-viable (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given promoting an alternate, when the primary changes, then the old primary stays approved only if the edit puts it in the list (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given a failing alternate, when evaluated, then C001/C002 are not raised for it and I002 names each failing check, fit, bound or require (language.md 3.2; obligations.md 2.3)"
bound = false
+++
