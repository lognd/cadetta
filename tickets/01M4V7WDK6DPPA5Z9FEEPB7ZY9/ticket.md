+++
id = "01M4V7WDK6DPPA5Z9FEEPB7ZY9"
title = "json/csv: --assume-unit precedence, whole-number rendering, O004 open obligation without drive"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T18:20:03Z"
updated = "2026-10-15T04:12:07Z"
scope = ["crates/cadetta-adapter/**"]

[[acceptance]]
text = "assume-unit wins over the locator native_unit, then the adapter native unit, when reading a unitless value"
bound = false
+++

found while working ~A4RA9PQ. See json.md 4, csv.md 4, adapters.md 2.
