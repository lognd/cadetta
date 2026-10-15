+++
id = "01M4VQXP1KGYPH7KRKBSK1B9S3"
title = "Conformance runner observes json_diag_schema, results_order, r001_suppressed_by_force, unbound_params_not_written, runs"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:22Z"
updated = "2026-10-15T09:12:29Z"
scope = ["crates/cadetta-cli/tests/**", "tests/conformance/README.md"]

[[acceptance]]
text = "results_order, r001_suppressed_by_force, unbound_params_not_written and runs are observed and compared; their gap entries are removed"
bound = true

[[acceptance]]
text = "json_diag_schema stays observed and compared"
bound = true
+++

found while working ~6BCVN48: these expect.json keys are not observed by verify.rs, so the entry fails 'does not observe'.
