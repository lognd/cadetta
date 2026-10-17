+++
id = "01M559F56CVTCX37155XMNTFDK"
title = "SOLIDWORKS: pattern count dimension scaled by length factor (reads x1000)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-17T16:00:10Z"
updated = "2026-10-17T16:05:28Z"
scope = ["adapters/solidworks/**"]

[[acceptance]]
text = "Given a scalar (pattern count) dimension, when pushed or read through SwComPushDocument, then its value is not scaled by the length factor"
bound = true
+++

found while working ~PF7XCNP; see adapters/solidworks/SMOKE.md
