+++
id = "01M557E9DXNA27TQJG7HXSWJEE"
title = "Extract shared unit_scale and observed-IR helpers into cadetta.adapter"
type = "task"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T15:24:44Z"
updated = "2026-10-17T15:37:28Z"
+++

Follow-up of PQMSJDX. adapters/fusion/src/cadetta_fusion/read.py copies unit_scale and the observed-IR builder shape from adapters/freecad read.py; move the shared pieces into bindings/python cadetta.adapter and have both adapters use them.
