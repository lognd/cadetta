+++
id = "01M51WA4ATJ8Q0R8JN6RWGVA1F"
title = "check raises S006 for a variant when the solver has no primary result at all"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T08:12:30Z"
updated = "2026-10-16T11:01:09Z"
labels = ["conformance"]
scope = ["crates/cadetta-cli/src/pipeline.rs", "tests/conformance/solvers/s006-primary-missing-is-not-s006*", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "A solver with no recorded primary result raises no S006 for its variants; s006-primary-missing-is-not-s006 passes and its gap is removed"
bound = false

[[acceptance]]
text = "A project whose solver has no primary result raises no S006 for its alternates (the outputs are unsolved, C002 on reads)"
bound = false
+++

solvers.md 4.1: a missing primary result is an unsolved output, not S006. variant_outputs (pipeline.rs) calls read_variant, which returns Unknown whenever the variant has no entry, so a project with no sizing.out.json at all raises S006 per alternate. Fix: skip the S006 push when the primary has no stored result (site.results.lookup(&site.key, None, ..) is Missing). The fixture's must-not-fire also needs C002 (the check reads an unknown output) listed in expect.json. Found while working ~QH5PDHZ; pipeline.rs is leased by ~3RK3GPH/C0B2X17.
