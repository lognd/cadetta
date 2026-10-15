+++
id = "01M4YPDTAGW415ZPEESX1PF271"
title = "conformance corpus/adapters/quantities/stdlib targets fail on extra U002/D002 warnings and a missing EVAL in quantities/eval-mixed-pi"
type = "bug"
category = "todo"
priority = "high"
reporter = "lognd"
created = "2026-10-15T02:31:57Z"
updated = "2026-10-15T03:10:33Z"

[[acceptance]]
text = "conformance_corpus_matches_the_spec passes"
bound = true

[[acceptance]]
text = "conformance_adapters, conformance_quantities, conformance_stdlib pass"
bound = false
+++

Found while working ~78B88BT. With the adapter hang fixed, the four conformance_* targets (corpus, adapters, quantities, stdlib) now terminate (about 100-300s each, run with timeout 300) but fail: corpus 371 fixtures, adapters 2 (adapters/unbound-param-info-only: extra U002), quantities 175, stdlib 28. Failures are extra U002/D002 warnings across geometry/language/lints/quantities/stdlib plus a missing EVAL in quantities/eval-mixed-pi, coinciding with in-flight U002/D002 lint work. Fix: decide per fixture whether the lint is correct (update the expected block) or over-eager (fix the lint), then re-run each target under timeout until green. Also consider running fixtures in parallel since each costs about 0.5s serially. Acceptance: all four conformance_* targets pass under timeout 600.
