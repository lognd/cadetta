+++
id = "01M4V1A44AT1NHYBE9VHZ72BYR"
title = "Conformance fixtures for language: remaining rules"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-13T16:25:12Z"
updated = "2026-10-16T02:10:34Z"
scope = ["tests/conformance/language/**"]

[[acceptance]]
text = "Given the uncovered language.md rules, when fixtures are written, then each has a must-fire and must-not-fire pair under tests/conformance/language/"
bound = true
+++

Follow-up to ~7MEVQAH. Uncovered: EVAL-class pairs (part count, index range, roll-up errors, array literal kinds, Q009), process-rule kind, defaults precedence, case canon digits, fn/recursion, bound satisfies, W003/W004, S002/S004, O003, remaining formatter pairs (indent, alignment, blank lines, final newline), keyword after dot, ISO class same line (E001), waive date E001 (parser double E001 cascade).
