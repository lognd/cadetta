+++
id = "01M4SPWSABPXKK62B5V0SMJ8FD"
title = "Built-in sw-equations adapter (emit and read)"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T13:34:27Z"
persona = "design engineer"
capability = "emit and read an equations text file for Link to external file"
outcome_text = "SOLIDWORKS globals update without retyping"
labels = ["creates:crates/cadetta-adapter/src/builtin/sw_equations.rs"]
scope = ["crates/cadetta-adapter/src/builtin/sw_equations.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWTA1PQV15S63741V30XT"

[[links]]
kind = "blocks"
target = "01M4STCF3A4JQGPYSE7BBHK4BD"

[[acceptance]]
text = "Given an IR and bindings, when emitted, then line 1 is the header `' cadetta emit sw-equations 0.1.0 ir=blake3:<hex> bindings=blake3:<hex>` (emitters/sw-equations.md 1)"
bound = true

[[acceptance]]
text = 'Given global and dimension locators, when emitted, then global lines are `"NAME" = VALUE` and dimension lines carry `"GLOBAL"` in ref mode or a value in value mode (emitters/sw-equations.md 1)'
bound = true

[[acceptance]]
text = "Given a value, when written, then it is the emit point in the locator native_unit with unit suffix at 6 decimals, trailing zeros trimmed; unrepresentable is Q004; Counts and Ratios are bare; Text is not emitted (emitters/sw-equations.md 1)"
bound = true

[[acceptance]]
text = "Given a trailing comment, when written, then it holds the authored literal if its text differs, then the emit point if not nominal, then the citation, and nothing else (emitters/sw-equations.md 1)"
bound = false

[[acceptance]]
text = "Given doc comments, when emitted, then they become comment lines directly above the first line of the path (emitters/sw-equations.md 1)"
bound = false

[[acceptance]]
text = "Given paths, when ordered, then dependency order with ties by canonical path, globals then dimensions in bindings-file order (emitters/sw-equations.md 2)"
bound = false

[[acceptance]]
text = "Given a file with expressions, functions or IIF, when read, then expressions become observed-IR expressions and unparseable lines are kept with exact:false and the raw text (emitters/sw-equations.md 3)"
bound = false

[[acceptance]]
text = "Given the same inputs twice, when emitted, then the bytes are identical (golden file) (adapters.md 4 rule 2)"
bound = false

[[acceptance]]
text = "Given the equations emitter, when run, then comments, ordering and SOLIDWORKS naming follow emitters/sw-equations.md 4 (component<2> forms, prefix)"
bound = false
+++
