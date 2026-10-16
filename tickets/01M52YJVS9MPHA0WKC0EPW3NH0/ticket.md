+++
id = "01M52YJVS9MPHA0WKC0EPW3NH0"
title = "cadetta sync report omits diagnostic severity (O004 info)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T18:11:28Z"
updated = "2026-10-16T19:13:52Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-cli/tests/conformance/o004.rs", "docs/spec/config-cli.md", "docs/schemas/sync.json"]

[[acceptance]]
text = "sync JSON diagnostics carry severity (O004 info, blocking codes error, others warning); o004 conformance uses the reported severity"
bound = false
+++

found while working ~BC7S5KT: sync JSON diagnostics (O004) carry no severity field; conformance o004.rs fills info.
