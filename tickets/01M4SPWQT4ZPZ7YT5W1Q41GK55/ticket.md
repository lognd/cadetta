+++
id = "01M4SPWQT4ZPZ7YT5W1Q41GK55"
title = "Lint family Naming rules N001-N006"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T13:01:31Z"
persona = "design engineer"
capability = "get lints for the N family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/n.rs", "creates:tests/conformance/lints/N*/**"]
scope = ["crates/cadetta-lint/src/rules/n.rs", "tests/conformance/lints/N*/**"]

[[acceptance]]
text = "Given parameter `Axle.axle_length`, when linted, then N002 is reported"
bound = false

[[acceptance]]
text = "Given `diameter_mm`, when linted, then N003 is reported"
bound = false

[[acceptance]]
text = "Given component `my_axle`, when linted, then N001 is reported"
bound = false

[[acceptance]]
text = "Given a name containing `__`, when linted, then N005 is an error and cannot be waived (lints.md N)"
bound = false

[[acceptance]]
text = "Given `Type.member` in an assembly that has a `part` of that type, when linted, then N006 suggests the instance (language.md 3.1)"
bound = false

[[acceptance]]
text = "Given `Iso4762M5` and `M2_5`, when linted, then N001 does not fire (language.md 4)"
bound = false
+++
