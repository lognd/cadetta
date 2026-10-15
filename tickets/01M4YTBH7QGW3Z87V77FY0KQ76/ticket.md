+++
id = "01M4YTBH7QGW3Z87V77FY0KQ76"
title = "sync: manual-adapter emit fallback does not raise R014 for bound Text params"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T03:40:36Z"
updated = "2026-10-15T03:42:44Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-cli/tests/r014_sync_manual_adapter.rs", "changelog.d/01M4YTBH7QGW3Z87V77FY0KQ76.fixed.md"]

[[acceptance]]
text = "sync manual-adapter fallback raises R014 for a bound Text param and withholds the file"
bound = true

[[acceptance]]
text = "sync manual fallback emits cleanly when the Text param is filtered out"
bound = true
+++

found while working ~T8PTE0A; sync.rs manual_adapter calls emit_package (empty Text set); should use emit_package_texts with LoweredPackage.text_params.
