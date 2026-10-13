+++
id = "01M4STAS70PCEHJ702R5GAW3SB"
title = "Conformance fixtures for config-cli"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:59Z"
updated = "2026-10-13T16:22:51Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for config-cli"
labels = ["creates:tests/conformance/config-cli/**"]
scope = ["tests/conformance/config-cli/**", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocks"
target = "01M4STCFQMGTXQAYPANNCHWEN4"

[[acceptance]]
text = "Given docs/spec/config-cli.md, when fixtures are written, then every normative rule and every diagnostic code in config-cli has a must-fire and a must-not-fire fixture under tests/conformance/config-cli/ (config-cli.md)"
bound = true

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = true

[[acceptance]]
text = "Given draft 1.4 revisions of the document, when fixtures are rewritten, then every rule added or changed in 1.4 has fixtures (docs/spec/README.md)"
bound = false
+++
