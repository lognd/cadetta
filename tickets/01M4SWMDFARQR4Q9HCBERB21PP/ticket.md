+++
id = "01M4SWMDFARQR4Q9HCBERB21PP"
title = "Session attach and hidden start"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-13T05:44:14Z"
persona = "design engineer"
capability = "drive SOLIDWORKS unattended"
idempotency_key = "replan14:sw-session"
labels = ["creates:adapters/solidworks/src/session.py"]
scope = ["adapters/solidworks/src/session.py"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TGHW85Z0PZS8QHBFB4S"

[[acceptance]]
text = "Given a running SOLIDWORKS, when open runs, then the adapter attaches; otherwise it starts one hidden (adapters.md 2.2)"
bound = false
+++
