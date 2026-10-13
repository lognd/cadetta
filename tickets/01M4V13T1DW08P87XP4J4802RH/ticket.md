+++
id = "01M4V13T1DW08P87XP4J4802RH"
title = "language: lint gaps (C001/C002 bounds, L001/L002 lifecycle)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:21:45Z"
updated = "2026-10-13T16:21:45Z"
scope = ["crates/cadetta-lint/**"]
+++

found while working ~7MEVQAH. Fixtures under tests/conformance/language/ are spec-true; these sides disagree with the binary:  c001-bound-violated, c002-bound-undecided, c002-fit-undecided, l001-date-passed, l001-eol-no-alternate, l001-month-grain, l002-nrnd.
