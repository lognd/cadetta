+++
id = "01M4ZCEYEDA4W3R18F6420HBMQ"
title = "solve --variants fixtures fail with EVAL at Rover.sizing; runner observes runs via marker file"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T08:57:02Z"
updated = "2026-10-16T15:34:45Z"
scope = ["crates/cadetta-cli/tests/conformance.rs"]

[[acceptance]]
text = "solve --variants run-count fixtures (equal inputs share one run; unchanged inputs read the primary) evaluate and pass, the runner counting runs via the marker file"
bound = false
+++

solvers/variants-equal-inputs-share-one-run and variants-unchanged-inputs-read-primary hit an EVAL error at acme.demo::Rover.sizing before any solver runs (runs observed 0). The runner now observes runs (marker line count) so these pass once evaluation works. Split from ~SK1B9S3.
