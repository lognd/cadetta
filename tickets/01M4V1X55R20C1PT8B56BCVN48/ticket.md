+++
id = "01M4V1X55R20C1PT8B56BCVN48"
title = "conformance runner executes only check entries"
type = "task"
category = "in-progress"
priority = "medium"
points = 8
reporter = "lognd"
created = "2026-10-13T16:35:36Z"
updated = "2026-10-13T23:12:23Z"
scope = ["crates/cadetta-cli/tests/**", "tests/conformance/known-gaps.toml", "tests/conformance/README.md", "tests/conformance/obligations/**", "tests/conformance/adapters/**", "tests/conformance/emitters/**"]

[[acceptance]]
text = "Given the conformance corpus, when the runner executes, then every fixture kind (check, emit, emit-twice, emit-read, read, classify, sync, push-plan, protocol, import, impact-swap, binary and any other kind present) runs against the real CLI or adapter host and is compared with its expect.json; none is skipped silently"
bound = true

[[acceptance]]
text = "Given a fixture whose implementation lags the spec, when the runner executes, then it is listed in tests/conformance/known-gaps.toml with a ticket handle and reason, and a listed fixture that starts passing fails the run"
bound = false
+++

Runner skips emit sw-equations, classify, sync, protocol, read, import, push-plan, impact-swap, emit-twice, emit-read and binary kinds; adapters corpus fixtures for those are written but unexecuted. found while working ~YNT9TWR
