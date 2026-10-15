+++
id = "01M4V1HDVGKT6AVWCWY7078TNV"
title = "Quantities: ISO 286 classes not evaluated (F002, roles from class letter, mmc/lmc, #[role])"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:11Z"
updated = "2026-10-15T02:12:19Z"
idempotency_key = "Z48-iso286"
scope = ["crates/cadetta-sema/**", "tests/conformance/known-gaps.toml", "changelog.d/01M4V1HDVGKT6AVWCWY7078TNV.added.md"]

[[acceptance]]
text = "ISO 286 class literal (20mm H7) evaluates to its table tolerance; out-of-range is F002"
bound = true

[[acceptance]]
text = "Class letter case gives hole/shaft role"
bound = true

[[acceptance]]
text = ".mmc/.lmc use the role (#[role], class letter, inherited); E010 without one"
bound = true
+++

found while working ~Z48GXKX. quantities.md 5.1, 5.2, 5.6. Fixtures f002, e010, e010-lmc, acc-mmc-hole-shaft: 'a = 20mm H7' gives EVAL; #[role(hole)] yields no role.
