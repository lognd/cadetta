+++
id = "01M4YSGF4CF0MBQG2K4A50KFBS"
title = "roll-up evaluation sum/max/min(self, member) not wired into the evaluator (commercial-misspelling, screw-price)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T03:25:49Z"
updated = "2026-10-15T10:40:27Z"
scope = ["crates/cadetta-sema/**"]
+++

found while working ~6K58FR1. Fixtures stdlib/commercial-misspelling-not-a-member (expects EVAL; gets E002 on max(self, lead_time)) and stdlib/screw-price-not-in-iso4762 (expects EVAL; gets unknown function sum). rollup.rs has roll_up with unit tests only; func.rs has no sum and no (self, member) form.
