+++
id = "01M4VSSDCJ82KNXA8MGEVK71FT"
title = "Evaluate #![requires(arg, when = cond)] as E006 missing argument"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T23:32:59Z"
updated = "2026-10-15T04:29:27Z"
scope = ["crates/cadetta-sema/**"]

[[acceptance]]
text = "#![requires(arg, when = cond)] is E006 when cond holds and arg is absent"
bound = true

[[acceptance]]
text = "requires is silent when arg is given"
bound = true
+++

found while working ~XCK8EGP: tap = Vendor without depth_available must be E006; attribute is parsed only.
