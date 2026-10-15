+++
id = "01M4V6BZ0MPYS6XXSC1992PZ41"
title = "check does not load dependencies: E012, E013, E014 and cross-package use unimplemented"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:53:35Z"
updated = "2026-10-15T03:20:11Z"
scope = ["crates/cadetta-cli/**"]

[[acceptance]]
text = "check reports E012 and E013 for unmet or conflicting dependency version requirements"
bound = false

[[acceptance]]
text = "check reports E014 when a dependency's content differs from its lock digest"
bound = false

[[acceptance]]
text = "dependency packages are loaded so cross-package use resolves and E016 applies across packages"
bound = false
+++

Found while working ~JKTB9TF. check never resolves [dependencies]: version requirements (E012, E013), lock digests (E014) are not checked and 'use acme.shop::X' gives E002 unknown package, so cross-package visibility (E016/E017 across packages) and pub use re-exports are untestable. Fixtures: packages/e012-*, e013-*, e014-*, e016-*-dependency, pub-use-*. Spec: packages.md 1, 3, 4.
