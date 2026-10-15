+++
id = "01M4V43BDMB7CKR8W79S5DC5W0"
title = "Geometry datum declaration checks not enforced"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:13:56Z"
updated = "2026-10-15T03:19:54Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-lint/src/rules/interface.rs", "crates/cadetta-geometry/src/datum.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/src/cmd/check.rs", "tests/conformance/known-gaps.toml", "changelog.d/01M4V43BDMB7CKR8W79S5DC5W0.fixed.md", "Cargo.lock"]

[[acceptance]]
text = "E002 fires for a datum based on a later or unknown datum"
bound = true

[[acceptance]]
text = "E006 fires for flip(z)"
bound = true

[[acceptance]]
text = "I001 fires only when an implementer omits a declared interface datum"
bound = false

[[acceptance]]
text = "C001/C002 fire for reversed or undecided datum-bound rotations"
bound = false

[[acceptance]]
text = "S001 fires for uncited ots datum offsets"
bound = false
+++

found while working ~ZFM3633. Fixtures datum-bound-*, datum-built-from-earlier, datum-flip-z, datum-ots-needs-citation, datum-unknown-of-placed-part, datum-zero-count-pattern and i001-interface-datum-omitted expect C001/C002 on datum bounds, E002/E006 decisions (geometry.md conformance decisions), S001 and I001; the binary emits none of them.
