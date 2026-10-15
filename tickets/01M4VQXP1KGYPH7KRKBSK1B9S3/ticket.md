+++
id = "01M4VQXP1KGYPH7KRKBSK1B9S3"
title = "Conformance runner observes active_alternate, json_diag_schema, results_order, r001_suppressed_by_force, unbound_params_not_written"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:22Z"
updated = "2026-10-15T08:52:15Z"
scope = ["crates/cadetta-cli/tests/**"]

[[acceptance]]
text = "Each key is observed and compared; the gap entries are removed."
bound = false
+++

found while working ~6BCVN48: these expect.json keys are not observed by verify.rs, so the entry fails 'does not observe'.
