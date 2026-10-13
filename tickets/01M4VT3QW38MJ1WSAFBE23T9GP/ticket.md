+++
id = "01M4VT3QW38MJ1WSAFBE23T9GP"
title = "obligations ack fixtures depend on the developer's git email (CI red)"
type = "bug"
category = "in-progress"
priority = "high"
points = 1
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:38:37Z"
updated = "2026-10-13T23:39:50Z"
scope = ["tests/conformance/obligations/**"]

[[acceptance]]
text = "Given the obligations fixtures that invoke ack, when run where git config user.email is unset (CI), then they pass because their args name the attester with --as"
bound = true
+++

CI has no git user.email, so ack exits 2 (no attester identity, config-cli.md 3.4) in ack-role-must-cover and five selector-* must-not-fire fixtures; they pass locally only because the developer's git email is set.
