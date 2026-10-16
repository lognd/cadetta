+++
id = "01M4V43BMZDEJT779G1N69K25Y"
title = "Spec decisions from geometry conformance fixtures"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-13T17:13:56Z"
updated = "2026-10-16T07:20:09Z"
scope = ["docs/spec/geometry.md"]

[[acceptance]]
text = "geometry.md records the six decisions (flip(z) E006, datum order/unknown E002, zero point count E006, unknown placed-part datum E002, circle_pattern CCW from +x, relation conditions usable in check)"
bound = true
+++

found while working ~ZFM3633. Decisions written into geometry.md: flip(z) E006, datum built from later or unknown datum E002, zero point count E006, unknown datum of a placed part E002, circle_pattern counter-clockwise from +x, aligned/coaxial/parallel/angle_between usable as check conditions. Review by the owner.
