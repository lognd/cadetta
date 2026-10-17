+++
id = "01M5596EN1KFSFCYTBW6GWEY93"
title = "Fusion live run needs a signed-in Autodesk account on nova"
type = "task"
category = "done"
outcome = "duplicate"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T15:55:25Z"
updated = "2026-10-17T18:15:17Z"
+++

Found while working ~DW3NEMP: Fusion on nova stops at the Sign In screen, so addin/CadettaFusion never loads. Needs the user to sign in to Fusion on nova (and provide a test design with user parameters), then rerun open/read/push/save/close with before/after open-document logs. See adapters/fusion/evidence/dw3nemp-live/.
