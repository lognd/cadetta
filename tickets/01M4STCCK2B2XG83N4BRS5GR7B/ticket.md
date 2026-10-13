+++
id = "01M4STCCK2B2XG83N4BRS5GR7B"
title = "Lints I001-I004 for interfaces and alternates"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:04:52Z"
updated = "2026-10-13T05:43:37Z"
persona = "design engineer"
capability = "be warned about unsound alternates and unevaluated roots"
idempotency_key = "replan13:Lints I001-I004 for interfaces and alternates"
labels = ["creates:crates/cadetta-lint/src/rules/interface.rs"]
scope = ["crates/cadetta-lint/src/rules/interface.rs"]

[[acceptance]]
text = "Given I001 to I004 in lints.md, when linted, then each fires on its defining condition with the severity and waivability in the catalog (lints.md I)"
bound = false

[[acceptance]]
text = "Given I001, when waived, then the waiver is refused (lints.md I)"
bound = false

[[acceptance]]
text = "Given a check that fails only under an approved alternate, when linted, then it is reported once as I002 (warning, configurable to error) on that alternate, while C001 and C002 are the primary's verdicts (language.md 3.2; lints.md 3 I)"
bound = false
+++
