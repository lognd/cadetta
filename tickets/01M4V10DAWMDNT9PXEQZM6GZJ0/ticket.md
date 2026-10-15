+++
id = "01M4V10DAWMDNT9PXEQZM6GZJ0"
title = "Conformance entry points ir-validate and lint on IR JSON are not in config-cli.md"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:19:54Z"
updated = "2026-10-15T01:58:38Z"
labels = ["creates:changelog.d/01M4V10DAWMDNT9PXEQZM6GZJ0.*"]
scope = ["docs/spec/**", "changelog.d/01M4V10DAWMDNT9PXEQZM6GZJ0.*"]

[[acceptance]]
text = "config-cli.md 3.9 specifies ir-validate FILE (exit 0 valid, 2 invalid) and lint FILE on IR JSON (D003 reachable)"
bound = true
+++

found while working ~99NMCBG: ir.md 6 introduces ir-validate FILE (exit 0 valid, 2 invalid); lint on an IR JSON file for D003 has no CLI form. Add both to config-cli.md or the runner contract.
