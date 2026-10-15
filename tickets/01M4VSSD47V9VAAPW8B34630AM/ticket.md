+++
id = "01M4VSSD47V9VAAPW8B34630AM"
title = "Evaluate optional params: given(x), absent members, E002 on absent reads"
type = "story"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T23:32:59Z"
updated = "2026-10-15T04:28:31Z"
scope = ["crates/cadetta-sema/**"]

[[acceptance]]
text = "given(x) is true only when the optional argument was supplied"
bound = false

[[acceptance]]
text = "#[when(cond)] members are absent while cond is false"
bound = false

[[acceptance]]
text = "reading an absent member is E002"
bound = false
+++

found while working ~XCK8EGP: ScrewJoint now declares name?: Type params, given(), and #[when(cond)] members/checks (std/src/fasteners/screwjoint.cdt); sema binds ArgSource::Absent but nothing evaluates given(), #[when] presence or E002 on absent members yet.
