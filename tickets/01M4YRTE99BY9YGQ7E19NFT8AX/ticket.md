+++
id = "01M4YRTE99BY9YGQ7E19NFT8AX"
title = "sema: member privacy (priv members) not enforced on expression paths across packages"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T03:13:47Z"
updated = "2026-10-15T04:10:03Z"
scope = ["crates/cadetta-sema/**", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "E016 fires when an expression path reads a priv member of an item from another package"
bound = true
+++

found while working ~992PZ41: fixtures packages/e016-member-public-by-default, e016-priv-member-read-from-dependency, e016-priv-member-same-package, e016-priv-member-through-part-chain expect E016 for reads like Joint.row of a priv member; sema only checks E016 on use items.
