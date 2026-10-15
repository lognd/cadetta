+++
id = "01M4V2J3BCSB3MFKDRSPGR0M75"
title = "E016 not raised for priv member read through an instance (stdlib joint)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:47:02Z"
updated = "2026-10-15T04:08:00Z"
scope = ["tests/conformance/stdlib/**", "crates/cadetta-sema/src/deps.rs", "crates/cadetta-sema/src/visibility.rs", "docs/spec/packages.md", "tests/conformance/known-gaps.toml", "changelog.d/**"]

[[acceptance]]
text = "A priv member read through an instance of an imported component (j.major) raises E016; pub members stay clean."
bound = true
+++

Fixtures failing against the binary, gapped in known-gaps.toml: d003-joint-reads-itself e016-joint-priv-intermediate m003-hole-fit-side policy-iso-default-silent. Found while working ~GDYVJ8R.
