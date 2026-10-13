+++
id = "01M4SWMDM7FRKACF83CT4H5X0H"
title = "Save after sync and file digest"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-13T05:44:15Z"
persona = "design engineer"
capability = "persist pushed values"
idempotency_key = "replan14:sw-save"
labels = ["creates:adapters/solidworks/src/save.py"]
scope = ["adapters/solidworks/src/save.py"]

[[links]]
kind = "blocked-by"
target = "01M4SWMDFARQR4Q9HCBERB21PP"

[[acceptance]]
text = "Given save = after-sync, when sync succeeds, then files are saved and file digests recorded; save = never leaves them dirty (adapters.md 2.2; config-cli.md 2)"
bound = false
+++
