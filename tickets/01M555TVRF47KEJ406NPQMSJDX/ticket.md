+++
id = "01M555TVRF47KEJ406NPQMSJDX"
title = "Fusion adapter: spec and Python external adapter (read, drive)"
type = "story"
category = "todo"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-17T14:56:39Z"
updated = "2026-10-17T14:56:49Z"
scope = ["adapters/fusion/**", "docs/spec/fusion.md", "docs/spec/adapters.md"]

[[acceptance]]
text = "docs/spec/fusion.md states the Fusion adapter: user-parameters mapping, units, read and drive, atomic push, transport (Fusion Python runs only inside Fusion, so an in-Fusion add-in speaks JSON-RPC to the cadetta process)"
bound = false

[[acceptance]]
text = "adapters/fusion implements initialize, read and push on the cadetta.adapter base, with fake-session transcript tests that run on CI without Fusion"
bound = false

[[acceptance]]
text = 'A live run on nova (Fusion is installed under %LOCALAPPDATA%\Fusion360) pushes and reads back a user parameter; open documents are tracked and closed'
bound = false

[[acceptance]]
text = "The target matrix row for Fusion is updated and a workflow runs the fake-session tests"
bound = false
+++
