+++
id = "01M4YTBH7QGW3Z87V77FY0KQ76"
title = "sync: manual-adapter emit fallback does not raise R014 for bound Text params"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T03:40:36Z"
updated = "2026-10-15T03:40:36Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs"]
+++

found while working ~T8PTE0A; sync.rs manual_adapter calls emit_package (empty Text set); should use emit_package_texts with LoweredPackage.text_params.
