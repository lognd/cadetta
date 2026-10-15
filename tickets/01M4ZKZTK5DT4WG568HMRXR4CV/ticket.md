+++
id = "01M4ZKZTK5DT4WG568HMRXR4CV"
title = "roll-ups do not read non-argument members of parts nested in a parameterized callee (screw mass inside ScrewJoint)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T11:08:35Z"
updated = "2026-10-15T12:48:34Z"
scope = ["crates/cadetta-sema/**", "docs/spec/language.md"]

[[acceptance]]
text = "A roll-up reads non-argument members and defaulted parameters of parts nested in a parameterized callee, at any nesting depth"
bound = true
+++

found while working ~A50KFBS: rolleval reads only the argument a nested part site passes for the roll-up parameter; other members (mass, lead_time of a nested Iso4762) need nested instances materialised.
