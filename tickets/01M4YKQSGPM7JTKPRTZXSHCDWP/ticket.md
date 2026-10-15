+++
id = "01M4YKQSGPM7JTKPRTZXSHCDWP"
title = "IR Project model and schema lack requires[] and Project fields for interfaces/tables/waivers"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T01:44:58Z"
updated = "2026-10-15T01:46:39Z"
scope = ["crates/cadetta-ir/**", "docs/spec/ir.md", "changelog.d/01M4YKQSGPM7JTKPRTZXSHCDWP.*"]

[[acceptance]]
text = "Project carries interfaces, tables and waivers and serializes them when non-empty (omitted when empty)"
bound = false

[[acceptance]]
text = "Project with those collections validates against the IR JSON Schema"
bound = false
+++

found while working ~RC9QR10: schema now allows optional Project interfaces/tables/waivers but the Project struct does not carry or serialize them, and no Require entity shape exists in ir.md 3.
