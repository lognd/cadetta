+++
id = "01M4SPWSQQNCC23KA25TCFVQ98"
title = "Headless freecadcmd CI job"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWHC2VWRX6V0G4NZKYPB9"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-14T12:44:01Z"
persona = "cadetta contributor"
capability = "run the FreeCAD adapter tests under freecadcmd in CI"
outcome_text = "read and drive are exercised without a license"
labels = ["creates:.github/workflows/freecad.yml", "creates:adapters/freecad/tests/**"]
scope = [".github/workflows/freecad.yml", "adapters/freecad/tests/**"]

[[acceptance]]
text = "Given a pull request, when the freecad job runs, then emit, push, regen and read round trip passes headless"
bound = true
+++
