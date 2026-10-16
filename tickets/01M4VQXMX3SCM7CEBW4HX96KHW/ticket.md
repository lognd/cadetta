+++
id = "01M4VQXMX3SCM7CEBW4HX96KHW"
title = "Conformance runner: executor for quantities binding check entries"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:20Z"
updated = "2026-10-16T05:16:42Z"
scope = ["crates/cadetta-cli/tests/**"]

[[acceptance]]
text = "Those fixtures execute and are compared; known-gaps.toml entries removed."
bound = true
+++

found while working ~6BCVN48: quantities/ has 10 kind=binding entry=check fixtures needing the display formatter and emit point observable (see ~K2NMWYA, ~ZV526KN); no executor exists.
