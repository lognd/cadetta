+++
id = "01M4SV2T5ACY1EFVCXF3JQ1E7R"
title = "C ABI binding (bindings/c)"
type = "story"
category = "in-progress"
priority = "low"
points = 8
parent = "01M4SV2RY3PVQ3TBN2CAYWAJQY"
reporter = "lognd"
created = "2026-10-13T05:17:07Z"
updated = "2026-10-16T09:28:32Z"
persona = "C or MATLAB user"
capability = "call cadetta through a stable C ABI"
outcome_text = "non-Python languages get in-process interop"
labels = ["later", "creates:bindings/c/**"]
scope = ["bindings/c/**", ".github/workflows/ci.yml", "Cargo.toml", "docs/spec/bindings.md", "bindings/README.md", ".github/workflows/release.yml", "Cargo.lock"]

[[links]]
kind = "blocked-by"
target = "01M4SV2P5W45J454JP96ZMPC6J"

[[acceptance]]
text = "Given bindings/c cadetta-capi, when a C test program evaluates a conformance project, then results equal the CLI --format json output (bindings.md 1, 2)"
bound = false

[[acceptance]]
text = "Given the bindings layout, when added, then it follows bindings/README.md steps 1-5 including its own CI job and README row (bindings.md 2)"
bound = false
+++
