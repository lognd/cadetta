+++
id = "01M4ST8EABK1PHDV3MGSY9GBF1"
title = "Interface-typed part slots with primary and alt list"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4ST8E5XX0TX0MBND07JQME1"
reporter = "lognd"
created = "2026-10-13T05:02:42Z"
updated = "2026-10-13T05:05:16Z"
persona = "design engineer"
capability = "name a primary part and approved alternates for a slot"
idempotency_key = "replan13:Interface-typed part slots with primary and alt list"
labels = ["creates:crates/cadetta-variants/src/slot.rs"]
scope = ["crates/cadetta-variants/src/slot.rs"]

[[links]]
kind = "blocks"
target = "01M4ST8EC6HVWM52PEHS43F430"

[[acceptance]]
text = "Given `part wheel[4]: Caster = Vendor4471B alt [Colson2Series]`, when resolved, then the slot has a primary and one alternate, all implementing Caster (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given an alternate that does not implement the interface, when resolved, then an error names it (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given the slot, when its instance paths are built, then every instance of the slot shares the primary and alternates (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given `default(Iface = Impl)`, when a component parameter of that interface type is unset, then the scoped default fills it (language.md 3.11, stdlib.md 4)"
bound = false
+++
