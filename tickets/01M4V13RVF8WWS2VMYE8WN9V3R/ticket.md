+++
id = "01M4V13RVF8WWS2VMYE8WN9V3R"
title = "language: sema gaps (E002 use target, E003 diagnostic, E006 arg name, E016/E017 privacy, N001, N004, N006)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:21:38Z"
updated = "2026-10-15T02:56:55Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/src/project.rs", "crates/cadetta-cli/tests/sema_gaps.rs", "docs/**", "changelog.d/01M4V13RVF8WWS2VMYE8WN9V3R.*", "tests/conformance/language/**", "tests/conformance/known-gaps.toml"]
+++

found while working ~7MEVQAH. Fixtures under tests/conformance/language/ are spec-true; these sides disagree with the binary:  e002-use-target, e003-duplicate-type (exits 2 raw, no diagnostic), e006-argument-mismatch, e016-private-item, e017-private-in-public, n001-case, n004-was, n006-type-read-beside-instance; parser emits two E001 for one bad waive date token (cascade).
