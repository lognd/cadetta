+++
id = "01M4SPWSFF7Z2K6FSGSZ1N30JH"
title = "FreeCAD emit to Spreadsheet aliases"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWHC2VWRX6V0G4NZKYPB9"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-14T03:34:08Z"
persona = "design engineer"
capability = "emit parameters as Spreadsheet aliases in an FCStd file"
outcome_text = "FreeCAD models update without retyping"
labels = ["creates:adapters/freecad/src/cadetta_freecad/emit.py", "creates:adapters/freecad/tests/unit/test_emit.py", "creates:adapters/freecad/tests/unit/_ir.py"]
scope = ["adapters/freecad/src/cadetta_freecad/adapter.py", "adapters/freecad/src/cadetta_freecad/backend.py", "adapters/freecad/pyproject.toml", "adapters/freecad/uv.lock", "adapters/freecad/README.md", "docs/spec/adapters.md", "adapters/freecad/src/cadetta_freecad/emit.py", "adapters/freecad/tests/unit/test_emit.py", "adapters/freecad/tests/unit/_ir.py", "adapters/freecad/tests/unit/test_freecad_adapter.py", "adapters/freecad/tests/unit/test_adapter.py"]

[[links]]
kind = "blocks"
target = "01M4SPWSJ81C9Q9BV5AYXHEWE6"

[[acceptance]]
text = "Given a design IR, when emitted twice, then the output is deterministic"
bound = true

[[acceptance]]
text = "Given a value not representable at precision, when emitted, then Q004 is returned"
bound = true

[[acceptance]]
text = "Given a path with several locators, when emitted to FreeCAD, then each alias is written and ref-mode aliases reference the emitted global (adapters.md 3)"
bound = true
+++
