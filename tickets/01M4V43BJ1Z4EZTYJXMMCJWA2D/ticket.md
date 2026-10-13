+++
id = "01M4V43BJ1Z4EZTYJXMMCJWA2D"
title = "Geometry evaluation: axis builtins, point ops, Q006 and joint checks"
type = "bug"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T17:13:56Z"
updated = "2026-10-13T23:29:31Z"
scope = ["crates/cadetta-ir/**"]
+++

found while working ~ZFM3633. Fixtures aligned-*, coaxial-*, parallel-*, angle-between-axes, tolerance-*, q006-*, joint-* and the value fixtures need Point2/Point3 typed positions, datum ops, aligned/coaxial/parallel/angle_between as check conditions, and Q006 for rotations/circle patterns; the binary reports EVAL errors (this operator is not evaluated by this stage).
