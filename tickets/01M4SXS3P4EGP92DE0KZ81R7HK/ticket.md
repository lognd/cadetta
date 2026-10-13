+++
id = "01M4SXS3P4EGP92DE0KZ81R7HK"
title = "Renames and moves across modules with was"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:04:14Z"
updated = "2026-10-13T09:10:42Z"
persona = "library author"
capability = "move a file or item without breaking bindings, waivers or attestations"
labels = ["creates:crates/cadetta-sema/src/renames.rs"]
scope = ["crates/cadetta-sema/src/renames.rs", "crates/cadetta-sema/src/deps.rs", "crates/cadetta-sema/src/visibility.rs", "crates/cadetta-sema/src/lib.rs", "docs/spec/packages.md", "docs/spec/lints.md"]

[[links]]
kind = "blocked-by"
target = "01M4SPWPG1WBAHFPZ19ZV3BAPJ"

[[links]]
kind = "blocked-by"
target = "01M4SXRFTTTTJE5CHCZAE8MX0Q"

[[links]]
kind = "blocked-by"
target = "01M4SXRV606ZNADV511Z42YR0Q"

[[acceptance]]
text = "Given `was` with a full canonical path, when an item moves to another module, file or package, then references and bindings, waivers and attestations on the old path carry over (packages.md 5)"
bound = false

[[acceptance]]
text = "Given a use of the old path, when resolved, then it resolves and N004 is emitted (packages.md 5; lints.md N004)"
bound = false

[[acceptance]]
text = "Given a library that removes or renames a pub item or non-priv member without was, changes the kind of either, or changes a value a dependent verdict reads, when compared with the previous version, then it is reported as a semver-major change (packages.md 5)"
bound = false
+++
