+++
id = "01M4V5XAW55E04T7EVWDX2KSSF"
title = "Spec: bindings.md leaves Python parse-failure and float-operand error types unspecified"
type = "task"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-13T17:45:36Z"
updated = "2026-10-15T06:09:50Z"
scope = ["docs/spec/bindings.md"]

[[acceptance]]
text = "bindings.md 3.1 states: quantity() parse failure (incl. Q007 split literal) is Err(QuantityError); float operands raise TypeError; QuantityError.code may carry E010"
bound = true
+++

found while working ~VNWRJHC. (1) cadetta.quantity returns Result but the Err payload type and whether a Q007 split literal is Err or Ok-with-diagnostic is not stated. (2) Quantity * 1.5 'floats never enter' does not say whether it raises QuantityError (which code) or TypeError. (3) Quantity.mmc on an unroled quantity raises E010 yet E010 is a lint code, not a Q-code; say whether QuantityError.code may carry lint codes. (4) member.variant has no concrete example of an alternate-part package. Fixtures assume Err/Err/TypeError/E010.
