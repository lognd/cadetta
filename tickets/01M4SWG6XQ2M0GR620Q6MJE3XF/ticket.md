+++
id = "01M4SWG6XQ2M0GR620Q6MJE3XF"
title = "ScrewJoint at/onto: aligned with fixed and floating allowances"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:54Z"
updated = "2026-10-14T03:48:40Z"
persona = "design engineer"
capability = "screw here and prove alignment"
idempotency_key = "replan14:geo-screwpos"
labels = ["creates:std/src/fasteners/screwjoint_position.cdt"]
scope = ["std/src/fasteners/screwjoint_position.cdt", "std/**", "crates/cadetta-sema/**", "tests/conformance/**", "docs/spec/stdlib.md"]

[[links]]
kind = "blocked-by"
target = "01M4STAPFPQ9GV8XJB4XCK8EGP"

[[links]]
kind = "blocked-by"
target = "01M4SWG6T58Q489WASV5DYGF7F"

[[links]]
kind = "blocked-by"
target = "01M4TRBJ7D2JTGX6XX94BJ1ZGM"

[[links]]
kind = "blocked-by"
target = "01M4W3YJCH948JJQ3HY646D1T7"

[[links]]
kind = "blocked-by"
target = "01M4W5HM6SAY12P1J3R3G80Q7F"

[[acceptance]]
text = "Given at and onto of equal count, when the joint is checked, then aligned uses (min(clearance_hole) - max(major)) / 2 from the clearance class nominal, unaffected by the user's hole tolerance (geometry.md 5)"
bound = true

[[acceptance]]
text = "Given nut = true with onto a second clearance pattern, when checked, then the floating allowance is the sum of both halves (geometry.md 5; stdlib.md 3)"
bound = true

[[acceptance]]
text = "Given the M5 medium example, when checked, then the fixed allowance is 0.25mm (geometry.md 1)"
bound = true
+++
