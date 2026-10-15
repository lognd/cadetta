+++
id = "01M4YMKHAW1T9QY9FWJNQ0HFFK"
title = "Quantities: inexact cbrt (non-cube / toleranced) needs units RootOp::Cbrt and interval cbrt"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:00:07Z"
updated = "2026-10-15T02:00:07Z"
scope = ["crates/cadetta-units/**", "crates/cadetta-sema/**"]
+++

found while working ~FP8V54M. cbrt currently exact only for perfect cubes without tolerance (EVAL otherwise); sqrt-style inexact handling needs cadetta-units support.
