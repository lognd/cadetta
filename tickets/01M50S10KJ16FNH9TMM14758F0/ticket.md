+++
id = "01M50S10KJ16FNH9TMM14758F0"
title = "Conformance fixture emitters/sw-equations/read-unparsed-line-kept-raw contradicts spec: IIF is parseable"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-15T21:55:52Z"
updated = "2026-10-15T22:23:57Z"
scope = ["tests/conformance/**"]

[[acceptance]]
text = "Fixture read-unparsed-line-kept-raw uses an unknown function call (kept raw per sw-equations.md 3) and its known-gaps entry is removed"
bound = false
+++

The fixture expects IIF(...) to be kept raw, but emitters/sw-equations.md 3 says IIF parses to an expression (and a unit test asserts it). Change the fixture to a genuinely unparseable line, then remove its known-gaps.toml entry (currently pointed at ~G0V5V7D). Found while working ~G0V5V7D.
