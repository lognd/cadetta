+++
id = "01M4YMKHAW1T9QY9FWJNQ0HFFK"
title = "Quantities: inexact cbrt (non-cube / toleranced) needs units RootOp::Cbrt and interval cbrt"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:00:07Z"
updated = "2026-10-15T02:02:54Z"
labels = ["creates:changelog.d/01M4YMKHAW1T9QY9FWJNQ0HFFK.added.md"]
scope = ["crates/cadetta-units/**", "crates/cadetta-sema/**", "docs/spec/quantities.md", "changelog.d/01M4YMKHAW1T9QY9FWJNQ0HFFK.added.md"]

[[acceptance]]
text = "cbrt of a non-cube or toleranced value yields an inexact value with interval and root cbrt (sema + units)"
bound = true

[[acceptance]]
text = "units Value::cbrt and interval cbrt are exact for cubes and conservative otherwise"
bound = true
+++

found while working ~FP8V54M. cbrt currently exact only for perfect cubes without tolerance (EVAL otherwise); sqrt-style inexact handling needs cadetta-units support.
