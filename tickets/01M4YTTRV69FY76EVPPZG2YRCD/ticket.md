+++
id = "01M4YTTRV69FY76EVPPZG2YRCD"
title = "conformance: emitters/json|csv name-mangled-path and note-order fixtures hit a syntax error"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T03:48:55Z"
updated = "2026-10-15T03:48:55Z"
scope = ["crates/cadetta-cli/src/**"]
+++

Found while working ~TQMFETK. emit json/csv on emitters/{json,csv}/name-mangled-path and note-order exits 2 with 'a syntax error stopped analysis' (expected 0). Inspect the fixture source vs. the grammar; either fixture or parser needs fixing.
