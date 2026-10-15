+++
id = "01M4Y3S1M0V9W3JCDBTHPED6RY"
title = "Route sync execute through push_atomic and matched_locators"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-14T21:06:01Z"
updated = "2026-10-15T09:15:45Z"
idempotency_key = "cadetta:sync-wire-atomic"
scope = ["crates/cadetta-adapter/src/sync.rs", "crates/cadetta-adapter/src/geometry_bind.rs", "crates/cadetta-cli/tests/conformance/sync.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWMC85Q2YCHQ3GS4MREKBP"

[[acceptance]]
text = "Given a regen failure (R004) during sync execute, when the push fails, then the pre-push snapshot is restored via push_atomic and no CAD file stays half-written"
bound = false

[[acceptance]]
text = "Given a read-back that differs from the pushed value, when sync records results, then only locators from matched_locators get a synced record and the inline matcher is removed"
bound = false
+++

Follow-up to ~4MREKBP. atomic::push_atomic and atomic::matched_locators exist but sync::execute still pushes non-atomically and matches read-backs inline (sync.rs near line 341). Route Backend::push through push_atomic so an R004 regen failure restores the snapshot, and replace the inline read-back matcher with matched_locators. Update docs/spec/adapters.md 2.3 if behaviour text changes.
