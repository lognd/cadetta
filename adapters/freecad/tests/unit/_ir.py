"""Builders for small design IR documents used by the emit tests."""

from typing import Any

from cadetta.models import ir

_ZERO = {
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


def length(
    mm: str, authored: str = "", lo: str | None = None, hi: str | None = None
) -> dict[str, Any]:
    """A Length value of `mm` millimetres (a rational like '12' or '127/20')."""
    from fractions import Fraction

    def metres(text: str) -> dict[str, Any]:
        return {"r": str(Fraction(text) / 1000), "pi": 0}

    value: dict[str, Any] = {
        "angle": 0,
        "authored": authored or f"{mm}mm",
        "conservative": False,
        "dim": dict(_ZERO, m=1),
        "exact": True,
        "nominal": metres(mm),
    }
    if lo is not None and hi is not None:
        value["tol"] = {"terms": [{"var": "t", "lo": metres(lo), "hi": metres(hi)}]}
    return value


def count(n: int) -> dict[str, Any]:
    """A dimensionless Count value."""
    return {
        "angle": 0,
        "authored": str(n),
        "conservative": False,
        "dim": dict(_ZERO),
        "exact": True,
        "nominal": {"r": f"{n}/1", "pi": 0},
    }


def member(
    path: str,
    value: dict[str, Any] | None,
    deps: tuple[str, ...] = (),
    role: str | None = None,
    attrs: dict[str, str] | None = None,
) -> dict[str, Any]:
    """A param member at `path`."""
    out: dict[str, Any] = {
        "analysis": False,
        "attrs": attrs or {},
        "deps": list(deps),
        "kind": "param",
        "path": path,
        "priv": False,
        "provenance": "authored",
        "was": [],
    }
    if value is not None:
        out["value"] = value
    if role is not None:
        out["role"] = role
    return out


def project(*members: dict[str, Any], digest: str | None = "ab12") -> ir.Project:
    """A design Project holding `members` in one component."""
    doc: dict[str, Any] = {
        "schema": "cadetta.ir/1",
        "bindings": [],
        "checks": [],
        "citations": [],
        "components": [
            {
                "attrs": {},
                "class": "model",
                "fits": [],
                "implements": [],
                "members": list(members),
                "parts": [],
                "path": "Rover/axle",
            }
        ],
        "dependencies": [],
        "flavour": "design",
        "modules": [],
        "package": {"name": "rover", "version": "0.1.0"},
        "solvers": [],
    }
    if digest is not None:
        doc["digest"] = digest
    return ir.Project.model_validate(doc)


def locator(
    name: str,
    kind: str = "global",
    mode: str = "value",
    **extra: Any,
) -> dict[str, Any]:
    """A binding locator."""
    return {
        "kind": kind,
        "locator": name,
        "mode": mode,
        "native_unit": "mm",
        **extra,
    }


def binding(path: str, *locators: dict[str, Any]) -> dict[str, Any]:
    """A freecad binding of `path`."""
    return {"adapter": "freecad", "path": path, "locators": list(locators)}
