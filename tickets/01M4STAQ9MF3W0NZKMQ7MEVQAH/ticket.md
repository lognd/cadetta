+++
id = "01M4STAQ9MF3W0NZKMQ7MEVQAH"
title = "Conformance fixtures for language"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:57Z"
updated = "2026-10-13T16:25:45Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for language"
labels = ["creates:tests/conformance/language/**"]
scope = ["tests/conformance/language/**", "docs/spec/language.md", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocks"
target = "01M4STCDGQMN86JQJXKAQ2FX25"

[[acceptance]]
text = "Given docs/spec/language.md, when fixtures are written, then every normative rule and every diagnostic code in language has a must-fire and a must-not-fire fixture under tests/conformance/language/ (language.md)"
bound = true

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = true
+++
