+++
id = "01M50Z764H4P103XX06KB9GCES"
title = "Fix s003-solver-stale conformance: must-fire loses S003 and gets C002"
type = "bug"
category = "in-progress"
priority = "high"
points = 2
reporter = "lognd"
created = "2026-10-15T23:44:05Z"
updated = "2026-10-16T00:20:29Z"
idempotency_key = "s003-solver-stale-fix"
scope = ["tests/conformance/**", "crates/cadetta-cli/src/**"]

[[acceptance]]
text = "conformance entry obligations/s003-solver-stale passes on goway"
bound = true
+++

On Dwarf the obligations/s003-solver-stale entry fails: S003 (warning) missing and an extra C002 at acme.demo::Frame.wall_ok. The must-fire dir has analysis/wall_sizing.out.json with zero digests and min_wall=2mm, so the recorded output should be read (stale, S003) rather than treated as unsolved (C002). Suspect the solver output sidecar lookup (solver path key acme.demo::Frame.wall_sizing, file naming) or ~MGXDNYP's check/bind_solver_outputs. Introduced by 61a635d3.
