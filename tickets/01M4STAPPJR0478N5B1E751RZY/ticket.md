+++
id = "01M4STAPPJR0478N5B1E751RZY"
title = "std citation audit"
type = "story"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:57Z"
updated = "2026-10-14T05:27:31Z"
persona = "design engineer"
capability = "trust every std value"
idempotency_key = "replan13:std citation audit"
scope = ["std/src/**"]

[[links]]
kind = "blocks"
target = "01M4STAPTT2HJK6J5Y2HP6PTJH"

[[links]]
kind = "blocks"
target = "01M4STCFHFMG7999J9WDPK1BC3"

[[acceptance]]
text = "Given the whole std tree, when linted, then no S001 is raised, as stdlib.md intro requires every std value cited (stdlib.md intro, lints.md S)"
bound = true

[[acceptance]]
text = "Given a value without a citation, when found, then the audit fails (stdlib.md intro)"
bound = true
+++
