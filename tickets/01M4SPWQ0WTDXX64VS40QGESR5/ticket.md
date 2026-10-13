+++
id = "01M4SPWQ0WTDXX64VS40QGESR5"
title = "Fit declarations and fit type"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T04:03:52Z"
persona = "design engineer"
capability = "declare `fit wheel_axle = H7/p6 on Wheel.hub_bore`"
outcome_text = "fit members and type (clearance, transition, interference) are computed, never authored"
labels = ["creates:crates/cadetta-sema/src/fits.rs"]
scope = ["crates/cadetta-sema/src/fits.rs"]

[[acceptance]]
text = "Given `fit f = H7/p6 on Wheel.hub_bore`, when evaluated, then f.hole, f.shaft and fit_type=interference exist"
bound = false

[[acceptance]]
text = "Given the fitted parameter carries a disagreeing tolerance, when analysed, then F003 is signalled"
bound = false
+++
