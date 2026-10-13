+++
id = "01M4SPWQM289M7Q58CSS6F029F"
title = "Lint engine, severity gating and waiver application"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T09:01:55Z"
persona = "design engineer"
capability = "configure severities, fail_on and waive diagnostics"
outcome_text = "CI fails only on what I decide matters"
labels = ["creates:crates/cadetta-lint/src/engine.rs"]
scope = ["crates/cadetta-lint/src/engine.rs", "crates/cadetta-lint/src/lib.rs", "docs/spec/lints.md"]

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

[[links]]
kind = "blocks"
target = "01M4SPWR4SAWTESPSK2DP99M5S"

[[links]]
kind = "blocks"
target = "01M4SPWR7SG2GDJ254566CDH0B"

[[links]]
kind = "blocks"
target = "01M4SPWRAWXJTWT4G08578RNQM"

[[links]]
kind = "blocks"
target = "01M4SPWRDX98PHVNHPD14HYY8D"

[[links]]
kind = "blocks"
target = "01M4SPWT3FP4SN4RTCHE631Y52"

[[links]]
kind = "blocks"
target = "01M4STCCK2B2XG83N4BRS5GR7B"

[[links]]
kind = "blocks"
target = "01M4STCCQD6C55WEN7HFEBPEYV"

[[links]]
kind = "blocks"
target = "01M4STCCVN188CD3VRZC374QTM"

[[links]]
kind = "blocks"
target = "01M4STCCZ58C875BTMQ9E54BMT"

[[acceptance]]
text = 'Given [lint] severity N002="info", when linted, then N002 is info'
bound = true

[[acceptance]]
text = "Given a fixed code with a severity override, when loaded, then the override is ignored"
bound = true

[[acceptance]]
text = "Given `waive M001 on acme.rover::Axle.chamfer`, when linted, then M001 on that path is suppressed"
bound = true

[[acceptance]]
text = 'Given `#[allow(M001, "vendor drawing 1234")]` on a member, when linted, then M001 is suppressed there without expiry, and the same without a because string is rejected (lints.md 2)'
bound = true

[[acceptance]]
text = "Given a waiver glob and `for alt NAME`, when linted, then it applies only to that alternate variant (lints.md 2, language.md 3.2)"
bound = true

[[acceptance]]
text = "Given a waiver, when created, then it binds the closure digests of its targets (lints.md 2, ir.md 4)"
bound = true

[[acceptance]]
text = "Given `cadetta explain CODE` data, when queried, then each code exposes name, default severity, fixed flag and meaning from one catalog table (lints.md 3)"
bound = true

[[acceptance]]
text = "Given a fixed code, required_for floor, [lint] severity and waive/allow, when diagnostics are gated, then precedence is exactly lints.md 1 and fail_on = never makes diagnostics informational (lints.md 1; config-cli.md 2)"
bound = true
+++
