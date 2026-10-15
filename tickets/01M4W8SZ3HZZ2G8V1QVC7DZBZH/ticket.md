+++
id = "01M4W8SZ3HZZ2G8V1QVC7DZBZH"
title = "golden 07/08 cadetta ir fails: depends on unknown node for std component members Din7981St42.length, ScrewJoint.unit_cost (found while working ~6MJE3XF; unverified if pre-existing)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-14T03:55:26Z"
updated = "2026-10-15T07:35:33Z"
labels = ["creates:changelog.d/01M4W8SZ3HZZ2G8V1QVC7DZBZH.fixed.md"]
scope = ["crates/cadetta-sema/src/deps.rs", "crates/cadetta-ir/src/digest.rs", "docs/spec/ir.md", "changelog.d/01M4W8SZ3HZZ2G8V1QVC7DZBZH.fixed.md"]

[[acceptance]]
text = "A commercial roll-up skips parts lacking the member, and a member reading a component argument digests"
bound = true
+++
