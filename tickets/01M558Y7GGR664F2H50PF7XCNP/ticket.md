+++
id = "01M558Y7GGR664F2H50PF7XCNP"
title = "SOLIDWORKS live smoke gaps: pilot source, rebuild-failure restore, pattern-count and tolerance links, datum menu verb"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-17T15:50:55Z"
updated = "2026-10-17T16:00:57Z"
scope = ["adapters/solidworks/**", "docs/spec/solidworks.md", "changelog.d/01M6PF7XCNPZ0000000000000A.added.md"]

[[acceptance]]
text = "Round trip on the r3-05 pilot source on nova"
bound = false

[[acceptance]]
text = "Live rebuild-failure restore exercised"
bound = false

[[acceptance]]
text = "Pattern count link and tolerance link exercised from the menu; datum frame creation verb added or dropped with reason"
bound = false
+++

Gaps left by ~NEHY9X7, listed in adapters/solidworks/SMOKE.md. Live rules: log open docs before and after each step, close without saving, leave none open.
