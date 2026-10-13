+++
id = "01M4STCFQMGTXQAYPANNCHWEN4"
title = "Conformance run for config-cli"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T05:04:55Z"
updated = "2026-10-13T05:04:55Z"
persona = "maintainer"
capability = "know the implementation matches the spec"
idempotency_key = "replan13:Conformance run for config-cli"
labels = ["creates:tests/conformance_config_cli.rs"]
scope = ["tests/conformance_config_cli.rs"]

[[acceptance]]
text = "Given tests/conformance/config-cli/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/config-cli.md)"
bound = false
+++
