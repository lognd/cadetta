+++
id = "01M4SPWSMZASH05AGR95PGAZKD"
title = "FreeCAD push, regen and R004"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4SPWHC2VWRX6V0G4NZKYPB9"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-14T11:58:56Z"
persona = "design engineer"
capability = "push changed params, regenerate and read back headless"
outcome_text = "sync is hands-free"
labels = ["creates:adapters/freecad/src/drive.py", "creates:adapters/freecad/src/cadetta_freecad/drive.py", "creates:adapters/freecad/tests/unit/test_drive.py"]
scope = ["adapters/freecad/src/drive.py", "adapters/freecad/src/cadetta_freecad/backend.py", "adapters/freecad/src/cadetta_freecad/adapter.py", "adapters/freecad/README.md", "adapters/freecad/tests/unit/test_read.py", "docs/spec/adapters.md", "adapters/freecad/src/cadetta_freecad/drive.py", "adapters/freecad/tests/unit/test_drive.py"]

[[links]]
kind = "blocks"
target = "01M4SPWSQQNCC23KA25TCFVQ98"

[[links]]
kind = "blocks"
target = "01M4SPWTDD069AXXQBXD4T6ATZ"

[[acceptance]]
text = "Given a changed digest, when pushed, then only that alias is updated"
bound = true

[[acceptance]]
text = "Given a rebuild failure, when regen runs, then R004 regen-failed is returned"
bound = true

[[acceptance]]
text = "Given a session with unsaved user changes, when driving, then the adapter refuses and reports (OQ-8)"
bound = true
+++
