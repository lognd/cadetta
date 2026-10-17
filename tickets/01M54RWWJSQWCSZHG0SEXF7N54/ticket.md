+++
id = "01M54RWWJSQWCSZHG0SEXF7N54"
title = "SOLIDWORKS add-in never connects on nova: LoadAddIn returns 0 but ConnectToSW does not run"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-17T11:10:34Z"
updated = "2026-10-17T11:10:34Z"
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.AddIn/**"]
+++

Found while working ~4FG4VVR. After regasm /codebase, ISldWorks.LoadAddIn(path) returned 0, yet no ctor or ConnectToSW ran, GetAddInObject was null and UIA showed no Cadetta menu or pane. Try Tools > Add-Ins plus restart, and ClassInterfaceType.AutoDispatch for the Menu* callbacks. Also needs a 3+ component assembly for Assign array.
