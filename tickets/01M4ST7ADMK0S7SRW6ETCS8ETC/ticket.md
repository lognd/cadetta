+++
id = "01M4ST7ADMK0S7SRW6ETCS8ETC"
title = "Tables, lookups and Table<K> parameters"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T08:02:00Z"
persona = "design engineer"
capability = "look values up in cited tables by key, nearest or at-least"
idempotency_key = "replan13:Tables, lookups and Table<K> parameters"
labels = ["creates:crates/cadetta-sema/src/tables.rs"]
scope = ["crates/cadetta-sema/src/tables.rs"]

[[acceptance]]
text = "Given `Iso273[thread].medium`, when evaluated, then the exact-match row column is read (language.md 3.6)"
bound = true

[[acceptance]]
text = "Given a quantity-keyed table, when `T.at_least(x)`, `T.at_most(x)` and `T.nearest(x)` are read, then they return the smallest key >= x, largest <= x and closest with ties to the larger (language.md 3.6)"
bound = true

[[acceptance]]
text = "Given any row, when `.key` is read, then the matched key is returned (language.md 3.6)"
bound = true

[[acceptance]]
text = "Given a lookup that misses, when evaluated, then an error value is returned (and the part is non-viable under variant evaluation) (language.md 3.6, 3.2)"
bound = true

[[acceptance]]
text = "Given a key-only table such as Iso4762Len, when `.at_least(x).key` is read, then it works and column reads are rejected (language.md 3.6)"
bound = true

[[acceptance]]
text = "Given a parameter of type `Table<K>`, when a column the reader uses is missing in the table passed, then an error is reported at the use site (language.md 3.3)"
bound = true

[[acceptance]]
text = "Given an ots literal citing a table that matches none of its rows, when analysed, then the T001 fact is recorded (language.md 3.6)"
bound = false
+++
