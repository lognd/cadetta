+++
id = "01M4SPWQ3VSX2CA3TZY4KQ2DZW"
title = "IR data model types"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T04:03:59Z"
persona = "cadetta contributor"
capability = "define Project, Component, Param, Value, Fit, Check, Waiver, Binding and Provenance"
outcome_text = "every lint and adapter shares one model"
labels = ["creates:crates/cadetta-ir/src/model.rs"]
scope = ["crates/cadetta-ir/src/model.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWQ6VDD0177RREW273PYG"

[[acceptance]]
text = "Given design and observed flavours, when constructed, then both use the same Project type with flavour set"
bound = false

[[acceptance]]
text = "Given a Value, when built, then it holds nominal, optional lower/upper rationals, exact and authored_unit"
bound = false
+++
