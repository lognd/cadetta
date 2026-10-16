+++
id = "01M5145JM1B0PVWDSJS98TQVDQ"
title = "to_pint accepts 5USD: currency quantity raises QuantityError Q002 instead of returning Err"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-16T01:10:35Z"
updated = "2026-10-16T01:47:00Z"
scope = ["bindings/python/python/cadetta/pint_interop.py", "bindings/python/tests/test_binding_fixtures.py"]

[[acceptance]]
text = "to_pint of a currency quantity returns Err; KNOWN_DIVERGENCES entry removed"
bound = true
+++

Found while working CDRJTN9. pint-currency-rejected/must-fire: to_pint(quantity('5USD')) raises _core.QuantityError (Q002 unit rad^0) from q.to_float instead of returning Err; _unit_terms does not detect the currency dimension. Replay marks it xfail in KNOWN_DIVERGENCES.
