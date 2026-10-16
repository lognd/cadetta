"""Optional `cadetta[pint]` interop: Quantity to pint Quantity (bindings.md 3.1)."""

# frob:ticket 01M4SV2P8R37D8SQA4G3AQ8XV1

from typing import Any

from typani import Err, Ok, Result

from cadetta._core import Quantity
from cadetta.errors import QuantityError
from cadetta.logging import get_logger

__all__ = ["to_pint"]

_log = get_logger(__name__)

# SI base units in the order of the dimension vector [L, M, T, I, Theta, N, J].
_BASES = ("m", "kg", "s", "A", "K", "mol", "cd")


def _unit_terms(q: Quantity) -> list[tuple[str, int]] | None:
    """Return (unit, exponent) pairs for the SI coherent unit, None if unsupported."""
    dim = list(q.dim)
    if len(dim) > len(_BASES):
        return None  # a currency base dimension has no pint equivalent
    terms = [(u, e) for u, e in zip(_BASES, dim, strict=False) if e != 0]
    if q.angle != 0:
        terms.append(("rad", q.angle))
    return terms


def to_pint(q: Quantity, registry: Any) -> Result[Any, QuantityError]:
    """Convert the nominal of `q` to `registry.Quantity` in SI coherent units.

    Err names the `cadetta[pint]` extra when pint is not installed, or the
    reason when the quantity has no pint equivalent (tolerance is dropped).
    """
    try:
        import pint  # noqa: F401
    except ImportError:
        _log.warning("to_pint called without pint installed")
        return Err(
            QuantityError(
                "pint is not installed: install the optional extra `cadetta[pint]`"
            )
        )
    terms = _unit_terms(q)
    if terms is None:
        _log.warning("to_pint: quantity has a currency dimension: %r", q)
        return Err(QuantityError("quantity has a currency dimension; no pint unit"))
    if not terms:
        unit, pint_unit = "rad^0", "dimensionless"
    else:
        unit = "*".join(f"{u}^{e}" for u, e in terms)
        pint_unit = " * ".join(f"{u} ** {e}" for u, e in terms)
    try:
        # A currency dimension is not exposed on `dim`; it surfaces as a Q002 here.
        magnitude = q.to_float(unit)
    except QuantityError as exc:
        _log.warning("to_pint: no SI conversion to %r: %s", unit, exc)
        return Err(QuantityError(f"quantity has no pint equivalent: {exc}"))
    try:
        result = registry.Quantity(magnitude, pint_unit)
    except Exception as exc:  # pint raises many error types for foreign registries
        _log.warning("to_pint: registry rejected unit %r: %s", pint_unit, exc)
        return Err(QuantityError(f"pint registry rejected `{pint_unit}`: {exc}"))
    _log.debug("to_pint: %r -> %r", q, result)
    return Ok(result)
