+++
id = "01M4Y2Z95B99FTVX68T4XTSTXR"
title = "Lower ScrewJoint at/onto to EdgeFloat in sema"
type = "task"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-14T20:51:57Z"
updated = "2026-10-15T09:53:58Z"
labels = ["creates:crates/cadetta-sema/src/boltfloat.rs"]
scope = ["crates/cadetta-geometry/src/float.rs", "docs/spec/geometry.md", "changelog.d/**", "crates/cadetta-sema/src/geom.rs", "crates/cadetta-sema/src/eval.rs", "crates/cadetta-geometry/src/placement.rs", "crates/cadetta-geometry/src/reexpress.rs", "crates/cadetta-sema/tests/**", "crates/cadetta-sema/src/boltfloat.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWG6Z6JVP5Z7H38HWJP0GQ"

[[acceptance]]
text = "Given a non-locating ScrewJoint whose two parts are placement parent and child in either order, whatever datum the placement names, when lowered, then the edge carries an EdgeFloat with [+-float] on x and y and a rotation [+-theta] about z through the onto centroid (geometry.md 5.2; takes over ~HWJP0GQ acceptance 2)"
bound = true

[[acceptance]]
text = "Given the joint's own aligned check, when evaluated, then it places the tree with FloatMode::Without for that joint; joints between parts that are not parent and child add no float (geometry.md 5.2; takes over ~HWJP0GQ acceptance 5)"
bound = false

[[acceptance]]
text = "Given several joints on one edge, when lowered, then the pattern points, variable ids and locating flags reach float::select unchanged (takes over the lowering half of ~HWJP0GQ acceptance 4)"
bound = false
+++

Found while working ~HWJP0GQ. The geometry crate evaluates EdgeFloat (float.rs, Assembly.floats, place_assembly_reading); nothing yet builds EdgeFloat from a ScrewJoint at/onto: pattern points, parent/child detection in either order, variable ids, and FloatMode::Without(joint path) for the joint's own aligned check.
