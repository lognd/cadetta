+++
id = "01M4SWKAAVAK4WGP5RYZB8MA5V"
title = "Lint family E001-E011 resolution and evaluation"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:43:36Z"
updated = "2026-10-13T05:46:31Z"
persona = "design engineer"
capability = "get stable codes for every compile failure"
idempotency_key = "replan14:lint-e"
labels = ["creates:crates/cadetta-lint/src/rules/e.rs", "creates:tests/conformance/lints/E*/**"]
scope = ["crates/cadetta-lint/src/rules/e.rs", "tests/conformance/lints/E*/**"]

[[links]]
kind = "blocked-by"
target = "01M4SPWPCW1C0QC67GZG99EHTZ"

[[links]]
kind = "blocked-by"
target = "01M4SPWQM289M7Q58CSS6F029F"

[[links]]
kind = "blocked-by"
target = "01M4ST7ADMK0S7SRW6ETCS8ETC"

[[acceptance]]
text = "Given each of E001-E011, when its trigger is present, then the code fires as a fixed error except E005 which is a fixed warning (lints.md 3 E)"
bound = false

[[acceptance]]
text = "Given E004 and E011, when names resolve through two use imports or Type.member reads a parameter-dependent member, then each fires with spans (lints.md 3 E; language.md 3.1)"
bound = false

[[acceptance]]
text = "Given E010, when mmc or lmc reads a tolerance with no role, then it fires (lints.md 3 E)"
bound = false

[[acceptance]]
text = "Given the rules, when shipped, then must-fire and must-not-fire fixtures exist under tests/conformance/lints/E*/ (lints.md 4)"
bound = false
+++
