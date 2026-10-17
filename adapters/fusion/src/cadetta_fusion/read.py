"""Read: Fusion user parameters to one observed IR project (fusion.md 3, 4)."""

# frob:ticket 01M555TVRF47KEJ406NPQMSJDX

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

from cadetta_fusion.session import NativeParam

__all__ = ["LITERAL_RE", "observed_from_parameters", "unit_scale"]

#: A literal expression: a decimal and an optional unit ("25 mm", "3", "45deg").
LITERAL_RE = re.compile(r"^\s*(-?\d+(?:\.\d+)?)\s*([A-Za-z]+)?\s*$")
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


def _package_name(path: str | None) -> str:
    """Lowercase identifier-safe stem of the first file; `observed` when none."""
    name = (
        re.sub(r"[^a-z0-9_]", "_", PurePath(path).stem.lower()).strip("_")
        if path
        else ""
    )
    if not name:
        return "observed"
    return "p" + name if name[0].isdigit() else name


def unit_scale(unit: str) -> tuple[dict[str, Any], Fraction, int, int]:
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
    parts = []
    for sym, e in zip(_SI_UNITS, list(one.dim), strict=True):
        if e:
            parts.append(f"{sym}^{e}")
            dim[sym] = e
    if not parts:
        raise QuantityError(f"unit {unit} is dimensionless")
    factor = one.to_exact("*".join(parts))
    if factor is None:
        raise QuantityError(f"unit {unit} has no exact SI scale")
    return dim, factor, 0, 0


def _resolve(param: NativeParam, path: str) -> tuple[dict[str, Any], str] | str:
    """Return (IR value, locator unit), or why the parameter is unreadable."""
    literal = LITERAL_RE.match(param.expression)
    unit = (literal.group(2) if literal else None) or param.unit
    try:
        if unit:
            dim, factor, pi, angle = unit_scale(unit)
        else:
            dim, factor, pi, angle = dict(_ZERO_DIM), Fraction(1), 0, 0
    except QuantityError as e:
        return f"unit {unit!r} is not readable: {e}"
    reason: str | None = None
    if literal is None:
        reason = "expression-valued: evaluated by Fusion from other parameters"
    if reason is None:
        assert literal is not None
        frac = Fraction(Decimal(literal.group(1))) * factor
        nominal: dict[str, Any] = {
            "r": f"{frac.numerator}/{frac.denominator}",
            "pi": pi,
        }
    else:
        # A solver or expression result is never claimed exact (adapters.md 4).
        scaled = param.value * float(factor) * (math.pi if pi else 1.0)
        nominal = {"f64": scaled, "root": path}
    value: dict[str, Any] = {
        "angle": angle,
        "authored": param.expression.replace(" ", ""),
        "conservative": False,
        "dim": dim,
        "exact": reason is None,
        "nominal": nominal,
    }
    if reason is not None:
        value["reason"] = reason
    return value, unit


def observed_from_parameters(
    designs: list[tuple[str, list[NativeParam]]],
) -> tuple[ir.Project, list[Diagnostic]]:
    """Build the observed IR of `(file, params)` designs: one synthetic component."""
    pkg = _package_name(designs[0][0] if designs else None)
    comp = f"{pkg}::Observed"
    diags: list[Diagnostic] = []
    members: list[dict[str, Any]] = []
    bindings: list[dict[str, Any]] = []
    taken: set[str] = set()
    for file, param in ((f, p) for f, ps in designs for p in ps):
        name = _safe_name(param.name, taken)
        path = f"{comp}.{name}"
        match _resolve(param, path):
            case str() as why:
                _log.warning("not read %s from %s: %s", param.name, file, why)
                taken.discard(name)
                diags.append(
                    _diag("R002", "warning", f"{param.name} not read: {why}", None)
                )
                continue
            case (value, loc_unit):
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
                    "adapter": "fusion",
                    "file": file,
                    "native_name": param.name,
                }
            },
            "value": value,
            "was": [],
        }
        if not value["exact"]:
            member["expr"] = param.expression
        members.append(member)
        bindings.append(
            {
                "adapter": "fusion",
                "path": path,
                "locators": [
                    {
                        "kind": "global",
                        "locator": param.name,
                        "mode": "value",
                        "native_unit": loc_unit or "1",
                    }
                ],
            }
        )
        _log.debug("read %s as %s", param.name, path)
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
    _log.info("observed %d params from %d designs", len(members), len(designs))
    return project, diags
