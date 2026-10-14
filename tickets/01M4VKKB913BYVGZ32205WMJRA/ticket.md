+++
id = "01M4VKKB913BYVGZ32205WMJRA"
title = "frob pytest provider does not use the binding's virtualenv"
type = "chore"
category = "in-progress"
priority = "medium"
points = 1
parent = "01M4SPWFRR9DAEM60DX796GVPN"
reporter = "lognd"
created = "2026-10-13T21:44:49Z"
updated = "2026-10-14T02:11:31Z"
idempotency_key = "d15:frob-pytest-venv"
scope = ["frob.toml"]

[[acceptance]]
text = "Given frob.toml, when frob test or evidence add --provider pytest runs for bindings/python or adapters/freecad, then pytest runs in that project's uv environment and collection errors from the system interpreter no longer occur; uvx --from frob==0.532.0 frob check still passes"
bound = false
+++
