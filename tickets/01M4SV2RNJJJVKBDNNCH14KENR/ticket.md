+++
id = "01M4SV2RNJJJVKBDNNCH14KENR"
title = "OQ-16: Tolerance in Python Quantity arithmetic"
type = "docs"
category = "done"
outcome = "wont-fix"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T05:17:05Z"
updated = "2026-10-13T05:45:34Z"
labels = ["open-question"]

[[links]]
kind = "relates"
target = "01M4SV2NGV5P9KBBT7987BZCKQ"
+++

Open question OQ-16 from bindings.md. Closes with an ADR in docs/decisions/.

Question: Should Quantity arithmetic in Python carry tolerance (affine forms, quantities.md 5.4) or nominal values only?

Recommendation: carry it; the core already computes it, so dropping it would make Python results disagree with checks. Affects the Quantity story ~87BZCKQ.
