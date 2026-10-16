+++
id = "01M51H3Z5W0MDKYPX4M1Y7TJNH"
title = "Evaluator rejects .min/.max on fit.clearance in checks (EVAL)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T04:56:54Z"
updated = "2026-10-16T05:04:22Z"
scope = ["crates/cadetta-sema/**"]

[[acceptance]]
text = "check c: seat.clearance.min > 0mm evaluates with no EVAL error"
bound = false
+++

found while working ~5KNEDZF: 'check c: seat.clearance.min > 0mm' reports EVAL 'only .hole, .shaft and .clearance are readable on a fit here', but language.md 3.7 says .clearance is an interval so .min/.max apply. Also blocks D002 end-to-end probing of fit accessor literals.
