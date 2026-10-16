+++
id = "01M4V13SWX6K7SC1E6T0N41BNX"
title = "language: table lookup, fit and variant pipeline unimplemented (E007 E008 F001-F004 I001-I004 T001 G001 P001 U002)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-13T16:21:45Z"
updated = "2026-10-16T20:13:10Z"
scope = ["crates/**"]
+++

found while working ~7MEVQAH. Fixtures under tests/conformance/language/ are spec-true; these sides disagree with the binary:  e007-lookup-miss, e008-*, f001-*, f002-*, f003, f004, fit-clearance-member, g001-array-plus-point, i001-*, i002-*, i003-*, i004-root-not-evaluated, p001-missing-role, t001-not-in-table, process-drawn-value, u002-unused-private.
