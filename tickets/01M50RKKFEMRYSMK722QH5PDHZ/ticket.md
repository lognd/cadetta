+++
id = "01M50RKKFEMRYSMK722QH5PDHZ"
title = "Fix s006 solver fixtures: S008 cite and array-part solver input"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T21:48:32Z"
updated = "2026-10-16T08:25:01Z"
scope = ["tests/conformance/solvers/s006-*/**,tests/conformance/solvers/s006-*,tests/conformance/known-gaps.toml", "tests/conformance/solvers/s006-*/**"]

[[acceptance]]
text = "The two s006 variant fixtures (unsolved-variant, unchanged-variant-reads-primary) pass in the conformance corpus; their gaps are removed; the primary-missing fixture is re-gapped under ~RWGVA1F"
bound = true
+++

Found while landing ~B645GTM. check now emits S006 (unit-tested), but the three s006-* fixtures still fail for fixture reasons: (1) their cite src is a url with neither file nor accessed, so S008 fires in both must-fire and must-not-fire; (2) they read 'in load = wheel.load_rating' on 'part wheel[4]', which the evaluator rejects ('is a part array; read one instance'), so the solver input never evaluates. Fix the fixtures (add accessed to the cite, index one instance or use a non-array part, regenerate analysis/*.out.json input digests), then remove the s006-* entries from known-gaps.toml.
