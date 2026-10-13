+++
id = "01M4SV2TGHW85Z0PZS8QHBFB4S"
title = "SOLIDWORKS adapter skeleton, initialize and capabilities"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:17:07Z"
updated = "2026-10-13T05:17:07Z"
persona = "design engineer"
capability = "connect to SOLIDWORKS through the adapter protocol"
labels = ["creates:adapters/solidworks/src/**", "creates:adapters/solidworks/pyproject.toml"]
scope = ["adapters/solidworks/src/**", "adapters/solidworks/pyproject.toml"]

[[acceptance]]
text = "Given the adapter, when initialize is sent, then name, version, capabilities emit, read and drive, native units, precision and platforms (windows) are returned (adapters.md 5, 6)"
bound = false
+++
