+++
id = "01M51ZFG091S5HEJV88NKXYVWC"
title = "Evaluate checks ranging over part instances and reads of a part instance"
type = "bug"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-16T09:07:52Z"
updated = "2026-10-16T09:59:53Z"
scope = ["crates/cadetta-sema/src/eval/**", "crates/cadetta-sema/src/checks.rs", "crates/cadetta-sema/src/eval.rs", "crates/cadetta-sema/src/geom.rs"]

[[acceptance]]
text = "A check quantifier ranging over a part array evaluates per instance, and reads of a part-instance member (b.dia) evaluate"
bound = true
+++

found while working ~A1N7FAP: checks.rs and eval.rs say 'evaluated by the parts ticket'; golden 02 seat_creep/seat_squeeze/... and golden 01 Bracket.bore_fits_tube not in IR
