+++
id = "01M4VQXNA0BKA7HPM9GNWB8SVP"
title = "Attribute role(hole) not honored for mmc/lmc emit point (E010)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T23:00:21Z"
updated = "2026-10-14T05:30:08Z"
scope = ["crates/cadetta-sema/**", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "#[role(hole), emit(mmc)] evaluates and emits; the listed fixtures pass."
bound = false
+++

found while working ~6BCVN48: quantities.md 5.6 spells #[role(hole), emit(mmc)] but the implementation raises E010; only #[hole, emit(mmc)] works. Fixtures emitters/sw-equations/comment-trailing-emit-point.must-fire and adapters/comment-trailing-order-and-citation.
