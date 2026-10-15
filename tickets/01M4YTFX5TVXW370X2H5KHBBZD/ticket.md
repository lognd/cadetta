+++
id = "01M4YTFX5TVXW370X2H5KHBBZD"
title = "Remove dead emit_package and share text_params lookup (sync.rs/emit.rs)"
type = "chore"
category = "todo"
priority = "low"
points = 1
reporter = "lognd"
created = "2026-10-15T03:42:59Z"
updated = "2026-10-15T03:56:12Z"
idempotency_key = "emit-dedup-text-params"
scope = ["crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-cli/src/cmd/sync.rs"]

[[acceptance]]
text = "No dead-code warning for emit_package; one shared text_params lookup."
bound = true
+++

After ~FY0KQ76, emit_package has no callers (dead-code warning) and the text_params lookup is duplicated in sync.rs and emit.rs. Remove the dead fn and extract a shared helper. Do after ~WS81S98 lands (emit.rs in flight).
