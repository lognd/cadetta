+++
id = "01M4SPWQ3VSX2CA3TZY4KQ2DZW"
title = "IR data model types"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T06:04:57Z"
persona = "cadetta contributor"
capability = "define Project, Component, Param, Value, Fit, Check, Waiver, Binding and Provenance"
outcome_text = "every lint and adapter shares one model"
labels = ["creates:crates/cadetta-ir/src/model.rs"]
scope = ["crates/cadetta-ir/src/model.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWQ6VDD0177RREW273PYG"

[[links]]
kind = "blocks"
target = "01M4SPWQED1EN19NWQJE5NT0J4"

[[links]]
kind = "blocks"
target = "01M4ST7B09TGN6VSXJBZ8FDVCN"

[[acceptance]]
text = "Given design and observed flavours, when constructed, then both use the same Project type with flavour set (ir.md 1)"
bound = false

[[acceptance]]
text = "Given the entity list in ir.md 3, when modelled, then Project, Component, Member, Value, Exact, Inexact, Affine, Bound, Part, Fit, Check, Solver, Citation, Provenance and Binding exist with the listed fields"
bound = false

[[acceptance]]
text = "Given a Value, when built, then it holds nominal (Exact or Inexact), angle, optional Affine tol, conservative, optional designed and authored (ir.md 3)"
bound = false

[[acceptance]]
text = 'Given a Member, when built, then it carries the analysis flag, deps, optional variants keyed "slot=alternate", digest and closure (ir.md 3)'
bound = false

[[acceptance]]
text = "Given a Binding, when built, then it holds a list of locators each with kind (global|dimension|property), mode (value|ref) and native_unit (ir.md 3, adapters.md 3)"
bound = false

[[acceptance]]
text = "Given a Check, when built, then it holds verdicts for the primary and per alternate and waived_for (ir.md 3)"
bound = false

[[acceptance]]
text = "Given a Value, when lowered, then nominal is always the designed value and drawn? exists only when a process rule applied, with no designed field (ir.md 3)"
bound = false

[[acceptance]]
text = "Given a Locator, when serialized, then it carries kind global|dimension|property|frame, mode, native_unit, ref_to, for, config, sign and component (ir.md 3)"
bound = false

[[acceptance]]
text = "Given Project, when built, then it holds package {name, version}, a sorted modules[] of every module path and dependencies[] (name, version, source, digest), and items in src/lib.cdt sit at the package root (ir.md 3, 4; packages.md 2)"
bound = false
+++
