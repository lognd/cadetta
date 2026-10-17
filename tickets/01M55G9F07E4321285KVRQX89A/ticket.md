+++
id = "01M55G9F07E4321285KVRQX89A"
title = "Fusion: doc.open accepts a local .f3d path so live runs need no cloud design id"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-17T17:59:23Z"
updated = "2026-10-17T18:12:59Z"
scope = ["adapters/fusion/**", "docs/spec/fusion.md"]

[[acceptance]]
text = "doc.open accepts a local .f3d path and the add-in unit tests cover open/save/close of it"
bound = true

[[acceptance]]
text = "A live run on nova against a signed-in Fusion opened, read, pushed, saved and closed a script-made design with user parameters, logging open documents before and after each step and leaving none open"
bound = true
+++

Found in ~6GWEY93: Fusion is signed in and the add-in answers on nova, but doc.open uses app.data.findFileById and needs a cloud data-file id. Add a local-path branch (importManager.importToNewDocument or open of .f3d) in addin/CadettaFusion, create a small test design with user parameters on nova via script, then run open/read/push/save/close live with before/after open-document logs. Evidence under adapters/fusion/evidence/. Scope: adapters/fusion/**, docs/spec/fusion.md.
