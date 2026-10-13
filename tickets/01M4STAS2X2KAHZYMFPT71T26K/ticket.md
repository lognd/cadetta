+++
id = "01M4STAS2X2KAHZYMFPT71T26K"
title = "Conformance fixtures for lints"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:59Z"
updated = "2026-10-13T16:51:52Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for lints"
labels = ["creates:tests/conformance/lints/**"]
scope = ["tests/conformance/lints/**", "docs/spec/lints.md", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocks"
target = "01M4STCFMHY2RXD7WFF3Q9FRJD"

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = true

[[acceptance]]
text = "Given docs/spec/lints.md, when fixtures are written, then the Q, N, M, E010, S002, S007, W and U001 codes each have a must-fire and a must-not-fire fixture under tests/conformance/lints/ (lints.md 3); the remaining catalog codes and 1.4 rules are tracked in ~ZKZTK5Z"
bound = true
+++
