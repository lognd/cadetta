+++
id = "01M4TA3BTW256J8FKMS6KASY82"
title = "Sema resolve/eval/dep diagnostics lack module identity"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:33Z"
updated = "2026-10-14T21:50:58Z"
scope = ["crates/cadetta-sema", "crates/cadetta-cli", "docs", "changelog.d", "tests/conformance"]

[[acceptance]]
text = "Resolve errors (E002/E003/E004/E016/E017) carry the module they were raised in"
bound = true

[[acceptance]]
text = "Evaluation diagnostics carry the module they were raised in"
bound = true

[[acceptance]]
text = "Dependency diagnostics (D003, N004) carry the module they were raised in"
bound = true

[[acceptance]]
text = "The check pipeline attributes a diagnostic to its own module's file, dropping std, instead of guessing from the span"
bound = true
+++

found while working ~E631Y52. Check attributes these heuristically to project files and drops std ones.
