+++
id = "01M4TA3CGRPW987SHWQ4E2F944"
title = "Add check.json output schema"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-15T01:45:05Z"
scope = ["docs/schemas"]

[[acceptance]]
text = "Given a clean or failing project, when check --format json runs, then the output validates against docs/schemas/check.json and each diagnostic against diag.json"
bound = true
+++

found while working ~E631Y52. absent.
