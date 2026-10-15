+++
id = "01M4YNMDMKRAP06VPQP71ZWJXV"
title = "IR decoder so lint FILE runs all IR-only rules (D001, D002, F004, M, N, Q006, S)"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:18:04Z"
updated = "2026-10-15T02:18:04Z"
+++

lint FILE (T 5ZFM9K6) only runs D003 via a skeleton decode; the value-reading rules need a full cadetta.ir/1 JSON decoder into cadetta_ir::model.
