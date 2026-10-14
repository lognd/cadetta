+++
id = "01M4TRBJ7D2JTGX6XX94BJ1ZGM"
title = "sema evaluator cannot evaluate std ScrewJoint (parameterized components, table index, table-by-name defaults)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 8
reporter = "lognd"
created = "2026-10-13T13:48:42Z"
updated = "2026-10-14T02:30:39Z"
scope = ["crates/cadetta-sema/**", "std/**", "tests/**", "docs/spec/**"]

[[acceptance]]
text = "Given T[x] with a non-literal integer/key index, when evaluated, then the table row is selected per language.md"
bound = false

[[acceptance]]
text = "Given IsoDefault table-by-name values (IsoDrills, IsoEngagement, IsoTapColumns, Iso4762Len), when resolved, then they resolve without E002"
bound = true

[[acceptance]]
text = "Given a design instantiating std::fasteners ScrewJoint with a bare enum variant argument (thread = M6), when cadetta ir runs, then the std::fasteners import, the table imports and the argument resolve with no E002"
bound = true
+++

found while working ~ZDSBA7J: index_component skips parameterized components; T[x] gives 'a canonical path index must be an integer literal'; IsoDefault table-by-name values give E002 (IsoDrills, IsoEngagement, IsoTapColumns, unresolved import Iso4762Len).
