+++
id = "01M4SPWP4HDVWPMA4AFJ65KWPN"
title = "Expression grammar with attributes, was and source"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWG5A0FJNPKRPDDCJQZN8"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T04:03:58Z"
persona = "design engineer"
capability = "write expressions, @emit/@display/@native attributes, `was` and `source`"
outcome_text = "derived values and renames are first-class"
labels = ["creates:crates/cadetta-syntax/src/expr.rs"]
scope = ["crates/cadetta-syntax/src/expr.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWPAJ4ZGEGGVF63ZY806Z"

[[links]]
kind = "blocks"
target = "01M4SPWPCW1C0QC67GZG99EHTZ"

[[acceptance]]
text = "Given `2 * Wheel.hub_width + Frame.span`, when parsed, then precedence is multiplication before addition"
bound = false

[[acceptance]]
text = "Given `@emit(mid)`, when parsed, then the attribute attaches to the next parameter"
bound = false

[[acceptance]]
text = "Given `was shaft_d`, when parsed, then the rename is recorded on the parameter"
bound = false
+++
