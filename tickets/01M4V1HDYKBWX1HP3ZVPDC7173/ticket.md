+++
id = "01M4V1HDYKBWX1HP3ZVPDC7173"
title = "Quantities: C004 derived-feature-in-check never fires"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:12Z"
updated = "2026-10-15T02:19:57Z"
idempotency_key = "Z48-c004"
scope = ["crates/cadetta-lint/**", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/c004_e010.rs", "docs/spec/lints.md", "tests/conformance/known-gaps.toml", "changelog.d/01M4V1HDYKBWX1HP3ZVPDC7173.fixed.md"]

[[acceptance]]
text = "C004 fires on the derived parameter read by a check, once per parameter, and not for let"
bound = true
+++

found while working ~Z48GXKX. quantities.md 5.3. Fixture c004.
