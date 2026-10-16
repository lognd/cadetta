+++
id = "01M4V6C5P91N220MBNZ4DKBZVW"
title = "packages.md: pub use of a private item and use of a plain use through a glob decide E002 versus E016"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T17:53:42Z"
updated = "2026-10-16T07:24:40Z"
scope = ["docs/spec/**"]
+++

Spec ambiguity found while writing ~JKTB9TF fixtures. packages.md 3 says a pub use of a private item is not re-exported and 4 says a glob skips plain use, but neither names the diagnostic a dependent gets. Fixtures packages/pub-use-private-item-not-reexported and e002-glob-skips-plain-use assume E002 (unresolved). Confirm or specify.
