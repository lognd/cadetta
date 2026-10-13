+++
id = "01M4SPWTA1PQV15S63741V30XT"
title = "cadetta emit and import"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:56Z"
updated = "2026-10-13T14:36:59Z"
persona = "design engineer"
capability = "emit to configured adapters and import native files"
outcome_text = "nobody retypes values and existing designs are adoptable"
labels = ["creates:crates/cadetta-cli/src/cmd/{emit,import}.rs"]
scope = ["crates/cadetta-cli/src/cmd/{emit,import}.rs", "crates/cadetta-cli/src/{lib.rs,cmd/mod.rs}", "crates/cadetta-adapter/src/import/propose.rs", "docs/spec/config-cli.md"]

[[links]]
kind = "blocks"
target = "01M4SPWTDD069AXXQBXD4T6ATZ"

[[acceptance]]
text = "Given a configured sw-equations adapter, when `cadetta emit` runs, then build/solidworks/equations.txt is written"
bound = true

[[acceptance]]
text = "Given a native file, when `cadetta import freecad f.FCStd` runs, then bindings and a scaffolded .cdt are proposed, not written until accepted"
bound = true

[[acceptance]]
text = "Given emit and import flags, when run, then exit codes and outputs follow config-cli.md 3 (Q004, R013 give exit 1)"
bound = true
+++
