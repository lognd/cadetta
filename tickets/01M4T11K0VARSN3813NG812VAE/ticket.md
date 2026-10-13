+++
id = "01M4T11K0VARSN3813NG812VAE"
title = "Wire short and github formats into the CLI output"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T07:01:18Z"
updated = "2026-10-13T07:02:31Z"
scope = ["crates/cadetta-cli/src/output.rs"]

[[acceptance]]
text = "Format::Short and Format::Github output is produced by cadetta_lint::render::short and ::github (no duplicate renderer in output.rs)"
bound = false

[[acceptance]]
text = "Waived diagnostics are omitted from short and github output"
bound = false
+++
