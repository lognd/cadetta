+++
id = "01M4V2KDYHJAB934TC7ZKZTK5Z"
title = "Conformance fixtures for lint codes not runnable via check source"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-13T16:47:46Z"
updated = "2026-10-16T03:12:19Z"
scope = ["tests/conformance/lints/**"]

[[acceptance]]
text = "D001 and D002 have a conformance pair under tests/conformance/lints that passes; lock/adapter/package codes are covered in their own docs, W002 is a registered gap"
bound = true
+++

Follow-up to ~T71T26K. Uncovered codes: D001, D002, M003, C004, G001-G007 (geometry), Q004 (emit), E012-E015 (package), S003-S006 (solver/lock), O001-O004 (lock), R001-R013 (adapter read-back), W002 (lock digests). Need multi-file package, lock or adapter entries the runner skips today, or lint gaps first.
