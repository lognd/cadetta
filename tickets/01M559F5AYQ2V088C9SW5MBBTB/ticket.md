+++
id = "01M559F5AYQ2V088C9SW5MBBTB"
title = "SOLIDWORKS: round trip the real r3-05 pilot and click the menu Link verb for count and tolerance on nova"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-17T16:00:10Z"
updated = "2026-10-17T17:25:57Z"
scope = ["adapters/solidworks/**", "docs/spec/solidworks.md", "changelog.d/01M559F5AYQ2V088C9SW5MBBTB.fixed.md"]

[[acceptance]]
text = "Round trip of the r3-05 lib.cdt on nova: sync, R001 cad move, import --update, no-op re-sync (evidence/w5mb-live run2/run4/run5)"
bound = true

[[acceptance]]
text = "Pattern count reads as a unitless 4 (DimensionUnitsTests KindOf)"
bound = true

[[acceptance]]
text = "Menu Link callback exercised for a tolerance and a count; real click not automatable, reason recorded (evidence/w5mb-live/addin-menu.log, SMOKE.md)"
bound = true
+++

found while working ~PF7XCNP; see adapters/solidworks/SMOKE.md
