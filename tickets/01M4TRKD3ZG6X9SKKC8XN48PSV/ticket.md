+++
id = "01M4TRKD3ZG6X9SKKC8XN48PSV"
title = "Lint config in check: precision, d001_min_digits, common_values, table cells"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-13T13:52:59Z"
updated = "2026-10-15T01:50:51Z"
scope = ["crates/cadetta-cli/src/pipeline.rs,crates/cadetta-cli/src/cmd/check.rs,crates/cadetta-cli/src/config.rs,docs/spec/lints.md"]

[[acceptance]]
text = "[lint] precision, d001_min_digits and common_values change M001/D001/D002 results in cadetta check; table cells feed D002"
bound = true
+++

Follow-up of ~MC5WD33: the driver runs D001/D002/M001 with DConfig/MConfig defaults; [lint] settings and table cells are not threaded through.
