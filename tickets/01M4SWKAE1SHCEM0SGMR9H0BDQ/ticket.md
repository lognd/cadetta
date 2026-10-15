+++
id = "01M4SWKAE1SHCEM0SGMR9H0BDQ"
title = "Lint family G001-G007 catalog entries"
type = "story"
category = "done"
outcome = "duplicate"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:43:36Z"
updated = "2026-10-15T10:39:43Z"
persona = "design engineer"
capability = "catalog position errors"
idempotency_key = "replan14:lint-g"
labels = ["creates:crates/cadetta-lint/src/rules/g.rs", "creates:tests/conformance/lints/G*/**"]
scope = ["crates/cadetta-lint/src/rules/g.rs", "tests/conformance/lints/G*/**"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQM289M7Q58CSS6F029F"

[[links]]
kind = "blocked-by"
target = "01M4SWFX82NWXV79EQTA5QX081"

[[links]]
kind = "blocked-by"
target = "01M4SWFXC2K6D3S93F3ER0GZZA"

[[links]]
kind = "blocked-by"
target = "01M4SWG6T58Q489WASV5DYGF7F"

[[links]]
kind = "blocked-by"
target = "01M4SWG70T71SNTZ2TWXZD1Y50"

[[acceptance]]
text = "Given G001-G007, when registered, then defaults match geometry.md 7: G001-G003 and G006 fixed errors, G004 error, G005 and G007 warnings (lints.md 3 G)"
bound = false

[[acceptance]]
text = "Given each code, when shipped, then must-fire and must-not-fire fixtures exist (lints.md 4)"
bound = false
+++
