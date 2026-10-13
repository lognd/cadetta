+++
id = "01M4SPWNHPTRETQBYCM6EA2GEV"
title = "Kinds and kind algebra"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T04:03:57Z"
persona = "design engineer"
capability = "have kinds such as Torque, Energy, Angle, Ratio refine dimensions"
outcome_text = "a Torque is never silently assigned to an Energy"
labels = ["creates:crates/cadetta-units/src/kind.rs"]
scope = ["crates/cadetta-units/src/kind.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWNNDNT60Z38K24AEKVAE"

[[acceptance]]
text = "Given Length*Length, when multiplied, then the kind is Area"
bound = false

[[acceptance]]
text = "Given a Torque, when assigned to an Energy parameter without `as`, then Q003 kind-mismatch is signalled"
bound = false

[[acceptance]]
text = "Given an anonymous kind of equal dimension, when assigned to a named kind, then it is accepted"
bound = false
+++
