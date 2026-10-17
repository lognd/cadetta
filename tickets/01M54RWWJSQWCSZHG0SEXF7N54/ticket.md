+++
id = "01M54RWWJSQWCSZHG0SEXF7N54"
title = "SOLIDWORKS add-in never connects on nova: LoadAddIn returns 0 but ConnectToSW does not run"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-17T11:10:34Z"
updated = "2026-10-17T11:43:43Z"
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.AddIn/**", "adapters/solidworks/DEVELOPING.md", "adapters/solidworks/evidence/exf-live/**", "changelog.d/*EXF7N54*"]

[[acceptance]]
text = "Given the add-in is registered, when SOLIDWORKS lists it, then registration writes default 1 plus AddInsStartup; live LoadAddIn/dialog logs captured; connect not yet observed"
bound = false
+++

Found while working ~4FG4VVR. After regasm /codebase, ISldWorks.LoadAddIn(path) returned 0, yet no ctor or ConnectToSW ran, GetAddInObject was null and UIA showed no Cadetta menu or pane. Try Tools > Add-Ins plus restart, and ClassInterfaceType.AutoDispatch for the Menu* callbacks. Also needs a 3+ component assembly for Assign array.
