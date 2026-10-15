+++
id = "01M4V2K8XCW9XJ9CRKJA7FN9YF"
title = "W003/W004/W005 waiver validation incomplete"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:47:40Z"
updated = "2026-10-15T02:51:40Z"
scope = ["crates/cadetta-lint/**", "tests/conformance/known-gaps.toml", "changelog.d/01M4V2K8XCW9XJ9CRKJA7FN9YF.fixed.md"]

[[acceptance]]
text = "W003 fires for allow of a fixed code and waive of a require; W004 for an unused allow; W005 for an expiring waiver"
bound = false
+++

Fixtures lints/w003-allow-fixed-code, w003-waive-require, w004-unused-allow, w005-waiver-expiring: none fire; a waiver of a require's C001 wrongly suppresses it. found while working ~T71T26K
