+++
id = "01M54RWWNC7TS11SQGSGB9FXRG"
title = "cadetta CLI prints stdlib E002/EVAL warnings on stderr when the add-in runs it"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T11:10:34Z"
updated = "2026-10-17T11:28:55Z"
scope = ["crates/**"]

[[acceptance]]
text = "Given the CLI resolves or evaluates std, when a returned E002/EVAL error is produced, then nothing is logged at warn on stderr"
bound = true
+++

Found while working ~4FG4VVR: noisy stderr from the stdlib during live runs. Needs triage.
