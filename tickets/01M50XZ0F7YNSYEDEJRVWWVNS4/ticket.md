+++
id = "01M50XZ0F7YNSYEDEJRVWWVNS4"
title = "Evaluate .hole/.shaft/.clearance for fits in parameterized instances; fix is_underived_fit definition-level path"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T23:22:09Z"
updated = "2026-10-16T00:05:31Z"
idempotency_key = "leftovers-k8v6dsy-fit-instances"
scope = ["crates/cadetta-sema/src/**"]

[[acceptance]]
text = "A fit .hole/.shaft/.clearance read inside a parameterized instance (and via the instance, e.g. housing.bore_fit.hole) evaluates to a quantity"
bound = false

[[acceptance]]
text = "A dependency on a definition-level fit whose fit is derived per instance is kept (not treated as underived)"
bound = false
+++

Leftovers from ~K8V6DSY: the evaluator does not evaluate .hole/.shaft/.clearance for fits inside parameterized instances (tests/golden/02-bearing-housing/expected.toml mentions such paths), and lower.rs is_underived_fit and its dependency drop still point at the definition-level fit path.
