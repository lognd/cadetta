"""Quantity: exact parsing, tolerance limits, kind rules, inexact marking."""

# frob:ticket 01M4SV2NGV5P9KBBT7987BZCKQ

from fractions import Fraction

import cadetta
import pytest
from cadetta import Quantity, QuantityError


def q(text: str) -> Quantity:
    return cadetta.quantity(text).unwrap()


# frob:tests bindings/python/python/cadetta/quantity.py::quantity kind=unit
def test_parse_is_exact_and_converts_to_fraction():
    d = q("12mm [+-0.1mm]")
    assert d.to_exact("mm") == Fraction(12, 1)
    assert d.exact is True
    assert d.kind == "Length"


# frob:tests bindings/python/python/cadetta/quantity.py::quantity kind=unit
def test_parse_error_is_err_with_code():
    r = cadetta.quantity("12 mm")
    assert r.is_err
    assert r.unwrap_err().code == "Q007"
    assert cadetta.quantity("12mm )").is_err


# frob:tests bindings/python/python/cadetta/quantity.py::quantity kind=unit
def test_compound_and_fraction_literals_are_exact():
    assert q("2'6\"").to_exact("in") == Fraction(30)
    assert q("1-1/2in").to_exact("in") == Fraction(3, 2)
    assert q("0.1mm").to_exact("mm") == Fraction(1, 10)


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_limits_are_computed_in_rust():
    d = q("10mm [+0.018mm -0]")
    assert d.nominal.to_exact("mm") == 10
    assert d.min.to_exact("mm") == 10
    assert d.max.to_exact("mm") == Fraction(10018, 1000)
    assert d.mid.to_exact("mm") == Fraction(10009, 1000)
    assert d.min.max.to_exact("mm") == 10  # limits are untoleranced


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_mmc_lmc_need_a_role():
    d = q("10mm [+0.018mm -0]")
    with pytest.raises(QuantityError) as err:
        _ = d.mmc
    assert err.value.code == "E010"
    hole = d.with_role("hole")
    assert hole.mmc.to_exact("mm") == 10
    assert hole.lmc.to_exact("mm") == Fraction(10018, 1000)
    shaft = d.with_role("shaft")
    assert shaft.mmc.to_exact("mm") == Fraction(10018, 1000)
    assert shaft.lmc.to_exact("mm") == 10


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_no_dot_to_method():
    assert not hasattr(q("1mm"), "to")


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_inexact_value_reports_not_exact():
    r = q("2mm^2").sqrt()
    assert r.exact is False
    assert r.to_exact("mm") is None
    assert r.to_float("mm") == pytest.approx(2**0.5)
    assert q("4mm^2").sqrt().exact is True
    assert q("4mm^2").sqrt().to_exact("mm") == 2


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_pi_factor_makes_to_exact_none():
    angle = q("180deg")
    assert angle.to_exact("rad") is None
    assert angle.to_float("rad") == pytest.approx(3.141592653589793)


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_arithmetic_carries_tolerance_and_correlation():
    a = q("10mm [+-0.1mm]")
    s = a + q("5mm")
    assert s.min.to_exact("mm") == Fraction(149, 10)
    assert s.max.to_exact("mm") == Fraction(151, 10)
    # The same toleranced value cancels exactly.
    d = a - a
    assert d.min.to_exact("mm") == 0 and d.max.to_exact("mm") == 0
    assert (2 * a).max.to_exact("mm") == Fraction(202, 10)
    assert (-a).min.to_exact("mm") == Fraction(-101, 10)


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_squaring_never_goes_negative():
    x = q("1mm [+-2mm]")
    assert (x**2).min.to_exact("mm^2") == 0
    assert (x**2).max.to_exact("mm^2") == 9


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_kind_rule_violations_carry_q_codes():
    with pytest.raises(QuantityError) as err:
        q("1mm") + q("1s")
    assert err.value.code == "Q002"
    with pytest.raises(QuantityError) as err:
        q("1mm^3").sqrt()
    assert err.value.code == "Q011"
    with pytest.raises(QuantityError) as err:
        q("1mm").as_kind("Time")
    assert err.value.code == "Q002"


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_named_kinds_do_not_mix_implicitly():
    with pytest.raises(QuantityError) as err:
        q("1N*m") + q("1J")
    assert err.value.code == "Q003"
    assert q("1N*m").as_kind("Energy").kind == "Energy"


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_unit_dimension_mismatch_in_conversion():
    with pytest.raises(QuantityError) as err:
        q("1mm").to_float("s")
    assert err.value.code == "Q002"


# frob:tests bindings/python/src/quantity.rs::Quantity kind=unit
def test_division_by_interval_containing_zero_raises():
    with pytest.raises(QuantityError):
        q("1mm") / q("0mm [+-1mm]")
