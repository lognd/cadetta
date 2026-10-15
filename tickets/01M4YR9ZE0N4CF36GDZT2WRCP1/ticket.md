+++
id = "01M4YR9ZE0N4CF36GDZT2WRCP1"
title = "quantities: exact trig/zero/c002 fixtures miss expected Q006 info (5 fixtures)"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T03:04:48Z"
updated = "2026-10-15T03:39:59Z"
scope = ["crates/cadetta-cli/tests/conformance/exec.rs", "tests/conformance/known-gaps.toml", "tests/conformance/quantities/*", "changelog.d/01M4YR9ZE0N4CF36GDZT2WRCP1.*", "crates/cadetta-cli/tests/conformance.rs"]

[[acceptance]]
text = "The five quantities fixtures (c002-inexact-boundary, exact-zero-normalised, trig-exact-atan2, trig-exact-sin, trig-exact-tan) pass the conformance corpus with no known-gaps entries"
bound = true
+++

Found while working ~X1PF271. quantities/{c002-inexact-boundary,exact-zero-normalised,trig-exact-atan2,trig-exact-sin,trig-exact-tan} must-not-fire: missing Q006 (info).
