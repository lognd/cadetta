+++
id = "01M4SXRV606ZNADV511Z42YR0Q"
title = "Use resolution order, globs, re-exports and prelude"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:04:06Z"
updated = "2026-10-13T08:03:43Z"
persona = "product engineer"
capability = "import shop policy and parts by name"
labels = ["creates:crates/cadetta-sema/src/imports.rs"]
scope = ["crates/cadetta-sema/src/imports.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SXRFTTTTJE5CHCZAE8MX0Q"

[[links]]
kind = "blocked-by"
target = "01M4SXRFXB1JYNPNKSAY6WB3QC"

[[acceptance]]
text = "Given a bare name, when resolved, then the order is the component members and arguments, current-module items, explicit use names, glob names, then the prelude (packages.md 4; language.md 4)"
bound = false

[[acceptance]]
text = "Given two globs bringing the same name, when the name is used, then E004 fires only at the use; an unresolved name is E002 (packages.md 4)"
bound = false

[[acceptance]]
text = "Given `pub use self::fasteners::{ScrewJoint, MetricThread}` in lib.cdt, when a dependent writes acme.shop::ScrewJoint, then it resolves to the definition and its canonical path is still the definition path (packages.md 4)"
bound = false

[[acceptance]]
text = "Given a fully qualified path with no use, when resolved, then it is legal anywhere (packages.md 4)"
bound = false

[[acceptance]]
text = "Given the prelude units and kinds, when no use is written, then they resolve and nothing else from std is in scope (language.md 4)"
bound = false

[[acceptance]]
text = "Given declaration order reversed, when resolved, then results are identical (language.md 4)"
bound = false
+++
