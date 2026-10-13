+++
id = "01M4V5MP7JHJVCCP08F6NZV18E"
title = "csv.md: read behaviour for unknown names and quoted hash fields unspecified"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:40:53Z"
updated = "2026-10-13T17:40:53Z"
scope = ["docs/spec/emitters/csv.md"]
+++

found while working ~9JX3J6F. csv.md 3 does not say what read does with a row whose name has no binding, nor whether a quoted multi-line field whose continuation line begins with # is skipped ('every line that begins with #'). Specify both; add fixtures.
