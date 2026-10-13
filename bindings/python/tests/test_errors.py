"""QuantityError: the numeric-library error carrying the compiler's Q-code."""

# frob:ticket 01M4SWP87FJ0AYP6MYC50SS49Y

import cadetta
import pytest
from cadetta.errors import QuantityError


# frob:tests bindings/python/python/cadetta/errors.py::QuantityError kind=unit
def test_operator_violations_raise_quantity_error_with_q_code():
    a = cadetta.quantity("1mm").unwrap()
    b = cadetta.quantity("1s").unwrap()
    with pytest.raises(QuantityError) as err:
        a + b
    assert err.value.code == "Q002"
    with pytest.raises(QuantityError) as err:
        a.as_kind("Time")
    assert err.value.code == "Q002"
    assert cadetta.QuantityError is QuantityError
