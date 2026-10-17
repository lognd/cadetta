+++
id = "01M4YSGF4CF0MBQG2K4A50KFBS"
title = "roll-up evaluation sum/max/min(self, member) not wired into the evaluator (commercial-misspelling, screw-price)"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T03:25:49Z"
updated = "2026-10-15T11:59:34Z"
scope = ["crates/cadetta-sema/**", "docs/spec/language.md", "tests/conformance/known-gaps.toml", "tests/conformance/stdlib/commercial-misspelling-not-a-member.must-fire.cdt", "tests/conformance/stdlib/commercial-misspelling-not-a-member.must-not-fire.cdt", "tests/conformance/stdlib/screw-price-not-in-iso4762.must-not-fire.cdt"]

[[acceptance]]
text = "sum/max/min(self, m) and the ? forms evaluate over every part instance, counting arrays"
bound = true

[[acceptance]]
text = "a roll-up over a misspelled member, a missing argument, no contributing part or mixed dimensions is EVAL, not E002"
bound = true

[[acceptance]]
text = "a parameterized part contributes the argument its part site passes (ScrewJoint screw_cost)"
bound = true
+++

found while working ~6K58FR1. Fixtures stdlib/commercial-misspelling-not-a-member (expects EVAL; gets E002 on max(self, lead_time)) and stdlib/screw-price-not-in-iso4762 (expects EVAL; gets unknown function sum). rollup.rs has roll_up with unit tests only; func.rs has no sum and no (self, member) form.
