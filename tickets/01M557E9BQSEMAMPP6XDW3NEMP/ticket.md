+++
id = "01M557E9BQSEMAMPP6XDW3NEMP"
title = "Fusion live run on nova: exercise addin/CadettaFusion (open, read, push, save, close)"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-17T15:24:44Z"
updated = "2026-10-17T15:55:37Z"
scope = ["adapters/fusion/evidence/dw3nemp-live/**", "docs/spec/fusion.md", "changelog.d/01M557E9BQSEMAMPP6XDW3NEMP.changed.md"]
+++

Follow-up of PQMSJDX. The in-Fusion add-in (adapters/fusion/addin/CadettaFusion) is written against the documented adsk API but never run in Fusion (docs/spec/fusion.md 8). Live run on nova (Windows ARM64): install add-in, read/push/save/close a document, log open docs before and after, leave none open.
