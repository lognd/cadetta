+++
id = "01M52N4A2CFMX2RJ32RWA4PAR9"
title = "Conformance: waived diagnostics no longer counted; fix w001-w005 fixtures"
type = "bug"
category = "in-progress"
priority = "high"
points = 3
reporter = "lognd"
created = "2026-10-16T15:26:14Z"
updated = "2026-10-16T15:26:16Z"
scope = ["tests/conformance/lints/w00*", "tests/conformance/obligations/w003-*"]

[[acceptance]]
text = "lints/w001..w005 and obligations/w003-* conformance fixtures pass; must-not-fire blocks expect no waived/allowed diagnostics"
bound = false
+++

Runner (4fab6fa9) drops waived diagnostics; fixtures still listed the waived C001/M001 as expected produced. Fixtures were wrong, engine and spec (lints.md 2) right.
