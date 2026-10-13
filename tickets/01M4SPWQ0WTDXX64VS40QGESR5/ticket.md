+++
id = "01M4SPWQ0WTDXX64VS40QGESR5"
title = "Fit declarations and fit type"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T08:16:08Z"
persona = "design engineer"
capability = "declare `fit wheel_axle = H7/p6 on Wheel.hub_bore`"
outcome_text = "fit members and type (clearance, transition, interference) are computed, never authored"
labels = ["creates:crates/cadetta-sema/src/fits.rs"]
scope = ["crates/cadetta-sema/src/fits.rs", "docs/spec/language.md", "crates/cadetta-sema/src/lib.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWQH44JWFXZDNTGEBMMNX"

[[acceptance]]
text = "Given `fit f = H7/p6 on Wheel.hub_bore`, when evaluated, then f.hole, f.shaft, f.clearance and f.type (interference) exist (language.md 3.7)"
bound = false

[[acceptance]]
text = "Given `p6 on wheel.hub_bore` and `H7 on motor.shaft`, when evaluated, then the class letter case picks the side it plays and x is the other side (language.md 3.7)"
bound = false

[[acceptance]]
text = "Given `fit insert = hole / Insert.od`, when evaluated, then both sides are paths with their own tolerances (language.md 3.7)"
bound = false

[[acceptance]]
text = "Given a required range `[-0.4mm .. -0.1mm]`, when the clearance lies entirely outside it, then F004 violates; when it overlaps only partly then C002 undecided (language.md 3.7)"
bound = false

[[acceptance]]
text = "Given the fitted parameter carries a disagreeing tolerance, when analysed, then F003 is signalled (language.md 3.7)"
bound = false

[[acceptance]]
text = "Given a fit side, when evaluated, then it has the hole or shaft role (quantities.md 5.6)"
bound = false
+++
