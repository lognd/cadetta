"""The seam between the adapter and the FreeCAD Python API (adapters.md 6)."""

# frob:ticket 01M4SPWSCZ06W3EW6B1RDPEZRQ
# frob:ticket 01M4SPWSFF7Z2K6FSGSZ1N30JH

from __future__ import annotations

import importlib
import os
from collections.abc import Sequence
from typing import TYPE_CHECKING, Protocol

from cadetta.logging import get_logger
from typani import Err, Ok, Result

if TYPE_CHECKING:
    from cadetta_freecad.emit import CellWrite

__all__ = ["FreeCadBackend", "LiveBackend"]

_log = get_logger(__name__)


class FreeCadBackend(Protocol):
    """What the adapter needs from FreeCAD; tests substitute a fake."""

    def freecad_version(self) -> Result[str, str]:
        """Return the FreeCAD version string, or why FreeCAD is unreachable."""
        ...

    def write_sheet(
        self, out: str, sheet: str, cells: Sequence[CellWrite]
    ) -> Result[None, str]:
        """Write `cells` into Spreadsheet `sheet` of the FCStd at `out`, then save."""
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

    def write_sheet(
        self, out: str, sheet: str, cells: Sequence[CellWrite]
    ) -> Result[None, str]:
        """Open or create the FCStd at `out`, rewrite Spreadsheet `sheet`, save."""
        try:
            module = importlib.import_module("FreeCAD")
        except ImportError as e:
            _log.warning("FreeCAD is not importable: %s", e)
            return Err(f"FreeCAD is not importable: {e}")
        try:
            if os.path.exists(out):
                doc = module.openDocument(out)
            else:
                doc = module.newDocument("cadetta")
            obj = doc.getObject(sheet)
            if obj is not None:
                doc.removeObject(sheet)
            obj = doc.addObject("Spreadsheet::Sheet", sheet)
            for c in cells:
                obj.set(c.cell, c.content)
                if c.alias is not None:
                    obj.setAlias(c.cell, c.alias)
            doc.recompute()
            doc.saveAs(out)
            module.closeDocument(doc.Name)
        except (RuntimeError, OSError) as e:  # FreeCADError is a RuntimeError
            _log.error("writing %s failed: %s", out, e)
            return Err(f"writing {out} failed: {e}")
        _log.info("wrote %d cells to %s", len(cells), out)
        return Ok(None)
