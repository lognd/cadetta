+++
id = "01M4SV3ADT95XD9V1CFNEHY9X7"
title = "SOLIDWORKS release smoke on a licensed machine"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:17:23Z"
updated = "2026-10-17T15:50:55Z"
persona = "release manager"
capability = "verify the real COM path before release"
labels = ["creates:adapters/solidworks/SMOKE.md"]
scope = ["adapters/solidworks/SMOKE.md", "adapters/solidworks/evidence/**", "adapters/solidworks/DEVELOPING.md", "adapters/solidworks/src/**", "adapters/solidworks/tests/**"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TKZD26CK9FFA1EJXSV4"

[[links]]
kind = "blocked-by"
target = "01M4SV2TR8ENTA87J1DDKCF816"

[[links]]
kind = "blocked-by"
target = "01M4SV2TVM50WJRZ42ZHXVD96N"

[[links]]
kind = "blocked-by"
target = "01M4VB7VXXHJ6TKHF6J2STC1FE"

[[acceptance]]
text = "Given a licensed Windows machine, when the cadetta sync loop runs on the pilot r3-05 part, then emit, read, push, regen round trip passes and the result is recorded in adapters/solidworks/SMOKE.md (adapters.md 6)"
bound = false

[[acceptance]]
text = "Given a release smoke test, when run on Windows, then attach, sync, save and restore are exercised against a real SOLIDWORKS (adapters.md 2)"
bound = false

[[acceptance]]
text = "Given the release smoke, when run, then the add-in links a dimension, a pattern count and a tolerance, creates datum frames and assigns parts, and the sync, cad-moved R001, import --update, no-op sequence of solidworks.md 6 passes and is recorded as transcripts"
bound = false
+++
