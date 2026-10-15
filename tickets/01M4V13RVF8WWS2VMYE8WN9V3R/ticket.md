+++
id = "01M4V13RVF8WWS2VMYE8WN9V3R"
title = "language: sema gaps (E002 use target, E003 diagnostic, E006 arg name)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:21:38Z"
updated = "2026-10-15T02:57:05Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/src/project.rs", "crates/cadetta-cli/tests/sema_gaps.rs", "docs/**", "changelog.d/01M4V13RVF8WWS2VMYE8WN9V3R.*", "tests/conformance/language/**", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "An unresolved use target is an E002 diagnostic even when the name is unused (e002-use-target)"
bound = true

[[acceptance]]
text = "A duplicate type name is an E003 diagnostic, not a raw load failure (e003-duplicate-type)"
bound = false

[[acceptance]]
text = "An argument name matching no parameter, or too many positional arguments, is a single E006 (e006-argument-mismatch)"
bound = false
+++

found while working ~7MEVQAH. Fixtures under tests/conformance/language/ are spec-true; these sides disagree with the binary:  e002-use-target, e003-duplicate-type (exits 2 raw, no diagnostic), e006-argument-mismatch, e016-private-item, e017-private-in-public, n001-case, n004-was, n006-type-read-beside-instance; parser emits two E001 for one bad waive date token (cascade).
