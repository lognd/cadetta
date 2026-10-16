+++
id = "01M52SAFHK1694NYSMG6NNC66V"
title = "Conformance: adapter-side harness for session attach and close-only-opened fixtures"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T16:39:31Z"
updated = "2026-10-16T17:54:32Z"
scope = [".github/workflows/ci.yml", "adapters/freecad/README.md", "adapters/freecad/tests/conformance/session_driver.py", "adapters/freecad/tests/unit/test_session_driver.py", "crates/cadetta-cli/tests/conformance/exec.rs", "crates/cadetta-cli/tests/conformance/session.rs", "crates/cadetta-cli/tests/conformance/sync.rs", "tests/conformance/README.md", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Fixtures adapters/session-attach-running and adapters/session-close-only-opened pass through an adapter-side harness and are no longer gapped"
bound = true
+++

Found while working ~TSTDBAZ. adapters.md 2.2 attach-to-running and close-only-what-it-opened live inside each adapter; the host only issues open/close over the protocol, so the executor cannot observe them without a tautological Sim. Needs a harness that drives a real adapter (solidworks/freecad) or a protocol-level contract for attached/opened sessions. Fixtures: adapters/session-attach-running, adapters/session-close-only-opened.
