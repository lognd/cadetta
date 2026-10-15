+++
id = "01M4YRPNTCYCGNZ5VP5PDPCRB2"
title = "sema: package:: expression paths do not raise E016 / N004 (was-path)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T03:11:44Z"
updated = "2026-10-15T03:36:56Z"
scope = ["crates/cadetta-sema/**"]

[[acceptance]]
text = "A package::-qualified expression path to an invisible item raises E016 (fixture e016-fully-qualified-path-checks-visibility)"
bound = false

[[acceptance]]
text = "A package::-qualified expression path through a was rename raises N004 (fixture n004-reference-to-was-path)"
bound = false
+++

Follow-up of ~KRVD0V6 (parser now accepts package::/super:: in expressions). Fixtures packages/e016-fully-qualified-path-checks-visibility and n004-reference-to-was-path still do not fire E016/N004 (sema).
