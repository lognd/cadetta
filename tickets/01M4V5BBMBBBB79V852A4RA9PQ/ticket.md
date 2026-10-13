+++
id = "01M4V5BBMBBBB79V852A4RA9PQ"
title = "Spec: json read unit precedence and number formatting gaps"
type = "docs"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-13T17:35:47Z"
updated = "2026-10-13T18:11:06Z"
scope = ["docs/spec/emitters/json.md", "docs/spec/emitters/csv.md", "docs/spec/adapters.md", "tests/conformance/emitters/json/**", "tests/conformance/emitters/csv/**"]

[[acceptance]]
text = "Given json.md 1 and 3, when the spec is read, then precedence of --assume-unit over native_unit, whole-number rendering, note on Text and the O004 behaviour are stated"
bound = true

[[acceptance]]
text = "Given a unitless value read by json or csv with both --assume-unit and a locator native_unit, when read runs, then the spec says --assume-unit wins, then native_unit, then the adapter default"
bound = true

[[acceptance]]
text = "Given json and csv with no drive, when sync runs, then the spec says the obligation CAD file reflects design stays open with reason unverifiable without drive and is reported as O004, and a binding fixture pair for each adapter covers it"
bound = true
+++

Found while working ~5RCYHAJ. emitters/json.md 3: a missing unit takes native_unit (unit_assumed) or --assume-unit, with no precedence stated when both exist (fixtures read-missing-unit-* assume --assume-unit wins). Also 1: trailing-zero trimming of a whole number (10 vs 10.0) is decided as 10 in the fixtures but not written; and whether a Text param gets a note, and whether json carries the O004 'unverifiable without drive' obligation (adapters.md 2 names only sw-equations) are unstated.
