+++
id = "01M52TZ6SYRVDG9400SHM4C6HQ"
title = "CI: per-doc conformance tests each run the whole corpus (28 full runs, rust job times out)"
type = "bug"
category = "todo"
priority = "high"
points = 3
reporter = "lognd"
created = "2026-10-16T17:08:18Z"
updated = "2026-10-16T17:08:18Z"
idempotency_key = "ci-conformance-per-doc-filter"
scope = ["tests/conformance_*.rs", "crates/cadetta-cli/tests/conformance/corpus.rs"]

[[acceptance]]
text = "Each per-doc conformance test runs only its own doc's fixtures and finishes well under 300s on CI"
bound = false
+++

Each tests/conformance_<doc>.rs test calls corpus::run_corpus over the WHOLE corpus then filters by doc prefix; ~14 binaries x 2 tests = ~28 full runs, each ~9 min on CI, so *_fixtures_match_the_spec and *_corpus_is_not_empty hit nextest's 300s limit (run 38927556347). Add a doc-filtered run_corpus (skip docs outside the requested one) and use it in doc_report; share doc_report/count_in instead of copying it per binary if feasible.
