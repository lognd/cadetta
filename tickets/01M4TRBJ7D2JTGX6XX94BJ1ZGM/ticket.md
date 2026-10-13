+++
id = "01M4TRBJ7D2JTGX6XX94BJ1ZGM"
title = "sema evaluator cannot evaluate std ScrewJoint (parameterized components, table index, table-by-name defaults)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T13:48:42Z"
updated = "2026-10-13T13:48:42Z"
scope = ["crates/cadetta-sema/**"]
+++

found while working ~ZDSBA7J: index_component skips parameterized components; T[x] gives 'a canonical path index must be an integer literal'; IsoDefault table-by-name values give E002 (IsoDrills, IsoEngagement, IsoTapColumns, unresolved import Iso4762Len).
