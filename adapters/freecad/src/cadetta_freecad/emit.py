"""Emit: the IR to a deterministic Spreadsheet cell plan (adapters.md 3, 4, 6)."""

# frob:ticket 01M4SPWSFF7Z2K6FSGSZ1N30JH

from __future__ import annotations

import heapq
import json
import re
from fractions import Fraction
from typing import Literal

from blake3 import blake3
from cadetta import Quantity, QuantityError
from cadetta.adapter import Diagnostic
from cadetta.logging import get_logger
from cadetta.models import ir
from pydantic import BaseModel, ConfigDict, Field
from typani import Err, Ok, Result

__all__ = [
    "SHEET_NAME",
    "CellWrite",
    "EmitPolicy",
    "SheetPlan",
    "build_plan",
    "digest_plan",
    "to_decimal",
]

#: Name of the Spreadsheet object the adapter owns inside the document.
SHEET_NAME = "cadetta"

_ALIAS_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
_CELL_ADDRESS_RE = re.compile(r"^[A-Za-z]{1,2}[0-9]+$")

_log = get_logger(__name__)


class EmitPolicy(BaseModel):
    """The upstream policy of an emit request for FreeCAD (adapters.md 6.1)."""

    model_config = ConfigDict(extra="forbid")

    out: str
    precision: int = Field(default=6, ge=0)
    point: Literal["nominal", "mid", "mmc", "lmc"] = "nominal"
    paths: list[str] | None = None
    names: dict[str, str] = Field(default_factory=dict)
    alias_of: dict[str, str] = Field(default_factory=dict)
    native_unit: str = "mm"


class CellWrite(BaseModel):
    """One Spreadsheet cell: its address, content and optional alias."""

    model_config = ConfigDict(frozen=True)

    cell: str
    content: str
    alias: str | None = None


class SheetPlan(BaseModel):
    """The ordered cell writes that make up the `cadetta` Spreadsheet."""

    cells: list[CellWrite]


def _diag(code: str, message: str, path: str | None = None) -> Diagnostic:
    """Build an error diagnostic record."""
    return Diagnostic(code=code, severity="error", message=message, path=path)


def to_decimal(value: Fraction, precision: int) -> Result[str, str]:
    """Render `value` at `precision` fractional digits, or Err when inexact (Q004)."""
    scaled = value * 10**precision
    if scaled.denominator != 1:
        return Err(f"{value} is not representable at precision {precision}")
    n = int(scaled)
    sign = "-" if n < 0 else ""
    digits = str(abs(n)).rjust(precision + 1, "0")
    whole, frac = digits[: len(digits) - precision], digits[len(digits) - precision :]
    frac = frac.rstrip("0")
    return Ok(f"{sign}{whole}.{frac}" if frac else f"{sign}{whole}")


def digest_plan(plan: SheetPlan) -> str:
    """Return `blake3:<hex>` of the canonical text of `plan`."""
    text = "\n".join(f"{c.cell}\t{c.alias or ''}\t{c.content}" for c in plan.cells)
    return "blake3:" + blake3(text.encode("utf-8")).hexdigest()


def _is_dimensionless(value: ir.Value) -> bool:
    """True for Counts and Ratios: no dimension exponents and no angle."""
    dims = value.dim.model_dump()
    dims.pop("currency", None)
    return value.angle == 0 and not any(dims.values()) and not value.dim.currency


def _point_fraction(
    member: ir.Member, value: ir.Value, point: str, unit: str
) -> Result[Fraction, Diagnostic]:
    """Resolve the emit point of `value` as an exact rational in `unit`."""
    path = member.path.root
    if not value.exact:
        return Err(
            _diag("Q004", f"{path}: inexact value cannot be emitted (Q006)", path)
        )
    quantity = Quantity._from_ir(value.model_dump_json(exclude_none=True))
    dimensionless = _is_dimensionless(value)
    if dimensionless:
        # No unit reads a dimensionless quantity back; borrow mm and drop it again.
        quantity = quantity * Quantity._parse("1mm")
        unit = "mm"
    try:
        if point in ("mmc", "lmc"):
            if member.role not in ("hole", "shaft"):
                return Err(
                    _diag(
                        "E010",
                        f"{path}: emit point {point} needs a hole or shaft role",
                        path,
                    )
                )
            quantity = quantity.with_role(member.role)
        picked = getattr(quantity, point)
        exact = picked.to_exact(unit)
    except QuantityError as e:
        return Err(_diag("Q004", f"{path}: {e}", path))
    if exact is None:
        return Err(
            _diag("Q004", f"{path}: value is not exactly representable in {unit}", path)
        )
    return Ok(exact)


def _citation_text(cite: ir.CiteRef | None) -> str:
    """Display text of a citation: its text, else the key plus its locator."""
    if cite is None:
        return ""
    if cite.text:
        return cite.text
    key = cite.cite or (cite.builtin.value if cite.builtin else "")
    return f"{key} {cite.locator}".strip() if cite.locator else key


def _topological(
    paths: list[str], members: dict[str, ir.Member]
) -> Result[list[str], str]:
    """Order `paths` so dependencies precede dependents; smallest ready path first.

    Dependencies are followed transitively through unbound members (same rule as
    the sw-equations emitter, emitters/sw-equations.md 2).
    """
    wanted = set(paths)

    def bound_deps(path: str) -> set[str]:
        found: set[str] = set()
        seen: set[str] = set()
        stack = [d.root for d in members[path].deps]
        while stack:
            dep = stack.pop()
            if dep in seen:
                continue
            seen.add(dep)
            if dep in wanted:
                found.add(dep)
            elif dep in members:
                stack.extend(d.root for d in members[dep].deps)
        found.discard(path)
        return found

    deps = {p: bound_deps(p) for p in paths}
    indegree = {p: len(deps[p]) for p in paths}
    dependents: dict[str, list[str]] = {p: [] for p in paths}
    for p, ds in deps.items():
        for d in ds:
            dependents[d].append(p)
    ready = [p for p, n in indegree.items() if n == 0]
    heapq.heapify(ready)
    ordered: list[str] = []
    while ready:
        p = heapq.heappop(ready)
        ordered.append(p)
        for q in dependents[p]:
            indegree[q] -= 1
            if indegree[q] == 0:
                heapq.heappush(ready, q)
    if len(ordered) != len(paths):
        stuck = sorted(set(paths) - set(ordered))
        return Err(f"dependency cycle among bound paths: {', '.join(stuck)}")
    return Ok(ordered)


def _member_path(binding_path: str) -> str:
    """Map a `[*]` binding path to instance 0 (adapters.md 3)."""
    return binding_path.replace("[*]", "[0]")


def _alias_problem(alias: str) -> str | None:
    """Why `alias` cannot be a Spreadsheet alias, or None when it can."""
    if not _ALIAS_RE.match(alias):
        return f"alias {alias!r} is not a valid Spreadsheet alias"
    if _CELL_ADDRESS_RE.match(alias):
        return f"alias {alias!r} collides with a cell address"
    return None


class _Builder:
    """Accumulates rows of the plan and reports the first configuration error."""

    def __init__(self, policy: EmitPolicy, design: ir.Project) -> None:
        self.policy = policy
        self.rows: list[tuple[str, str, str, str]] = []  # path, alias, content, note
        self.diags: list[Diagnostic] = []
        self.members = {m.path.root: m for c in design.components for m in c.members}
        self.written: set[str] = set()


def _value_text(
    b: _Builder, member: ir.Member, locator: ir.Locator
) -> Result[tuple[str, str], Diagnostic]:
    """Return (content, compact text) of a member's value for one locator."""
    path = member.path.root
    value = member.value
    assert value is not None
    unit = locator.native_unit or b.policy.native_unit
    point = member.attrs.get("emit", b.policy.point)
    dimensionless = _is_dimensionless(value)
    match _point_fraction(member, value, point, unit):
        case Err(d):
            return Err(d)
        case Ok(frac):
            pass
    if locator.sign == ir.Sign.integer__1 and frac != 0:
        frac = -frac
    match to_decimal(frac, b.policy.precision):
        case Err(why):
            return Err(_diag("Q004", f"{path}: {why}", path))
        case Ok(dec):
            pass
    if dimensionless:
        return Ok((dec, dec))
    return Ok((f"={dec} {unit}", f"{dec}{unit}"))


def _note(b: _Builder, member: ir.Member, compact: str) -> str:
    """Trailing note: authored literal if different, emit point, citation."""
    parts: list[str] = []
    value = member.value
    if value is not None and value.authored.replace(" ", "") != compact:
        parts.append(value.authored)
    point = member.attrs.get("emit", b.policy.point)
    if point != "nominal":
        parts.append(f"emit={point}")
    cite = _citation_text(member.citation)
    if cite:
        parts.append(cite)
    return "; ".join(parts)


def _first_global(binding: ir.Binding | None) -> str | None:
    """The first value-mode global locator name of a binding, if any."""
    if binding is None:
        return None
    for loc in binding.locators:
        if loc.kind == ir.LocatorKind.global_ and loc.for_ is None:
            return loc.locator
    return None


def _emit_path(
    b: _Builder,
    binding: ir.Binding,
    by_path: dict[str, ir.Binding],
) -> Result[None, Diagnostic]:
    """Append the rows of one bound path: globals first, then dimensions."""
    path = _member_path(binding.path.root)
    member = b.members[path]
    if member.value is None:
        _log.info("skipping %s: no value to emit", path)
        return Ok(None)
    locators = [loc for loc in binding.locators if loc.for_ is None]
    ordered = [loc for loc in locators if loc.kind == ir.LocatorKind.global_] + [
        loc for loc in locators if loc.kind == ir.LocatorKind.dimension
    ]
    for loc in locators:
        if loc.kind not in (ir.LocatorKind.global_, ir.LocatorKind.dimension):
            _log.info(
                "%s: %s locator %r is not written by emit", path, loc.kind, loc.locator
            )
    created = False
    for loc in ordered:
        if loc.mode == ir.LocatorMode.ref and loc.kind == ir.LocatorKind.dimension:
            target = loc.ref_to or _first_global(binding)
            if target is None and path in b.policy.alias_of:
                target = _first_global(by_path.get(b.policy.alias_of[path]))
                if target is None:
                    return Err(
                        _diag(
                            "R015",
                            f"{path}: alias_of referent "
                            f"{b.policy.alias_of[path]} has no global locator",
                            path,
                        )
                    )
            if target is None:
                name = b.policy.names.get(path)
                if name is None:
                    return Err(
                        _diag(
                            "R015",
                            f"{path}: ref locator needs a created global name "
                            "(policy.names)",
                            path,
                        )
                    )
                if not created:
                    match _value_text(b, member, loc):
                        case Err(d):
                            return Err(d)
                        case Ok((content, compact)):
                            b.rows.append(
                                (path, name, content, _note(b, member, compact))
                            )
                    created = True
                target = name
            b.rows.append((path, loc.locator, f"={target}", ""))
            continue
        match _value_text(b, member, loc):
            case Err(d):
                return Err(d)
            case Ok((content, compact)):
                b.rows.append((path, loc.locator, content, _note(b, member, compact)))
    return Ok(None)


def build_plan(
    design: ir.Project,
    bindings: list[ir.Binding],
    policy: EmitPolicy,
    version: str,
) -> Result[SheetPlan, list[Diagnostic]]:
    """Plan the `cadetta` Spreadsheet for `bindings`; diagnostics withhold the file."""
    assert design.digest is not None, "callers reject digestless designs"
    b = _Builder(policy, design)
    by_path = {_member_path(x.path.root): x for x in bindings}
    wanted = sorted(by_path)
    if policy.paths is not None:
        wanted = [p for p in wanted if p in set(policy.paths)]
    missing = [p for p in wanted if p not in b.members]
    if missing:
        _log.error("bound paths absent from the IR: %s", missing)
        return Err(
            [_diag("R005", f"{p}: bound path not in the design", p) for p in missing]
        )
    match _topological(wanted, b.members):
        case Err(why):
            return Err([_diag("D003", why)])
        case Ok(order):
            pass
    errors: list[Diagnostic] = []
    for p in order:
        match _emit_path(b, by_path[p], by_path):
            case Err(d):
                _log.error("%s %s", d.code, d.message)
                errors.append(d)
            case Ok(_):
                pass
    seen: dict[str, str] = {}
    for path, alias, _content, _note_text in b.rows:
        problem = _alias_problem(alias)
        if problem is not None:
            errors.append(_diag("R015", f"{path}: {problem}", path))
        elif alias in seen:
            errors.append(
                _diag(
                    "R013",
                    f"{path}: alias {alias!r} already written for {seen[alias]}",
                    path,
                )
            )
        seen.setdefault(alias, path)
    if errors:
        return Err(errors)
    bdigest = (
        "blake3:"
        + blake3(
            json.dumps(
                [
                    x.model_dump(mode="json", by_alias=True, exclude_none=True)
                    for x in bindings
                ],
                sort_keys=True,
            ).encode("utf-8")
        ).hexdigest()
    )
    cells = [
        CellWrite(cell="A1", content=f"cadetta emit freecad {version}"),
        CellWrite(cell="B1", content=f"ir=blake3:{design.digest.root}"),
        CellWrite(cell="C1", content=f"bindings={bdigest}"),
    ]
    for n, (path, alias, content, note) in enumerate(b.rows, start=2):
        cells.append(CellWrite(cell=f"A{n}", content=path))
        cells.append(CellWrite(cell=f"B{n}", content=content, alias=alias))
        if note:
            cells.append(CellWrite(cell=f"C{n}", content=note))
    _log.info("planned %d cells for %d paths", len(cells), len(order))
    return Ok(SheetPlan(cells=cells))
