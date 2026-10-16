+++
id = "01M4VJ22H45PNX8GE24X9CVYM9"
title = "editors/vscode: publish the extension to the VS Code Marketplace and Open VSX"
type = "story"
category = "triage"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T21:17:54Z"
updated = "2026-10-16T23:38:20Z"
persona = "a designer using VS Code or VSCodium"
capability = "install the Cadetta extension from a marketplace"
outcome_text = "no manual VSIX build is needed"
scope = ["editors/vscode/**"]

[[acceptance]]
text = "Given the owner's publisher accounts, when the release workflow runs vsce publish and ovsx publish, then the extension is listed on both the Marketplace and Open VSX"
bound = false

[[acceptance]]
text = "Given editors/vscode/README.md, when read, then it describes marketplace install and the release procedure"
bound = false
+++

found while working ~98FFJ8V; needs the owner's publisher account and tokens, so not built there.
