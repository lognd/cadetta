+++
id = "01M4V0NSCWCW8W0P7DG628M8AP"
title = "Conformance corpus runner"
type = "task"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-13T16:14:06Z"
updated = "2026-10-13T16:18:01Z"
idempotency_key = "conformance-runner"
scope = ["crates/cadetta-cli/tests/conformance.rs", "crates/cadetta-cli/tests/conformance/**", "tests/conformance/known-gaps.toml", "tests/conformance/README.md"]

[[acceptance]]
text = "Given tests/conformance/**/NAME.expect.json, when the runner executes each must-fire and must-not-fire fixture through the cadetta library, then produced diagnostics match the expect.json closed-list rules of tests/conformance/README.md"
bound = true

[[acceptance]]
text = "Given a fixture whose expectation is not met, when the runner runs, then it fails naming the fixture, doc and missing or extra diagnostic code"
bound = false
+++

Runner for the conformance corpus (tests/conformance/README.md). Handles kind source (.cdt files and package directories); ir and binding kinds are skipped with a logged note until their entry points exist. Fixtures not yet passing because the implementation lags the spec are listed in a checked-in allowlist file with a ticket handle each, so the runner stays green.
