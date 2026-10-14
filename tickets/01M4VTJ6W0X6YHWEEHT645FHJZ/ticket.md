+++
id = "01M4VTJ6W0X6YHWEEHT645FHJZ"
title = "Feed composed geometry transforms and observed frames into sync (geometry verification glue)"
type = "story"
category = "in-progress"
priority = "high"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T23:46:31Z"
updated = "2026-10-14T01:43:52Z"
persona = "design engineer"
capability = "have sync verify datums and placements against CAD"
outcome_text = "a part that sits elsewhere in CAD is R001"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-adapter/src/geometry_bind.rs"]

[[links]]
kind = "blocked-by"
target = "01M4VQXPDA4TEKE455FNYB5TXC"

[[acceptance]]
text = "Given a design with bound datums and placements and an adapter reporting frames and instances, when cadetta sync runs, then composed transforms from the evaluated IR are converted to Mat34 and compared through geometry_bind (R001 on mismatch), and datums = create pushes frames (adapters.md 3.4)"
bound = false
+++

Left by ~2T1ZYBN: comparison, planning and schema exist; no CLI code feeds them. Needs ~NYB5TXC (geometry in the evaluated IR).
