+++
id = "01M4ZKXBHVHAHYVS59AGZ74Z9Y"
title = "Reconcile G008 half-turn (float >= spread) between geometry.md 5.2 and code"
type = "chore"
category = "in-progress"
priority = "low"
points = 1
reporter = "lognd"
created = "2026-10-15T11:07:14Z"
updated = "2026-10-15T12:41:35Z"
idempotency_key = "g008-halfturn-spec"
scope = ["docs/spec/geometry.md"]

[[acceptance]]
text = "geometry.md 5.2 lists the float >= spread case for G008"
bound = true
+++

float::unconstrained_rotation reports G008 when theta is 180deg (float at or above the point spread); geometry.md 5.2 names only a one-point pattern or coincident points. Update the spec text (code and fixture g005-float-half-turn already agree).
