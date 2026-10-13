+++
id = "01M4SWMCYW919R5JKSV2T1ZYBN"
title = "Geometry verification: frame and instance locators, transform comparison, datums = create"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-13T19:19:04Z"
persona = "design engineer"
capability = "drive sketch geometry from the design"
idempotency_key = "replan14:ad-geo"
labels = ["creates:crates/cadetta-adapter/src/geometry_bind.rs"]
scope = ["crates/cadetta-adapter/src/sync.rs", "crates/cadetta-adapter/src/locator.rs", "crates/cadetta-ir/src/observed.rs", "crates/cadetta-ir/src/schema.rs", "docs/schemas/cadetta.ir.1.json", "tests/conformance/adapters/**", "crates/cadetta-adapter/src/geometry_bind.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWK96MJF28HVDDJNPCBMPP"

[[links]]
kind = "blocked-by"
target = "01M4SWMCDEH4NF2SBVCRGE5VV9"

[[acceptance]]
text = "Given observed frames[] and instances[], when synced, then each bound datum's frame transform and each placement's composed frame are compared per adapters.md 3.4 (|design - read| <= 0.5 * 10^-precision) and a mismatch is R001 on the datum or placement path; placements are verify-only"
bound = false

[[acceptance]]
text = "Given datums = create and the create_frames feature, when synced, then missing datum coordinate systems are requested and composed transforms are pushed with lock synced records (adapters.md 3.4)"
bound = false

[[acceptance]]
text = "Given the instance locator kind and observed frames[] and instances[], when validated, then the IR schema and locator rules of ir.md 3 and adapters.md 3 hold"
bound = false

[[acceptance]]
text = "Given flip and exact 90deg rotations, when synced, then they are never pushed and are verified only through the transform comparison (adapters.md 3.4)"
bound = false
+++
