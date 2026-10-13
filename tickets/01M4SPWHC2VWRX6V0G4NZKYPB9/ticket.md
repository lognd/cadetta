+++
id = "01M4SPWHC2VWRX6V0G4NZKYPB9"
title = "FreeCAD reference adapter (adapters/freecad)"
type = "epic"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T04:03:47Z"
updated = "2026-10-13T05:17:09Z"
labels = ["creates:adapters/freecad/**"]
scope = ["adapters/freecad/**"]
+++

Outcome: the CI reference adapter for protocol conformance. FreeCAD is the only target that exercises read and drive headless without a license or Windows (adapters.md 6), so the protocol conformance tests run against it. SOLIDWORKS is the primary user-facing target (adapters/solidworks epic).
