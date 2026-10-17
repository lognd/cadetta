+++
id = "01M51461751SB8Z9SVSAAVM8QV"
title = "std::inserts: heat-set and wire-thread inserts (0.2.0 remainder of HP6PTJH)"
type = "story"
category = "done"
outcome = "done"
priority = "low"
points = 3
reporter = "lognd"
created = "2026-10-16T01:10:50Z"
updated = "2026-10-16T10:43:41Z"
scope = ["std/src/inserts.cdt", "std/src/lib.cdt", "docs/spec/stdlib.md", "tests/conformance/stdlib/inserts-*", "crates/cadetta-syntax/tests/std_inserts.rs", "crates/cadetta-cli/src/std_embed.rs", "crates/cadetta-syntax/tests/std_lib_root.rs"]

[[acceptance]]
text = "std::inserts ships cited WireInsertInch (ASME B18.29.1) and HeatSet (vendor catalog) tables, registered in std lib and the embedded std, with a conformance fixture and a parse/shape test"
bound = true
+++

Inch fasteners and version 0.2.0 landed under ~HP6PTJH; inserts need a citable source (ASME B18.29.1 for helical coil inserts; heat-set inserts are vendor data) before values may be added.
