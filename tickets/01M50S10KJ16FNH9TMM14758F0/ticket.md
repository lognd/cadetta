+++
id = "01M50S10KJ16FNH9TMM14758F0"
title = "Conformance fixture emitters/sw-equations/read-unparsed-line-kept-raw contradicts spec: IIF is parseable"
type = "bug"
category = "todo"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-15T21:55:52Z"
updated = "2026-10-15T22:07:57Z"
scope = ["tests/conformance/**"]
+++

The fixture expects IIF(...) to be kept raw, but emitters/sw-equations.md 3 says IIF parses to an expression (and a unit test asserts it). Change the fixture to a genuinely unparseable line, then remove its known-gaps.toml entry (currently pointed at ~G0V5V7D). Found while working ~G0V5V7D.
