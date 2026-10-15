+++
id = "01M4YR00X3TTZGSBAKQZDNTT6T"
title = "language: wire VisibilityChecker into check (E016/E017 privacy)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:59:22Z"
updated = "2026-10-15T03:01:53Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-cli/src/pipeline.rs", "tests/conformance/**", "crates/cadetta-cli/tests/privacy.rs", "changelog.d/01M4YR00X3TTZGSBAKQZDNTT6T.*"]

[[acceptance]]
text = "E016 fires for a use of a private item from a non-descendant module (language/e016-private-item)"
bound = false

[[acceptance]]
text = "E017 fires for a private type in a public signature (language/e017-private-in-public)"
bound = false
+++

Split from ~8WN9V3R. VisibilityChecker (crates/cadetta-sema/src/visibility.rs) exists but is not called from pipeline.rs. Directory fixtures tests/conformance/language/e016-private-item and e017-private-in-public disagree with the binary; remove their known-gaps.toml entries when fixed (also add tests/conformance/** to scope).
