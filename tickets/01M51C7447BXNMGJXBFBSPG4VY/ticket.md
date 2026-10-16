+++
id = "01M51C7447BXNMGJXBFBSPG4VY"
title = "EVAL checks for part arrays and array literals not implemented (language.md 3.5/3.8)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-16T03:31:15Z"
updated = "2026-10-16T10:15:45Z"
scope = ["crates/cadetta-sema/**", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Invalid part counts, out-of-range part-array indexing, mixed-kind array literals and unindexed reads of an empty part array each yield the EVAL/Q009 diagnostic the language fixtures require"
bound = true
+++

Found by ~4GVR0SD fixtures (known-gaps.toml). Part count w[3mm], w[-1] and fractional w[2.5] (Q009) produce no diagnostic; indexing a part array (w[3].mass) in a let reports 'only an array or a table can be indexed'; w.mass in a let reports 'read one instance of it' although language.md 3.5 allows it; an array literal of mixed kinds reports Q002 instead of EVAL.
