+++
id = "01M4SPWQM289M7Q58CSS6F029F"
title = "Lint engine, severity gating and waiver application"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T04:04:01Z"
persona = "design engineer"
capability = "configure severities, fail_on and waive diagnostics"
outcome_text = "CI fails only on what I decide matters"
labels = ["creates:crates/cadetta-lint/src/engine.rs"]
scope = ["crates/cadetta-lint/src/engine.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWQPYYT8Y5N3P5ZSG20Z9"

[[links]]
kind = "blocks"
target = "01M4SPWQT4ZPZ7YT5W1Q41GK55"

[[links]]
kind = "blocks"
target = "01M4SPWQWV4J8DF3B1RF4PFYCT"

[[links]]
kind = "blocks"
target = "01M4SPWQZF9T7HA75ESE6D2C96"

[[links]]
kind = "blocks"
target = "01M4SPWR26H443049B6E0Q9EBH"

[[acceptance]]
text = 'Given [lint] severity N002="info", when linted, then N002 is info'
bound = false

[[acceptance]]
text = "Given a fixed code with a severity override, when loaded, then the override is ignored"
bound = false

[[acceptance]]
text = "Given `waive M001 on acme.rover::Axle.chamfer`, when linted, then M001 on that path is suppressed"
bound = false
+++
