+++
id = "01M51HWCS99PJZETAR4E3MA4MY"
title = "display VALUE: accept exact fraction numerators (1/3m) for quantities/display-plain-exact-decimal"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-16T05:10:15Z"
updated = "2026-10-16T06:23:28Z"
scope = ["crates/cadetta-cli/src/cmd/display.rs"]
+++

found while working ~HX96KHW: fixture display-plain-exact-decimal passes value '1/3m'; cadetta display parse_value only accepts decimal literals, so it exits 2 (malformed unit expression '/3m'). Extend parse_value to accept N/D numerators (exact Rational), expecting 333.333333mm; then remove the known-gaps entry.
