"""to_pint: optional pint interop returns equivalent quantities or a named-extra Err."""

# frob:ticket 01M4SV2P8R37D8SQA4G3AQ8XV1

import builtins

import cadetta
import pint
import pytest
from cadetta.pint_interop import to_pint


def q(text: str) -> cadetta.Quantity:
    return cadetta.quantity(text).unwrap()


# frob:tests bindings/python/python/cadetta/pint_interop.py::to_pint kind=unit
def test_length_converts_to_equivalent_pint_quantity():
    ureg = pint.UnitRegistry()
    out = to_pint(q("12mm"), ureg).unwrap()
    assert out.to("mm").magnitude == pytest.approx(12.0)
    assert out.check("[length]")


# frob:tests bindings/python/python/cadetta/pint_interop.py::to_pint kind=unit
def test_compound_dimension_and_dimensionless():
    ureg = pint.UnitRegistry()
    force = to_pint(q("3N"), ureg).unwrap()
    assert force.to("newton").magnitude == pytest.approx(3.0)
    ratio = to_pint(q("3N") / q("1N"), ureg).unwrap()
    assert ratio.dimensionless
    assert ratio.magnitude == pytest.approx(3.0)


# frob:tests bindings/python/python/cadetta/pint_interop.py::to_pint kind=unit
def test_angle_maps_to_radian():
    ureg = pint.UnitRegistry()
    out = to_pint(q("180deg"), ureg).unwrap()
    assert out.to("radian").magnitude == pytest.approx(3.14159265, rel=1e-6)


# frob:tests bindings/python/python/cadetta/pint_interop.py::to_pint kind=unit
def test_missing_pint_is_err_naming_extra(monkeypatch):
    real_import = builtins.__import__

    def fake(name, *a, **k):
        if name == "pint":
            raise ImportError("no pint")
        return real_import(name, *a, **k)

    monkeypatch.setattr(builtins, "__import__", fake)
    r = to_pint(q("12mm"), object())
    assert r.is_err
    assert "cadetta[pint]" in str(r.unwrap_err())
