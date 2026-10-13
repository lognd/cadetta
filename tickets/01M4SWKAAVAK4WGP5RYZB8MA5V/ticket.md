+++
id = "01M4SWKAAVAK4WGP5RYZB8MA5V"
title = "Lint family E001-E011 resolution and evaluation"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:43:36Z"
updated = "2026-10-13T13:36:39Z"
persona = "design engineer"
capability = "get stable codes for every compile failure"
idempotency_key = "replan14:lint-e"
labels = ["creates:crates/cadetta-lint/src/rules/e.rs", "creates:tests/conformance/lints/E*/**"]
scope = ["crates/cadetta-lint/src/rules/e.rs", "tests/conformance/lints/E*/**", "crates/cadetta-lint/src/rules/mod.rs", "docs/spec/lints.md", "design/model.grmb"]

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
text = "Given E004 and E011, when names resolve through two use imports or Type.member reads a parameter-dependent member, then each fires with spans (lints.md 3 E; language.md 3.1)"
bound = true

[[acceptance]]
text = "Given E010, when mmc or lmc reads a tolerance with no role, then it fires (lints.md 3 E)"
bound = false

[[acceptance]]
text = "Given each of E001-E011, when its trigger is present, then the code fires as a fixed error, including E005 missing-package (an error, packages.md 1; lints.md 3 E)"
bound = false
+++
