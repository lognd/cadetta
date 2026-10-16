+++
id = "01M51C7447BXNMGJXBFBSPG4VY"
title = "EVAL checks for part arrays and array literals not implemented (language.md 3.5/3.8)"
type = "bug"
category = "todo"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-16T03:31:15Z"
updated = "2026-10-16T10:01:18Z"
+++

Found by ~4GVR0SD fixtures (known-gaps.toml). Part count w[3mm], w[-1] and fractional w[2.5] (Q009) produce no diagnostic; indexing a part array (w[3].mass) in a let reports 'only an array or a table can be indexed'; w.mass in a let reports 'read one instance of it' although language.md 3.5 allows it; an array literal of mixed kinds reports Q002 instead of EVAL.
