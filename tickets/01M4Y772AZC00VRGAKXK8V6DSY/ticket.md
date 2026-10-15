+++
id = "01M4Y772AZC00VRGAKXK8V6DSY"
title = "Derive fits whose operands are component args or parameterized instances (Resolved::Arg), e.g. H7 on bearing.outer_diameter"
type = "task"
category = "todo"
priority = "low"
points = 3
reporter = "lognd"
created = "2026-10-14T22:06:07Z"
updated = "2026-10-15T18:59:27Z"
scope = ["crates/cadetta-sema/src/fitderive.rs", "crates/cadetta-sema/src/instances.rs"]

[[acceptance]]
text = "A fit whose operand is a parameterized component argument or an instance member is evaluated and lowered into Component.fits (or a documented diagnostic is emitted), with a test"
bound = false
+++

Split from ~Q1ZBN9Y. fitderive::derive_fits skips (debug log) any fit whose operand resolves to Resolved::Arg or has no evaluated Member value; only component-member operands are derived.
