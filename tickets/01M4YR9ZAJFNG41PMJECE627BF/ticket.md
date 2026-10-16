+++
id = "01M4YR9ZAJFNG41PMJECE627BF"
title = "bind-invalid-* fixtures: binding-kind run does not observe names_path_or_key (7 adapters fixtures)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T03:04:48Z"
updated = "2026-10-16T05:30:53Z"
scope = ["crates/cadetta-cli/tests/conformance/**", "tests/conformance/known-gaps.toml", "tests/conformance/adapters/README.md"]

[[acceptance]]
text = "The conformance runner observes names_path_or_key: true holds when the config-error message names a design path or locator key of the offending binding"
bound = true

[[acceptance]]
text = "adapters/bind-invalid-* fixtures that now pass are removed from known-gaps.toml; any that cannot pass stay gapped with a follow-up ticket"
bound = false
+++

Found while working ~X1PF271. adapters/bind-invalid-{component-form,empty-config,empty-for,empty-locator,ref-to-needs-dimension,ref-to-needs-ref-mode,sign-not-on-property}: must-fire entry does not observe names_path_or_key.
