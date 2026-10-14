"""The seam between the adapter and the FreeCAD Python API (adapters.md 6)."""

# frob:ticket 01M4SPWSCZ06W3EW6B1RDPEZRQ
# frob:ticket 01M4SPWSFF7Z2K6FSGSZ1N30JH
# frob:ticket 01M4SPWSJ81C9Q9BV5AYXHEWE6

from __future__ import annotations

import importlib
import math
import os
from collections.abc import Sequence
from typing import TYPE_CHECKING, Any, Literal, Protocol

from cadetta.logging import get_logger
from pydantic import BaseModel, ConfigDict
from typani import Err, Ok, Result

if TYPE_CHECKING:
    from cadetta_freecad.emit import CellWrite

__all__ = ["FreeCadBackend", "LiveBackend", "NativeDocument", "NativeItem"]

_log = get_logger(__name__)


class NativeItem(BaseModel):
    """One readable native parameter: a Spreadsheet alias or a Sketcher dimension."""

    model_config = ConfigDict(frozen=True)

    kind: Literal["alias", "dimension"]
    name: str
    content: str | None = None
    expression: str | None = None
    value: float | None = None
    unit: str | None = None
    driven: bool = False


class NativeDocument(BaseModel):
    """What a read of one FCStd yields: its path, label and readable items."""

    model_config = ConfigDict(frozen=True)

    path: str
    label: str
    items: list[NativeItem]


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

    def read_document(self, path: str) -> Result[NativeDocument, str]:
        """Open the FCStd at `path` read-only and list its aliases and dimensions."""
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

    def read_document(self, path: str) -> Result[NativeDocument, str]:
        """Open `path`, list Spreadsheet aliases and Sketcher dimensions, close it."""
        try:
            module = importlib.import_module("FreeCAD")
        except ImportError as e:
            _log.warning("FreeCAD is not importable: %s", e)
            return Err(f"FreeCAD is not importable: {e}")
        try:
            doc = module.openDocument(path)
            items: list[NativeItem] = []
            for obj in doc.Objects:
                if obj.TypeId == "Spreadsheet::Sheet":
                    items.extend(_sheet_items(obj))
                elif obj.TypeId == "Sketcher::SketchObject":
                    items.extend(_sketch_items(obj))
            label = str(doc.Label)
            module.closeDocument(doc.Name)
        except (RuntimeError, OSError) as e:  # FreeCADError is a RuntimeError
            _log.error("reading %s failed: %s", path, e)
            return Err(f"reading {path} failed: {e}")
        _log.info("read %d native items from %s", len(items), path)
        return Ok(NativeDocument(path=path, label=label, items=items))


def _sheet_items(sheet: Any) -> list[NativeItem]:
    """List the aliased cells of a Spreadsheet as `alias` items."""
    out: list[NativeItem] = []
    for cell in sheet.getUsedCells():
        alias = sheet.getAlias(cell)
        if not alias:
            continue
        out.append(
            NativeItem(kind="alias", name=alias, content=str(sheet.getContents(cell)))
        )
    return out


_DISTANCE = frozenset({"Distance", "DistanceX", "DistanceY", "Radius", "Diameter"})


def _sketch_items(sketch: Any) -> list[NativeItem]:
    """List the named dimensional constraints of a sketch as `dimension` items."""
    out: list[NativeItem] = []
    expressions = {path: expr for path, expr in sketch.ExpressionEngine}
    for i, con in enumerate(sketch.Constraints):
        if not con.Name:
            continue
        if con.Type in _DISTANCE:
            value, unit = float(con.Value), "mm"
        elif con.Type == "Angle":
            value, unit = math.degrees(float(con.Value)), "deg"
        else:
            continue
        out.append(
            NativeItem(
                kind="dimension",
                name=f"{sketch.Name}.{con.Name}",
                expression=expressions.get(f"Constraints.{con.Name}"),
                value=value,
                unit=unit,
                driven=not con.Driving,
            )
        )
        _log.debug("sketch %s constraint %d %s", sketch.Name, i, con.Name)
    return out
