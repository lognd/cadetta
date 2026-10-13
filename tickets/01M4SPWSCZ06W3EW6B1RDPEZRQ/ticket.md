+++
id = "01M4SPWSCZ06W3EW6B1RDPEZRQ"
title = "FreeCAD adapter skeleton and initialize"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWHC2VWRX6V0G4NZKYPB9"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T05:17:09Z"
persona = "cadetta contributor"
capability = "have a Python adapter that speaks the JSON-RPC protocol"
outcome_text = "the reference adapter proves the protocol"
labels = ["creates:adapters/freecad/src/**", "creates:adapters/freecad/pyproject.toml"]
scope = ["adapters/freecad/src/**", "adapters/freecad/pyproject.toml"]

[[links]]
kind = "blocks"
target = "01M4SPWSFF7Z2K6FSGSZ1N30JH"

[[links]]
kind = "blocked-by"
target = "01M4SV2NZT32CAT33D13T86646"

[[acceptance]]
text = "Given `cadetta-freecad`, when initialize is sent, then capabilities emit, read and drive are returned"
bound = false
+++
