+++
id = "01M4SPWSJ81C9Q9BV5AYXHEWE6"
title = "FreeCAD read to observed IR"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWHC2VWRX6V0G4NZKYPB9"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-14T11:49:58Z"
persona = "design engineer"
capability = "import existing FCStd spreadsheet aliases and dimensions"
outcome_text = "I can lint a design before any .cdt exists"
labels = ["creates:adapters/freecad/src/cadetta_freecad/read.py", "creates:adapters/freecad/tests/unit/test_read.py"]
scope = ["adapters/freecad/src/cadetta_freecad/adapter.py", "adapters/freecad/src/cadetta_freecad/backend.py", "adapters/freecad/src/cadetta_freecad/read.py", "adapters/freecad/tests/unit/test_read.py", "adapters/freecad/README.md"]

[[links]]
kind = "blocks"
target = "01M4SPWSMZASH05AGR95PGAZKD"

[[acceptance]]
text = "Given an FCStd with a driven dimension, when read, then the value has exact:false with a reason"
bound = false

[[acceptance]]
text = "Given emit then read, when compared, then IR values are equal (round trip)"
bound = false
+++
