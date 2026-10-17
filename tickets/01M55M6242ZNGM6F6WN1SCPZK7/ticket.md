+++
id = "01M55M6242ZNGM6F6WN1SCPZK7"
title = "SOLIDWORKS: rerun count-link and import live check after unit fix, rebind criterion 6 evidence"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T19:07:26Z"
updated = "2026-10-17T19:23:10Z"
scope = ["adapters/solidworks/evidence/1scpzk7-live/**", "adapters/solidworks/tools/w5mb/**", "adapters/solidworks/DEVELOPING.md"]
+++

Criterion 6 evidence (w5mb-live/run5.log) predates the ~YMEEPJ0 / ~QK2F312 dimensionless fix and shows 4mm for a count. Rerun the live count-link and import check on nova with the fixed build, store the log under adapters/solidworks/evidence/, and rebind milestone 0.1.0 criterion 6 evidence. Live rules: log open docs before/after each step, close what you open without saving, leave none open, relaunch SOLIDWORKS only if it dies. Reach nova only via scratchpad nova.sh scripts.
