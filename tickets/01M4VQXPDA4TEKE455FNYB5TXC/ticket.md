+++
id = "01M4VQXPDA4TEKE455FNYB5TXC"
title = "Geometry values (datum ops, placement, patterns, radial, sqrt) absent from evaluated IR"
type = "bug"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:22Z"
updated = "2026-10-13T23:00:22Z"
scope = ["crates/cadetta-sema/**"]

[[acceptance]]
text = "geometry fixtures pass."
bound = false
+++

found while working ~6BCVN48: geometry/* fixtures expect members like P.qx not present in the evaluated IR (14 fixtures).
