+++
id = "01M4YR2T1ETTN2BE6X7G49G4TT"
title = "R012 reserved: decide whether to retire code and fixture or keep dormant"
type = "chore"
category = "in-progress"
priority = "low"
points = 1
reporter = "lognd"
created = "2026-10-15T03:00:53Z"
updated = "2026-10-16T00:52:23Z"
idempotency_key = "r012-reserved-cleanup"
scope = ["crates/cadetta-adapter/src/locator.rs", "crates/cadetta-lint/src/rules/r_adapter.rs", "tests/conformance/**", "changelog.d/**"]

[[acceptance]]
text = "R012 code and fixture are consistent with the reserved wording in lints.md"
bound = false
+++

Follow-up to ~V9JHZEN: specs now call R012 reserved/unreachable from source, but rule code and fixture adapters/r012-star-binding-differing-instances (gap ~6ES67RW) remain. Remove or keep dormant.
