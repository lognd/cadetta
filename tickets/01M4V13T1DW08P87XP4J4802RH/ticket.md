+++
id = "01M4V13T1DW08P87XP4J4802RH"
title = "language: lint gaps (C001/C002 bounds, L001/L002 lifecycle)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:21:45Z"
updated = "2026-10-15T02:47:54Z"
labels = ["creates:changelog.d/01M4V13T1DW08P87XP4J4802RH.*"]
scope = ["crates/cadetta-lint/**", "crates/cadetta-cli/src/lint_facts.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/src/cmd/ir_lint.rs", "tests/conformance/known-gaps.toml", "tests/conformance/language/**", "docs/spec/lints.md", "changelog.d/01M4V13T1DW08P87XP4J4802RH.*"]

[[acceptance]]
text = "C001 error is raised when a member tolerance lies wholly outside an interface bound"
bound = true

[[acceptance]]
text = "C002 warning is raised when a member tolerance straddles an interface bound"
bound = true

[[acceptance]]
text = "C002 warning is raised when a fit clearance only partly meets its required range"
bound = true

[[acceptance]]
text = "L001 stays an error once the EOL date has passed, even with a warning default"
bound = true

[[acceptance]]
text = "Interface bounds are lowered to SI and inherited for lint"
bound = false
+++

found while working ~7MEVQAH. Fixtures under tests/conformance/language/ are spec-true; these sides disagree with the binary:  c001-bound-violated, c002-bound-undecided, c002-fit-undecided, l001-date-passed, l001-eol-no-alternate, l001-month-grain, l002-nrnd.
