+++
id = "01M4SV2NGV5P9KBBT7987BZCKQ"
title = "Python Quantity type and cadetta.quantity parsing"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:17:02Z"
updated = "2026-10-13T06:38:50Z"
persona = "Python user"
capability = "work with exact Quantities and parse source syntax"
outcome_text = "results never silently become floats"
labels = ["creates:bindings/python/python/cadetta/quantity.py", "creates:bindings/python/tests/test_quantity.py", "creates:bindings/python/src/quantity.rs"]
scope = ["bindings/python/python/cadetta/quantity.py", "bindings/python/tests/test_quantity.py", "bindings/python/src/quantity.rs", "bindings/python/src/lib.rs", "bindings/python/Cargo.toml", "bindings/python/python/cadetta/_core.pyi", "bindings/python/python/cadetta/__init__.py", "Cargo.lock", "docs/spec/bindings.md", "docs/spec/quantities.md", "design/model.grmb"]

[[links]]
kind = "blocked-by"
target = "01M4SPWHVJVHRD0KWP8ESJD81M"

[[links]]
kind = "blocked-by"
target = "01M4SPWNBYWT610BHC6GNSP35P"

[[links]]
kind = "blocked-by"
target = "01M4SPWNHPTRETQBYCM6EA2GEV"

[[acceptance]]
text = 'Given cadetta.quantity("12mm [+-0.1mm]"), when .to("mm") is called, then Fraction(12, 1) is returned and .exact is True (bindings.md 3.1)'
bound = false

[[acceptance]]
text = "Given an inexact value, when inspected, then .exact == False (bindings.md 3.1)"
bound = false

[[acceptance]]
text = "Given arithmetic that violates a kind rule, when evaluated, then a Result error carries the same Q-code the compiler reports (bindings.md 3.1, quantities.md)"
bound = false

[[acceptance]]
text = "Given a Quantity, when used, then nominal, min, max, mid, mmc and lmc are computed in Rust, to_exact returns None for inexact and there is no .to (bindings.md 3.1)"
bound = false
+++
