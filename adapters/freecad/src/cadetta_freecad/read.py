"""Read: native FreeCAD documents to one observed IR project (adapters.md 3, 6)."""

# frob:ticket 01M4SPWSJ81C9Q9BV5AYXHEWE6

from __future__ import annotations

import math
import re
from decimal import Decimal
from fractions import Fraction
from typing import Any

from cadetta import QuantityError
from cadetta.adapter import (
    ZERO_DIM,
    Diagnostic,
    observed_binding,
    observed_member,
    observed_project,
    package_name,
    ratio,
    safe_name,
    unit_scale,
)
from cadetta.logging import get_logger
from cadetta.models import ir

from cadetta_freecad.backend import NativeDocument, NativeItem

__all__ = ["observed_from_documents"]

#: A cell literal as emit writes it: optional `=`, a decimal, an optional unit.
_LITERAL_RE = re.compile(r"^=?\s*(-?\d+(?:\.\d+)?)\s*([A-Za-z]+)?\s*$")
#: Longest decimal (significant digits) a binary64 is trusted to carry exactly.
_SHORT_DECIMAL_DIGITS = 15

_log = get_logger(__name__)


def _diag(code: str, severity: str, message: str, path: str | None) -> Diagnostic:
    """Build a diagnostic record."""
    return Diagnostic(code=code, severity=severity, message=message, path=path)


def _decimal_value(text: str) -> Fraction:
    """The exact rational of a decimal literal."""
    return Fraction(Decimal(text))


def _short_decimal(f: float) -> Fraction | None:
    """The decimal a binary64 prints as, when it is short enough to trust."""
    dec = Decimal(repr(f))
    if len(dec.as_tuple().digits) > _SHORT_DECIMAL_DIGITS:
        return None
    return Fraction(dec)


def _resolve(
    item: NativeItem, assume_unit: str | None, native_unit: str, path: str
) -> tuple[dict[str, Any], str, bool] | str:
    """Return (value, locator unit, unit assumed), or why the item is unreadable."""
    unit: str | None = item.unit
    number: Fraction | None = None
    inexact_f: float | None = None
    authored = item.name
    if item.content is not None:
        m = _LITERAL_RE.match(item.content)
        if m is None:
            return f"cell content {item.content!r} is not a literal"
        number = _decimal_value(m.group(1))
        unit = m.group(2)
        authored = item.content.lstrip("= ").replace(" ", "")
    elif item.value is not None:
        authored = repr(item.value)
        number = _short_decimal(item.value)
        if number is None:
            inexact_f = item.value
    else:
        return "no value is exposed"
    assumed = False
    if unit is None and item.kind == "dimension":
        unit, assumed = assume_unit or native_unit, True
    reason: str | None = None
    if unit is None:
        dim, factor, pi, angle = dict(ZERO_DIM), Fraction(1), 0, 0
    else:
        try:
            dim, factor, pi, angle = unit_scale(unit)
        except QuantityError as e:
            return f"unit {unit!r} is not readable: {e}"
    if item.driven:
        reason = "driven (reference) dimension: computed by the solver, not authored"
    elif number is None:
        reason = "binary64 value is not a short decimal"
    if reason is None:
        assert number is not None
        nominal: dict[str, Any] = ratio(number * factor, pi)
    else:
        # Never claim a solver output or a long binary64 exact (adapters.md 4).
        f = float(number) if number is not None else inexact_f
        assert f is not None
        nominal = {"f64": f * float(factor) * (math.pi if pi else 1.0), "root": path}
    value: dict[str, Any] = {
        "angle": angle,
        "authored": authored,
        "conservative": False,
        "dim": dim,
        "exact": reason is None,
        "nominal": nominal,
    }
    if reason is not None:
        value["reason"] = reason
    if assumed:
        value["unit_assumed"] = unit
    return value, unit or native_unit, assumed


def observed_from_documents(
    docs: list[NativeDocument],
    assume_unit: str | None = None,
    native_unit: str = "mm",
) -> tuple[ir.Project, list[Diagnostic]]:
    """Build the observed IR of `docs`: one synthetic component, imported params."""
    pkg = package_name(docs[0].path if docs else None)
    comp = f"{pkg}::Observed"
    diags: list[Diagnostic] = []
    members: list[dict[str, Any]] = []
    bindings: list[dict[str, Any]] = []
    taken: set[str] = set()
    for doc in docs:
        for item in doc.items:
            name = safe_name(item.name, taken)
            path = f"{comp}.{name}"
            match _resolve(item, assume_unit, native_unit, path):
                case str() as why:
                    _log.warning("not read %s from %s: %s", item.name, doc.path, why)
                    taken.discard(name)
                    diags.append(
                        _diag("R002", "warning", f"{item.name} not read: {why}", None)
                    )
                    continue
                case (value, loc_unit, _assumed):
                    pass
            if not value["exact"]:
                diags.append(_diag("Q006", "info", f"{path}: {value['reason']}", path))
            members.append(
                observed_member(
                    path, value, "freecad", doc.path, item.name, item.expression
                )
            )
            bindings.append(
                observed_binding(
                    "freecad",
                    path,
                    item.kind if item.kind == "dimension" else "global",
                    item.name,
                    loc_unit,
                )
            )
            _log.debug("read %s as %s", item.name, path)
    project = observed_project(pkg, comp, members, bindings)
    _log.info("observed %d params from %d documents", len(members), len(docs))
    return project, diags
