+++
id = "01M4SWMDPP54TE0XZGJRTKN07N"
title = "Snapshot and atomic restore"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-13T05:44:15Z"
persona = "design engineer"
capability = "never leave a broken model"
idempotency_key = "replan14:sw-restore"
labels = ["creates:adapters/solidworks/src/restore.py"]
scope = ["adapters/solidworks/src/restore.py"]

[[links]]
kind = "blocked-by"
target = "01M4SWMDFARQR4Q9HCBERB21PP"

[[acceptance]]
text = "Given a failed rebuild, when sync ends, then the pre-push file state is restored (adapters.md 2.3)"
bound = false
+++
