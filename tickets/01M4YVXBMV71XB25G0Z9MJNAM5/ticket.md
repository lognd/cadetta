+++
id = "01M4YVXBMV71XB25G0Z9MJNAM5"
title = "S001 for an uncited user ots policy should report at component path"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T04:07:49Z"
updated = "2026-10-16T03:33:11Z"
scope = ["crates/cadetta-lint/src/rules/s.rs", "tests/conformance/stdlib/**", "crates/cadetta-cli/src/pipeline.rs", "docs/spec/lints.md"]
+++

found while working ~PGR0M75. stdlib/policy-iso-default-silent expects S001 warning at exactly acme.demo::Shop; check_s001 fires per member (acme.demo::Shop.bottom_clearance, .tap_lead_in) so the exact path does not match. Spec decision needed: lints.md:367 says 'ots value with no citation on it or its component'; decide component-level vs per-member reporting.
