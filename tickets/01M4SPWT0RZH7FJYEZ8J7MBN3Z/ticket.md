+++
id = "01M4SPWT0RZH7FJYEZ8J7MBN3Z"
title = "cadetta init and fmt"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:56Z"
updated = "2026-10-13T04:03:56Z"
persona = "design engineer"
capability = "run `cadetta init` and `cadetta fmt [--check]`"
outcome_text = "I can start a project and keep layout canonical"
labels = ["creates:crates/cadetta-cli/src/cmd/{init,fmt}.rs"]
scope = ["crates/cadetta-cli/src/cmd/{init,fmt}.rs"]

[[acceptance]]
text = "Given an existing project, when init repeats, then nothing is overwritten"
bound = false

[[acceptance]]
text = "Given unformatted source, when `fmt --check` runs, then exit code is 1"
bound = false
+++
