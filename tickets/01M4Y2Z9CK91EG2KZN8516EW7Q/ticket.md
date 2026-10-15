+++
id = "01M4Y2Z9CK91EG2KZN8516EW7Q"
title = "Conformance fixtures for ScrewJoint placement float"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-14T20:51:57Z"
updated = "2026-10-15T11:06:03Z"
scope = ["tests/conformance/geometry/**", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocked-by"
target = "01M4Y2Z95B99FTVX68T4XTSTXR"

[[links]]
kind = "blocked-by"
target = "01M4Y2Z99SXRHPTNQS11392B89"

[[acceptance]]
text = "Given fixtures under tests/conformance/geometry/** for the placement float, when run, then they cover float on x and y, theta, 180deg when float >= s, selection among joints, locating removal, either parent/child order, and the own-check read without the float (geometry.md 5.2; takes over ~HWJP0GQ acceptance 2 to 5 at fixture level)"
bound = true
+++

Found while working ~HWJP0GQ. Fixtures need the sema lowering (~4XTSTXR) and G008 emission (~1392B89); remove the matching known-gaps entries when they pass.
