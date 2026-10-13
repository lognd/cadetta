+++
id = "01M4SXVA290E2HY5B284N1B3H9"
title = "std citation exports at the package root"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T06:05:26Z"
updated = "2026-10-13T09:02:20Z"
persona = "design engineer"
capability = "cite a standard with one line"
labels = ["creates:std/src/lib.cdt"]
scope = ["std/src/lib.cdt", "crates/cadetta-syntax/tests/std_lib_root.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST7AX1C1SYB67DVDW1EB06"

[[links]]
kind = "blocked-by"
target = "01M4STANV9T55FV8529E2RPFB1"

[[acceptance]]
text = "Given user code `cite din472 = std::din472`, when resolved and checked, then the citation resolves and no S001 fires (stdlib.md intro; packages.md 3)"
bound = true

[[acceptance]]
text = "Given the exported items of std, when listed, then every one is pub and exactly the items stdlib.md lists are exported (packages.md 3)"
bound = true

[[acceptance]]
text = "Given std lib.cdt, when read, then it exports one pub cite per standard it uses at the root (std::iso273, std::iso286, std::din472 and so on) (stdlib.md intro)"
bound = true
+++
