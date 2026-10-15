+++
id = "01M4VQXPJ7HD3BRK9WXNDGXJQ1"
title = "Solver S005 failures, Q001 outputs and variants flags not implemented in solve"
type = "bug"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:22Z"
updated = "2026-10-15T04:41:29Z"
scope = ["crates/cadetta-solve/**"]

[[acceptance]]
text = "solvers fixtures pass."
bound = false

[[acceptance]]
text = "solve --format json reports a diagnostics list for failed solvers (S005, Q001)"
bound = true
+++

found while working ~6BCVN48: solvers/s005-*, q001-output-*, transport-*, result-file-beside-script, variants-* fixtures (27) fail; see also ~QDVP8JQ.
