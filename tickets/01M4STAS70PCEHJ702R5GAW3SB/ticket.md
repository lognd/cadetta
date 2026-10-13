+++
id = "01M4STAS70PCEHJ702R5GAW3SB"
title = "Conformance fixtures for config-cli"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:59Z"
updated = "2026-10-13T05:03:59Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for config-cli"
labels = ["creates:tests/conformance/config-cli/**"]
scope = ["tests/conformance/config-cli/**"]

[[acceptance]]
text = "Given docs/spec/config-cli.md, when fixtures are written, then every normative rule and every diagnostic code in config-cli has a must-fire and a must-not-fire fixture under tests/conformance/config-cli/ (config-cli.md)"
bound = false

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = false
+++
