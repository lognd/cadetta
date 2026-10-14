"""Read: native FreeCAD documents to one observed IR project (adapters.md 3, 6)."""

# frob:ticket 01M4SPWSJ81C9Q9BV5AYXHEWE6

from __future__ import annotations

import math
import re
from decimal import Decimal
from fractions import Fraction
from pathlib import PurePath
from typing import Any

from cadetta import Quantity, QuantityError
from cadetta.adapter import Diagnostic
from cadetta.logging import get_logger
from cadetta.models import ir

from cadetta_freecad.backend import NativeDocument, NativeItem

__all__ = ["observed_from_documents"]

#: A cell literal as emit writes it: optional `=`, a decimal, an optional unit.
_LITERAL_RE = re.compile(r"^=?\s*(-?\d+(?:\.\d+)?)\s*([A-Za-z]+)?\s*$")
#: Longest decimal (significant digits) a binary64 is trusted to carry exactly.
_SHORT_DECIMAL_DIGITS = 15
_SI_UNITS = ("m", "kg", "s", "A", "K", "mol", "cd")
_ZERO_DIM = {
    "A": 0,
    "K": 0,
    "angle": 0,
    "cd": 0,
    "currency": {},
    "kg": 0,
    "m": 0,
    "mol": 0,
    "s": 0,
}

_log = get_logger(__name__)


def _diag(code: str, severity: str, message: str, path: str | None) -> Diagnostic:
    """Build a diagnostic record."""
    return Diagnostic(code=code, severity=severity, message=message, path=path)


def _safe_name(raw: str, taken: set[str]) -> str:
    """Path-safe member name; non-alphanumerics become `_`, collisions get `_2`."""
    base = re.sub(r"[^A-Za-z0-9]", "_", raw) or "_"
    if base[0].isdigit():
        base = "_" + base
    name, n = base, 1
    while name in taken:
        n += 1
        name = f"{base}_{n}"
    taken.add(name)
    return name


def _package_name(docs: list[NativeDocument]) -> str:
    """Lowercase, identifier-safe stem of the first file; `observed` when none."""
    stem = PurePath(docs[0].path).stem if docs else ""
    name = re.sub(r"[^a-z0-9_]", "_", stem.lower()).strip("_")
    if not name:
        return "observed"
    return "p" + name if name[0].isdigit() else name


def _ratio(frac: Fraction, pi: int = 0) -> dict[str, Any]:
    """An exact nominal `{r, pi}` from a Fraction."""
    return {"r": f"{frac.numerator}/{frac.denominator}", "pi": pi}


def _unit_scale(unit: str) -> tuple[dict[str, Any], Fraction, int, int]:
    """Return (dim, SI value of 1 `unit`, pi multiple, angle exponent) or raise."""
    one = Quantity._parse(f"1{unit}")
    angle = one.angle
    dim = dict(_ZERO_DIM)
    if angle:
        deg = one.to_exact("deg")
        if deg is None:
            raise QuantityError(f"angle unit {unit} is not a rational of a degree")
        dim["angle"] = angle
        return dim, deg / 180, 1, angle
    exps = list(one.dim)
    parts = []
    for sym, e in zip(_SI_UNITS, exps, strict=True):
        if e:
            parts.append(f"{sym}^{e}")
            dim[sym] = e
    if not parts:
        raise QuantityError(f"unit {unit} is dimensionless")
    factor = one.to_exact("*".join(parts))
    if factor is None:
        raise QuantityError(f"unit {unit} has no exact SI scale")
    return dim, factor, 0, 0


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
        dim, factor, pi, angle = dict(_ZERO_DIM), Fraction(1), 0, 0
    else:
        try:
            dim, factor, pi, angle = _unit_scale(unit)
        except QuantityError as e:
            return f"unit {unit!r} is not readable: {e}"
    if item.driven:
        reason = "driven (reference) dimension: computed by the solver, not authored"
    elif number is None:
        reason = "binary64 value is not a short decimal"
    if reason is None:
        assert number is not None
        nominal: dict[str, Any] = _ratio(number * factor, pi)
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
    pkg = _package_name(docs)
    comp = f"{pkg}::Observed"
    diags: list[Diagnostic] = []
    members: list[dict[str, Any]] = []
    bindings: list[dict[str, Any]] = []
    taken: set[str] = set()
    for doc in docs:
        for item in doc.items:
            name = _safe_name(item.name, taken)
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
            member: dict[str, Any] = {
                "analysis": False,
                "attrs": {},
                "deps": [],
                "kind": "param",
                "path": path,
                "priv": False,
                "provenance": {
                    "imported": {
                        "adapter": "freecad",
                        "file": doc.path,
                        "native_name": item.name,
                    }
                },
                "value": value,
                "was": [],
            }
            if item.expression is not None:
                member["expr"] = item.expression
            members.append(member)
            bindings.append(
                {
                    "adapter": "freecad",
                    "path": path,
                    "locators": [
                        {
                            "kind": item.kind if item.kind == "dimension" else "global",
                            "locator": item.name,
                            "mode": "value",
                            "native_unit": loc_unit,
                        }
                    ],
                }
            )
            _log.debug("read %s as %s", item.name, path)
    project = ir.Project.model_validate(
        {
            "schema": "cadetta.ir/1",
            "bindings": bindings,
            "checks": [],
            "citations": [],
            "components": [
                {
                    "attrs": {},
                    "class": "model",
                    "fits": [],
                    "implements": [],
                    "members": members,
                    "parts": [],
                    "path": comp,
                }
            ],
            "dependencies": [],
            "flavour": "observed",
            "modules": [],
            "package": {"name": pkg, "version": "0.0.0"},
            "solvers": [],
        }
    )
    _log.info("observed %d params from %d documents", len(members), len(docs))
    return project, diags
