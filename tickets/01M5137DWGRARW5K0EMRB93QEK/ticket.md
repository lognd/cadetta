+++
id = "01M5137DWGRARW5K0EMRB93QEK"
title = "Unknown [lint] precision kind should surface as a diagnostic, not only a log warning"
type = "task"
category = "in-progress"
priority = "low"
points = 1
reporter = "lognd"
created = "2026-10-16T00:54:08Z"
updated = "2026-10-16T01:43:49Z"
idempotency_key = "followup-76gmv4n-typo"
scope = ["crates/cadetta-cli/src/pipeline.rs"]

[[acceptance]]
text = "Given a [lint] precision key naming no builtin or package kind, when analysis runs, then a diagnostic is reported"
bound = false
+++

~76GMV4N made Lint::m_config_lenient skip unknown precision kinds with a tracing warning, so a typo (Lenth) is no longer caught at load. Surface it as a diagnostic or config error once the package kind table is known.
