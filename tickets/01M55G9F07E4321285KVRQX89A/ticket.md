+++
id = "01M55G9F07E4321285KVRQX89A"
title = "Fusion: doc.open accepts a local .f3d path so live runs need no cloud design id"
type = "task"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-17T17:59:23Z"
updated = "2026-10-17T17:59:26Z"
+++

Found in ~6GWEY93: Fusion is signed in and the add-in answers on nova, but doc.open uses app.data.findFileById and needs a cloud data-file id. Add a local-path branch (importManager.importToNewDocument or open of .f3d) in addin/CadettaFusion, create a small test design with user parameters on nova via script, then run open/read/push/save/close live with before/after open-document logs. Evidence under adapters/fusion/evidence/. Scope: adapters/fusion/**, docs/spec/fusion.md.
