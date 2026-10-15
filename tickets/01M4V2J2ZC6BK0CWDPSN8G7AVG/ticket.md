+++
id = "01M4V2J2ZC6BK0CWDPSN8G7AVG"
title = "ScrewJoint rules and checks not implemented (E006 G004 G005 joint C001 checks)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:47:02Z"
updated = "2026-10-15T04:24:27Z"
scope = ["tests/conformance/stdlib/**"]

[[acceptance]]
text = "The 11 ScrewJoint fixtures (check-*, e006-*, g004-joint-pattern-count, vendor-tap-members-absent) pass against the binary and are not gapped"
bound = true

[[acceptance]]
text = "g005-float-unlocated remains gapped under ~HWJP0GQ (needs placement-edge float, out of scope here)"
bound = true
+++

Fixtures failing against the binary, gapped in known-gaps.toml: check-aligned check-aligned-nut-floating check-depth-ok-drill check-depth-ok-vendor check-grip-positive check-not-bottomed e006-count-with-pattern e006-joint-per-pattern e006-vendor-needs-depth g004-joint-pattern-count g005-float-unlocated vendor-tap-members-absent. Found while working ~GDYVJ8R.
