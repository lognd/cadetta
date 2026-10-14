"""The seam between the adapter and the FreeCAD Python API (adapters.md 6)."""

# frob:ticket 01M4SPWSCZ06W3EW6B1RDPEZRQ
# frob:ticket 01M4SPWSFF7Z2K6FSGSZ1N30JH
# frob:ticket 01M4SPWSJ81C9Q9BV5AYXHEWE6
# frob:ticket 01M4SPWSMZASH05AGR95PGAZKD

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

__all__ = [
    "DocState",
    "FreeCadBackend",
    "LiveBackend",
    "NativeDocument",
    "NativeItem",
]

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


class DocState(BaseModel):
    """An opened document: unsaved changes, read-only, and whether we opened it."""

    model_config = ConfigDict(frozen=True)

    path: str
    dirty: bool = False
    readonly: bool = False
    attached: bool = False


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
        """List the aliases and dimensions of `path`, attaching if it is open."""
        ...

    def open_document(self, path: str) -> Result[DocState, str]:
        """Attach to `path` if open, else open it hidden; report its state."""
        ...

    def get_item(self, path: str, locator: str) -> Result[NativeItem | None, str]:
        """Return the alias or `Sketch.Name` dimension `locator`, None if absent."""
        ...

    def set_item(self, path: str, item: NativeItem) -> Result[None, str]:
        """Write `item` (content or value plus expression) to its native place."""
        ...

    def recompute(self, path: str) -> Result[list[str], str]:
        """Recompute `path`; return one message per failed object (empty is ok)."""
        ...

    def save_document(self, path: str) -> Result[None, str]:
        """Save the open document `path` in place."""
        ...

    def close_document(self, path: str) -> Result[None, str]:
        """Close the open document `path`."""
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
            doc, opened = _attach(module, path)
            items: list[NativeItem] = []
            for obj in doc.Objects:
                if obj.TypeId == "Spreadsheet::Sheet":
                    items.extend(_sheet_items(obj))
                elif obj.TypeId == "Sketcher::SketchObject":
                    items.extend(_sketch_items(obj))
            label = str(doc.Label)
            if opened:
                module.closeDocument(doc.Name)
        except (RuntimeError, OSError) as e:  # FreeCADError is a RuntimeError
            _log.error("reading %s failed: %s", path, e)
            return Err(f"reading {path} failed: {e}")
        _log.info("read %d native items from %s", len(items), path)
        return Ok(NativeDocument(path=path, label=label, items=items))

    def open_document(self, path: str) -> Result[DocState, str]:
        """Attach to or open `path` and report dirty and read-only state."""
        match _module():
            case Err(why):
                return Err(why)
            case Ok(module):
                pass
        try:
            doc, opened = _attach(module, path)
            readonly = bool(getattr(doc, "ReadOnly", False)) or not os.access(
                path, os.W_OK
            )
            dirty = False if opened else bool(doc.isTouched())
        except (RuntimeError, OSError) as e:
            _log.error("opening %s failed: %s", path, e)
            return Err(f"opening {path} failed: {e}")
        _log.info("opened %s dirty=%s readonly=%s", path, dirty, readonly)
        return Ok(
            DocState(path=path, dirty=dirty, readonly=readonly, attached=not opened)
        )

    def get_item(self, path: str, locator: str) -> Result[NativeItem | None, str]:
        """Return the named alias or dimension of the open document, or None."""
        match _open_doc(path):
            case Err(why):
                return Err(why)
            case Ok(doc):
                pass
        try:
            if "." in locator:
                sketch_name = locator.split(".", 1)[0]
                sketch = doc.getObject(sketch_name)
                if sketch is None:
                    return Ok(None)
                found = [i for i in _sketch_items(sketch) if i.name == locator]
                return Ok(found[0] if found else None)
            for obj in doc.Objects:
                if obj.TypeId != "Spreadsheet::Sheet":
                    continue
                for item in _sheet_items(obj):
                    if item.name == locator:
                        return Ok(item)
        except (RuntimeError, OSError) as e:
            _log.error("reading %s of %s failed: %s", locator, path, e)
            return Err(f"reading {locator} of {path} failed: {e}")
        return Ok(None)

    def set_item(self, path: str, item: NativeItem) -> Result[None, str]:
        """Write an alias cell's content or a dimension's value and expression."""
        match _open_doc(path):
            case Err(why):
                return Err(why)
            case Ok(doc):
                pass
        try:
            if item.kind == "dimension":
                sketch_name, con = item.name.split(".", 1)
                sketch = doc.getObject(sketch_name)
                if item.expression is None:
                    sketch.setExpression(f"Constraints.{con}", None)
                    if item.value is not None:
                        value = item.value
                        if item.unit == "deg":
                            value = f"{value} deg"
                        sketch.setDatum(con, _quantity(value))
                else:
                    sketch.setExpression(f"Constraints.{con}", item.expression)
            else:
                for obj in doc.Objects:
                    if obj.TypeId == "Spreadsheet::Sheet":
                        cell = obj.getCellFromAlias(item.name)
                        if cell:
                            obj.set(cell, item.content or "")
                            break
        except (RuntimeError, OSError) as e:
            _log.error("writing %s of %s failed: %s", item.name, path, e)
            return Err(f"writing {item.name} of {path} failed: {e}")
        _log.debug("set %s in %s", item.name, path)
        return Ok(None)

    def recompute(self, path: str) -> Result[list[str], str]:
        """Recompute the open document and list the objects that failed."""
        match _open_doc(path):
            case Err(why):
                return Err(why)
            case Ok(doc):
                pass
        try:
            doc.recompute()
            failed = [
                f"{o.Name}: {o.getStatusString()}"
                for o in doc.Objects
                if not o.isValid()
            ]
        except (RuntimeError, OSError) as e:
            _log.error("recompute of %s failed: %s", path, e)
            return Err(f"recompute of {path} failed: {e}")
        _log.info("recomputed %s: %d failed objects", path, len(failed))
        return Ok(failed)

    def save_document(self, path: str) -> Result[None, str]:
        """Save the open document in place."""
        match _open_doc(path):
            case Err(why):
                return Err(why)
            case Ok(doc):
                pass
        try:
            doc.save()
        except (RuntimeError, OSError) as e:
            _log.error("saving %s failed: %s", path, e)
            return Err(f"saving {path} failed: {e}")
        _log.info("saved %s", path)
        return Ok(None)

    def close_document(self, path: str) -> Result[None, str]:
        """Close the open document."""
        match _open_doc(path):
            case Err(why):
                return Err(why)
            case Ok(doc):
                pass
        try:
            importlib.import_module("FreeCAD").closeDocument(doc.Name)
        except (RuntimeError, OSError) as e:
            _log.error("closing %s failed: %s", path, e)
            return Err(f"closing {path} failed: {e}")
        _log.info("closed %s", path)
        return Ok(None)


def _module() -> Result[Any, str]:
    """Import the FreeCAD module, or say why not."""
    try:
        return Ok(importlib.import_module("FreeCAD"))
    except ImportError as e:
        _log.warning("FreeCAD is not importable: %s", e)
        return Err(f"FreeCAD is not importable: {e}")


def _attach(module: Any, path: str) -> tuple[Any, bool]:
    """Return (document, opened_by_us): the open document at `path`, else open it."""
    want = os.path.realpath(path)
    for doc in module.listDocuments().values():
        if doc.FileName and os.path.realpath(doc.FileName) == want:
            return doc, False
    return module.openDocument(path), True


def _open_doc(path: str) -> Result[Any, str]:
    """Return the already open document at `path`, or an Err."""
    match _module():
        case Err(why):
            return Err(why)
        case Ok(module):
            pass
    want = os.path.realpath(path)
    for doc in module.listDocuments().values():
        if doc.FileName and os.path.realpath(doc.FileName) == want:
            return Ok(doc)
    return Err(f"{path} is not open")


def _quantity(value: Any) -> Any:
    """Wrap a number (mm) or `N deg` string as a FreeCAD Quantity for setDatum."""
    module = importlib.import_module("FreeCAD")
    return module.Units.Quantity(f"{value} mm" if not isinstance(value, str) else value)


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
