+++
id = "01M4V10D4E9K312RP6HV9JHZEN"
title = "R012 cannot be reached from source: instances share arguments"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:19:54Z"
updated = "2026-10-15T03:00:23Z"
scope = ["docs/spec/**"]

[[acceptance]]
text = "R012 is specified as reserved and unreachable while instances share arguments (adapters.md 3, ir.md 2, lints.md 3)"
bound = false
+++

found while working ~99NMCBG: ir.md 2 and adapters.md say a [*] binding is R012 when instance emitted values differ, but instances of a part array share arguments and tolerance variables share the nominal, so no source can make emitted values differ. Specify the case (per-instance args, for-range-dependent members) or retire R012. No conformance pair for R012 until then.
