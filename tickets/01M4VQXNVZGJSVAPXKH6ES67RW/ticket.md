+++
id = "01M4VQXNVZGJSVAPXKH6ES67RW"
title = "Fixtures use invalid part-array syntax (r012, star-binding, name-mangled-path, naming-component-field)"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:21Z"
updated = "2026-10-15T08:01:05Z"
scope = ["tests/conformance/**"]

[[acceptance]]
text = "Fixtures rewritten to valid syntax per spec and pass or carry a precise gap."
bound = true
+++

found while working ~6BCVN48: language.md 3.5 gives 'part name[N]: Type(args)' but fixtures use 'wheel: Wheel[2]', 'axle = [Axle {...}]'. R012 (differing instances) cannot be expressed with identical args; also config-cli/init-sw-equations-bind-scope has a missing must-fire fixture file.
