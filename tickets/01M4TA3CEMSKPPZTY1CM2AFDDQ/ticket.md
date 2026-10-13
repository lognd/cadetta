+++
id = "01M4TA3CEMSKPPZTY1CM2AFDDQ"
title = "Switch why/impact/ir/ack/obligations verbs to pipeline::analyze"
type = "bug"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-13T09:52:56Z"
scope = ["crates/cadetta-cli"]

[[acceptance]]
text = "Given a project with sources, when why, impact, ir, ack, obligations or why --stale run, then they lower sources through pipeline::analyze instead of stub project_candidates, and a test per verb shows a real (non-empty) result"
bound = false
+++

found while working ~E631Y52. project_candidates are stubs.
