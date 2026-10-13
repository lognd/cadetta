+++
id = "01M4SXRV8HCNXHVFX87BTFJC7S"
title = "Visibility checking: pub, pub(package), priv, E016 and E017"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:04:06Z"
updated = "2026-10-13T09:08:11Z"
persona = "library author"
capability = "keep intermediates out of my semver promise"
labels = ["creates:crates/cadetta-sema/src/visibility.rs"]
scope = ["crates/cadetta-sema/src/visibility.rs", "crates/cadetta-sema/src/lib.rs", "docs/spec/packages.md"]

[[links]]
kind = "blocked-by"
target = "01M4SXRFTTTTJE5CHCZAE8MX0Q"

[[links]]
kind = "blocked-by"
target = "01M4SXRFXB1JYNPNKSAY6WB3QC"

[[links]]
kind = "blocked-by"
target = "01M4SXRV606ZNADV511Z42YR0Q"

[[acceptance]]
text = "Given a private item, when used from its module or a descendant, then it resolves; from elsewhere E016 fires; pub widens to everywhere reachable and pub(package) to the declaring package (packages.md 3)"
bound = true

[[acceptance]]
text = "Given component members (parameters, lets, fits, checks, parts) with no modifier, when read from another module or package, then they are visible (packages.md 3)"
bound = true

[[acceptance]]
text = "Given a `priv` member read from another package, when checked, then E016 fires; read inside its package, it is allowed (packages.md 3)"
bound = true

[[acceptance]]
text = "Given a priv member, when evaluated, shown by why or impact, carried in the IR, or bound by the package own bindings, then it behaves like any member (packages.md 3)"
bound = true

[[acceptance]]
text = "Given a pub item whose interface member type, component parameter type, or function argument or result type is private, when checked, then E017 fires (packages.md 3)"
bound = true

[[acceptance]]
text = "Given a member implementing an interface member that is priv, when checked, then E017 fires (packages.md 3, 7)"
bound = true

[[acceptance]]
text = "Given `std`, when used, then every item it exports is pub (packages.md 3; stdlib.md 1)"
bound = false
+++
