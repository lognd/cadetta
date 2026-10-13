+++
id = "01M4V5MP7JHJVCCP08F6NZV18E"
title = "csv.md: read behaviour for unknown names and quoted hash fields unspecified"
type = "bug"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-13T17:40:53Z"
updated = "2026-10-13T18:16:01Z"
scope = ["docs/spec/emitters/csv.md", "docs/spec/emitters/json.md", "docs/spec/lints.md", "docs/spec/adapters.md", "docs/spec/config-cli.md", "docs/spec/README.md", "crates/cadetta-lint/src/engine.rs", "tests/conformance/emitters/csv/**", "tests/conformance/emitters/json/**"]

[[acceptance]]
text = "Given a csv or json file row whose name has no binding, when read runs, then the spec says it is the error R016 naming the row, no observed IR is produced and exit is 1, and a fixture pair covers it for each format"
bound = false

[[acceptance]]
text = "Given a csv quoted field spanning lines whose continuation line begins with #, when read runs, then the spec says only a record start is tested for #, the continuation is data, and a fixture pair covers it"
bound = false
+++

found while working ~9JX3J6F. csv.md 3 does not say what read does with a row whose name has no binding, nor whether a quoted multi-line field whose continuation line begins with # is skipped ('every line that begins with #'). Specify both; add fixtures.
