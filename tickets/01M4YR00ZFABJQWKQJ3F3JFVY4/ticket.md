+++
id = "01M4YR00ZFABJQWKQJ3F3JFVY4"
title = "language: N001 case, N004 was, N006 type-read-beside-instance sema gaps"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:59:22Z"
updated = "2026-10-15T03:31:29Z"
labels = ["creates:changelog.d/01M4YR00ZFABJQWKQJ3F3JFVY4.fixed.md"]
scope = ["crates/cadetta-sema/**", "crates/cadetta-lint/**", "docs/spec/lints.md", "tests/conformance/known-gaps.toml", "changelog.d/01M4YR00ZFABJQWKQJ3F3JFVY4.fixed.md"]

[[acceptance]]
text = "N001: a lowercase component name is reported"
bound = true

[[acceptance]]
text = "N004: a stale item name in a was path or read is reported"
bound = true

[[acceptance]]
text = "N006: a type member read beside an instance is reported"
bound = true
+++

Split from ~8WN9V3R. Fixtures tests/conformance/language/n001-case, n004-was, n006-type-read-beside-instance disagree with the binary (see tests/conformance/known-gaps.toml).
