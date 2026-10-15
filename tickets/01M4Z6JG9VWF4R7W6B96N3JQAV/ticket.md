+++
id = "01M4Z6JG9VWF4R7W6B96N3JQAV"
title = "Adapter host gaps found by protocol conformance: platforms, precision, push-status shape, R009 shutdown, builtin capabilities"
type = "bug"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-15T07:14:07Z"
updated = "2026-10-15T11:57:43Z"
scope = ["crates/cadetta-adapter/**", "tests/conformance/known-gaps.toml", "crates/cadetta-cli/tests/conformance/protocol.rs", "docs/spec/adapters.md"]

[[acceptance]]
text = "The host checks initialize platforms (R008), uses initialize precision, rejects summary-only push results with R008, sends shutdown after an R009 refusal, and declares capabilities for built-in adapters; the seven known-gaps entries for these fixtures are removed."
bound = true
+++

found while working ~Q6AKDVQ. Executor runs; these fixtures fail honestly: adapters/protocol-platforms-checked, protocol-precision-from-initialize, protocol-push-status-per-locator (summary-only push result must be R008), protocol-open-dirty-r009 (no shutdown after R009 refusal in DriveSession::open), emitters/{csv,json,sw-equations}/capabilities-emit-read-no-drive (builtin adapters declare no capability set).
