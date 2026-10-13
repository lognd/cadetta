+++
id = "01M4SV3ADT95XD9V1CFNEHY9X7"
title = "SOLIDWORKS release smoke on a licensed machine"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:17:23Z"
updated = "2026-10-13T05:17:24Z"
persona = "release manager"
capability = "verify the real COM path before release"
labels = ["creates:adapters/solidworks/SMOKE.md"]
scope = ["adapters/solidworks/SMOKE.md"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TKZD26CK9FFA1EJXSV4"

[[acceptance]]
text = "Given a licensed Windows machine, when the cadetta sync loop runs on the pilot r3-05 part, then emit, read, push, regen round trip passes and the result is recorded in adapters/solidworks/SMOKE.md (adapters.md 6)"
bound = false
+++
