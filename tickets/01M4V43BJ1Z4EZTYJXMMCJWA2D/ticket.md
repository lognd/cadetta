+++
id = "01M4V43BJ1Z4EZTYJXMMCJWA2D"
title = "Geometry evaluation: axis builtins, point ops, Q006 and joint checks"
type = "bug"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T17:13:56Z"
updated = "2026-10-14T01:53:37Z"
scope = ["crates/cadetta-ir/**", "crates/cadetta-sema/**", "crates/cadetta-geometry/**", "tests/conformance/**", "docs/spec/**"]

[[links]]
kind = "blocked-by"
target = "01M4VS6WG57MAD0T8GK956NGG9"

[[acceptance]]
text = "Given the axis fixtures (angle-between-axes, coaxial-offset, coaxial-tilt-included, parallel-ignores-offset), when cadetta check runs, then they pass and their known-gaps entries are removed"
bound = false

[[acceptance]]
text = "Given the joint fixtures (joint-undecided-when-loosened, joint-violated-over-allowance), when they are not yet evaluable, then their known-gaps entries are retargeted to ~6MJE3XF"
bound = false
+++

found while working ~ZFM3633. Fixtures aligned-*, coaxial-*, parallel-*, angle-between-axes, tolerance-*, q006-*, joint-* and the value fixtures need Point2/Point3 typed positions, datum ops, aligned/coaxial/parallel/angle_between as check conditions, and Q006 for rotations/circle patterns; the binary reports EVAL errors (this operator is not evaluated by this stage).
