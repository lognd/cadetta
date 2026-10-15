+++
id = "01M4YR00ZFABJQWKQJ3F3JFVY4"
title = "language: N001 case, N004 was, N006 type-read-beside-instance sema gaps"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:59:22Z"
updated = "2026-10-15T03:30:18Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-lint/**"]

[[acceptance]]
text = "N001: a lowercase component name is reported"
bound = false

[[acceptance]]
text = "N004: a stale item name in a was path or read is reported"
bound = false
+++

Split from ~8WN9V3R. Fixtures tests/conformance/language/n001-case, n004-was, n006-type-read-beside-instance disagree with the binary (see tests/conformance/known-gaps.toml).
