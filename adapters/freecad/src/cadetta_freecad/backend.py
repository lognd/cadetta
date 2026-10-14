"""The seam between the adapter and the FreeCAD Python API (adapters.md 6)."""

# frob:ticket 01M4SPWSCZ06W3EW6B1RDPEZRQ

from __future__ import annotations

import importlib
from typing import Protocol

from cadetta.logging import get_logger
from typani import Err, Ok, Result

__all__ = ["FreeCadBackend", "LiveBackend"]

_log = get_logger(__name__)


class FreeCadBackend(Protocol):
    """What the adapter needs from FreeCAD; tests substitute a fake."""

    def freecad_version(self) -> Result[str, str]:
        """Return the FreeCAD version string, or why FreeCAD is unreachable."""
        ...


class LiveBackend:
    """The real backend: imports the `FreeCAD` module lazily on first use."""

    def freecad_version(self) -> Result[str, str]:
        """Return FreeCAD's dotted version, or an Err when it is not importable."""
        try:
            module = importlib.import_module("FreeCAD")
        except ImportError as e:
            _log.warning("FreeCAD is not importable: %s", e)
            return Err(f"FreeCAD is not importable: {e}")
        parts = [str(p) for p in module.Version()[:3]]
        _log.debug("FreeCAD version parts: %s", parts)
        return Ok(".".join(parts))
