+++
id = "01M4SPWTA1PQV15S63741V30XT"
title = "cadetta emit and import"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:56Z"
updated = "2026-10-13T04:03:56Z"
persona = "design engineer"
capability = "emit to configured adapters and import native files"
outcome_text = "nobody retypes values and existing designs are adoptable"
labels = ["creates:crates/cadetta-cli/src/cmd/{emit,import}.rs"]
scope = ["crates/cadetta-cli/src/cmd/{emit,import}.rs"]

[[acceptance]]
text = "Given a configured sw-equations adapter, when `cadetta emit` runs, then build/solidworks/equations.txt is written"
bound = false

[[acceptance]]
text = "Given a native file, when `cadetta import freecad f.FCStd` runs, then bindings and a scaffolded .cdt are proposed, not written until accepted"
bound = false
+++
