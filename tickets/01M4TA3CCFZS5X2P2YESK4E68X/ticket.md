+++
id = "01M4TA3CCFZS5X2P2YESK4E68X"
title = "Embed std in the cadetta binary"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-15T01:39:07Z"
scope = ["crates/cadetta-cli", "docs/spec/config-cli.md", "changelog.d/01M4TA3CCFZS5X2P2YESK4E68X.fixed.md"]

[[acceptance]]
text = "The std sources are compiled into the binary and match the std/ directory exactly"
bound = true

[[acceptance]]
text = "A project importing std resolves std with no std/ on disk and an unknown std variant still errors"
bound = true
+++

found while working ~E631Y52. std loaded from CADETTA_STD or repo std/ only.
