+++
id = "01M4SPWPAJ4ZGEGGVF63ZY806Z"
title = "Canonical formatter"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWG5A0FJNPKRPDDCJQZN8"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T06:45:19Z"
persona = "design engineer"
capability = "run a formatter that produces one canonical layout"
outcome_text = "layout never causes review noise or IR changes"
labels = ["creates:crates/cadetta-syntax/src/format.rs"]
scope = ["crates/cadetta-syntax/src/format.rs", "crates/cadetta-syntax/src/lib.rs", "docs/spec/language.md"]

[[links]]
kind = "blocks"
target = "01M4SPWQED1EN19NWQJE5NT0J4"

[[links]]
kind = "blocks"
target = "01M4SPWT0RZH7FJYEZ8J7MBN3Z"

[[acceptance]]
text = "Given any valid file, when formatted twice, then output is identical (idempotent)"
bound = false

[[acceptance]]
text = "Given `=` signs in a block, when formatted, then they are aligned with two-space indent and authored units preserved"
bound = false

[[acceptance]]
text = "Given formatted source, when lowered, then the IR digest is unchanged"
bound = false

[[acceptance]]
text = "Given trailing attributes and comments on consecutive members, when formatted, then they align into columns (language.md 7)"
bound = false

[[acceptance]]
text = "Given `cadetta fmt --check` on unformatted source, when run, then it reports the files and changes none (config-cli.md 2)"
bound = false
+++
