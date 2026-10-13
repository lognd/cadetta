+++
id = "01M4V436F7JRA8P832RH0EKNH2"
title = "Geometry diagnostics G001-G007 not emitted"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:13:51Z"
updated = "2026-10-13T17:13:51Z"
scope = ["crates/cadetta-lint/**"]
+++

found while working ~ZFM3633. The geometry conformance fixtures g001-* .. g007-* (tests/conformance/geometry) expect G001..G007 placement and datum-mismatch diagnostics; the binary emits none, and a.face-style datum references of placed parts report E002.
