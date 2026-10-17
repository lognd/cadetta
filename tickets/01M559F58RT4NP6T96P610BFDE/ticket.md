+++
id = "01M559F58RT4NP6T96P610BFDE"
title = "SOLIDWORKS: push-path R004 message does not name the failing feature"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-17T16:00:10Z"
updated = "2026-10-17T16:07:18Z"
scope = ["adapters/solidworks/**"]

[[acceptance]]
text = "Given a push whose rebuild fails at a feature, when R004 is reported, then the message names that feature"
bound = true
+++

found while working ~PF7XCNP; see adapters/solidworks/SMOKE.md
