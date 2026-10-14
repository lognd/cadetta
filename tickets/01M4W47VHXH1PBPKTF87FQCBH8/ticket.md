+++
id = "01M4W47VHXH1PBPKTF87FQCBH8"
title = "emit never skips a rewrite when only the header differs (same_modulo_header unused)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-14T02:35:38Z"
updated = "2026-10-14T03:13:47Z"
scope = ["crates/cadetta-cli/src/cmd/emit.rs"]

[[acceptance]]
text = "emit leaves a file untouched when it equals a fresh emit modulo the header line"
bound = true
+++

found while working ~HSDKA6V. adapters.md 2 says sync is idempotent: a file equal to a fresh emit modulo the header line is not rewritten. cadetta_adapter::builtin::sw_equations::same_modulo_header exists but cmd/emit.rs write_file writes unconditionally, so the emit-compare conformance fixture (adapters/sync-noop-excludes-header) passes only against the helper, not the CLI.
