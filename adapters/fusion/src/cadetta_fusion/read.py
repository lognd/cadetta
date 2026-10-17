"""Read: Fusion user parameters to one observed IR project (fusion.md 3, 4)."""

# frob:ticket 01M555TVRF47KEJ406NPQMSJDX

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

from cadetta_fusion.session import NativeParam

__all__ = ["LITERAL_RE", "observed_from_parameters"]

#: A literal expression: a decimal and an optional unit ("25 mm", "3", "45deg").
LITERAL_RE = re.compile(r"^\s*(-?\d+(?:\.\d+)?)\s*([A-Za-z]+)?\s*$")

_log = get_logger(__name__)


def _diag(code: str, severity: str, message: str, path: str | None) -> Diagnostic:
    """Build a diagnostic record."""
    return Diagnostic(code=code, severity=severity, message=message, path=path)


def _resolve(param: NativeParam, path: str) -> tuple[dict[str, Any], str] | str:
    """Return (IR value, locator unit), or why the parameter is unreadable."""
    literal = LITERAL_RE.match(param.expression)
    unit = (literal.group(2) if literal else None) or param.unit
    try:
        if unit:
            dim, factor, pi, angle = unit_scale(unit)
        else:
            dim, factor, pi, angle = dict(ZERO_DIM), Fraction(1), 0, 0
    except QuantityError as e:
        return f"unit {unit!r} is not readable: {e}"
    reason: str | None = None
    if literal is None:
        reason = "expression-valued: evaluated by Fusion from other parameters"
    if reason is None:
        assert literal is not None
        frac = Fraction(Decimal(literal.group(1))) * factor
        nominal: dict[str, Any] = ratio(frac, pi)
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
    pkg = package_name(designs[0][0] if designs else None)
    comp = f"{pkg}::Observed"
    diags: list[Diagnostic] = []
    members: list[dict[str, Any]] = []
    bindings: list[dict[str, Any]] = []
    taken: set[str] = set()
    for file, param in ((f, p) for f, ps in designs for p in ps):
        name = safe_name(param.name, taken)
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
        members.append(
            observed_member(
                path,
                value,
                "fusion",
                file,
                param.name,
                None if value["exact"] else param.expression,
            )
        )
        bindings.append(
            observed_binding("fusion", path, "global", param.name, loc_unit or "1")
        )
        _log.debug("read %s as %s", param.name, path)
    project = observed_project(pkg, comp, members, bindings)
    _log.info("observed %d params from %d designs", len(members), len(designs))
    return project, diags
