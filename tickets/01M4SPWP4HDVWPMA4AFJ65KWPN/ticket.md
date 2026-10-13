+++
id = "01M4SPWP4HDVWPMA4AFJ65KWPN"
title = "Expression grammar with tolerance, ISO class, attributes and citations"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWG5A0FJNPKRPDDCJQZN8"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T06:42:21Z"
persona = "design engineer"
capability = "write expressions, @emit/@display/@native attributes, `was` and `source`"
outcome_text = "derived values and renames are first-class"
labels = ["creates:crates/cadetta-syntax/src/expr.rs"]
scope = ["crates/cadetta-syntax/src/expr.rs", "crates/cadetta-syntax/src/lib.rs", "crates/cadetta-syntax/src/parser.rs", "docs/spec/language.md"]

[[links]]
kind = "blocks"
target = "01M4SPWPAJ4ZGEGGVF63ZY806Z"

[[links]]
kind = "blocks"
target = "01M4SPWPCW1C0QC67GZG99EHTZ"

[[links]]
kind = "blocks"
target = "01M4ST7A8ZG0BS11X26BB84S84"

[[acceptance]]
text = "Given `2 * Wheel.hub_width + Frame.span`, when parsed, then precedence is multiplication before addition (language.md 5)"
bound = true

[[acceptance]]
text = "Given the precedence list in language.md 5, when parsed, then postfix > unary - > ^ > * / > + - > .. > as > comparisons"
bound = false

[[acceptance]]
text = "Given `10mm [+-0.05mm]`, `8mm [+0 -0.008mm]`, `20mm H7` and `x j5`, when parsed, then tolerance and ISO class postfixes attach to the operand (quantities.md 5.1)"
bound = false

[[acceptance]]
text = "Given `a[0]` and `a [+-0.1mm]`, when parsed, then the first is indexing and the second a tolerance because the bracket starts with + or - (quantities.md 5.1)"
bound = false

[[acceptance]]
text = "Given `4mm <= x <= 6mm` and `x satisfies P`, when parsed, then chained comparison and satisfies nodes result (language.md 5, 6)"
bound = false

[[acceptance]]
text = "Given `if`, `match` and `{ let a = 1mm; a }`, when parsed, then each is an expression (language.md 5)"
bound = false

[[acceptance]]
text = "Given `#[emit(mid)]` trailing a member and `#![emit(nominal), display(mm)]` in a component, when parsed, then they attach as member and scope attributes (language.md 3.11)"
bound = false

[[acceptance]]
text = 'Given `was Globals.plate_thk` and `@vendor:"p.2, table 1"`, when parsed, then the rename and the citation with locator are recorded (language.md 3.3, 3.9)'
bound = false

[[acceptance]]
text = "Given `sum?(self, unit_cost)`, `Iso273[thread].medium` and `T.at_least(x).key`, when parsed, then the calls and lookups parse (language.md 3.5, 3.6)"
bound = false
+++
