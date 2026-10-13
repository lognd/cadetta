+++
id = "01M4SPWPXSG1YKHGF4KV5G43BT"
title = "ISO 286 fit tables (cadetta-fits)"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T06:32:35Z"
persona = "design engineer"
capability = "resolve hole and shaft classes like H7, p6, g6 for any size in range"
outcome_text = "fits are values, not comments"
labels = ["creates:crates/cadetta-fits/**"]
scope = ["crates/cadetta-fits/**", "docs/spec/language.md"]

[[links]]
kind = "blocks"
target = "01M4SPWQ0WTDXX64VS40QGESR5"

[[acceptance]]
text = "Given `20mm H7`, when resolved, then the tolerance is +0.021/0 mm per ISO 286"
bound = true

[[acceptance]]
text = "Given a size outside the table range, when resolved, then F002 is signalled"
bound = false

[[acceptance]]
text = "Given class `Z99`, when resolved, then F001 is signalled"
bound = false

[[acceptance]]
text = "Given uppercase and lowercase letters, when resolved, then they are hole and shaft respectively"
bound = false

[[acceptance]]
text = "Given `Bearing.bore.nominal j5` (a class on any expression), when resolved, then the tolerance comes from std.fits.iso286 (quantities.md 5.1)"
bound = false
+++
