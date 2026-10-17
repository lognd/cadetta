"""Open a project and read its evaluated design (bindings.md 3.1, 3.2)."""

# frob:ticket 01M4SV2NMPSM42YZT9MXT4YVMW

from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import TypeVar

from pydantic import BaseModel, ConfigDict
from typani import Err, Ok, Result

from cadetta import project_ops, queries
from cadetta._core import Quantity
from cadetta.logging import get_logger
from cadetta.models import (
    ack,
    cite_list,
    compare,
    emit,
    explain,
    impact,
    import_,
    ir,
    names,
    obligations,
    swap,
    update,
    why,
)
from cadetta.models import bind as bind_model
from cadetta.models import impact_since as since_model
from cadetta.result import JSON_EXITS, report_from_exit

__all__ = [
    "CadettaError",
    "Design",
    "Member",
    "PathNotFound",
    "Project",
    "ProjectError",
]

_log = get_logger(__name__)

_M = TypeVar("_M", bound=BaseModel)

# `bind` keyword fields to CLI flags where the name is not just `--<name with hyphens>`.
_BIND_FLAGS = {"for_alternate": "--for"}
_BIND_FIELDS = frozenset(
    {"mode", "native_unit", "for_alternate", "config", "sign", "component", "tolerance"}
)

CONFIG_FILE = "cadetta.toml"
DEFAULT_VERSION = "0.0.0"
DEFAULT_SRC = "src"


class CadettaError(BaseModel):
    """Base of the errors a Result carries: I/O, configuration or usage only."""

    model_config = ConfigDict(frozen=True)

    message: str


class ProjectError(CadettaError):
    """No project at the path, a bad `cadetta.toml`, or the CLI refused (exit 2)."""


class PathNotFound(CadettaError):
    """A design has no member at this instance path."""

    path: str


class Member:
    """One IR member: the generated model plus its value as a `Quantity`."""

    def __init__(self, model: ir.Member, value: ir.Value | None) -> None:
        """Wrap the generated `model`; `value` is the (possibly variant) value."""
        self.model = model
        self._value = value

    @property
    def path(self) -> str:
        """The canonical instance path of the member."""
        return self.model.path.root

    @property
    def kind(self) -> str:
        """The IR member kind (`param`, `let`, ...), as in the CLI's JSON."""
        return self.model.kind.value

    @property
    def value(self) -> Quantity | None:
        """The evaluated value as a Quantity; None for a bound-only member."""
        if self._value is None:
            return None
        text = self._value.model_dump_json(exclude_none=True)
        return Quantity._from_ir(text)

    @property
    def citation(self) -> ir.CiteRef | None:
        """The citation attached to the member, if any."""
        return self.model.citation

    def variant(self, slot: str, alternate: str) -> Member | None:
        """The member under variant `slot=alternate` (language.md 3.2), else None."""
        found = (self.model.variants or {}).get(f"{slot}={alternate}")
        if found is None:
            _log.debug("no variant %s=%s on %s", slot, alternate, self.path)
            return None
        return Member(self.model, found)

    def __repr__(self) -> str:
        """Short debug form."""
        return f"Member({self.path!r})"


class Design:
    """The evaluated design IR with its members indexed by instance path."""

    def __init__(self, model: ir.Project) -> None:
        """Index every member of every component of `model` by path."""
        self.model = model
        self._members: dict[str, ir.Member] = {
            m.path.root: m for c in model.components for m in c.members
        }
        self._citations: dict[str, ir.Citation] = {c.path: c for c in model.citations}
        _log.debug(
            "design indexed: %d members, %d citations",
            len(self._members),
            len(self._citations),
        )

    def get(self, path: str) -> Result[Member, PathNotFound]:
        """Look up a member by instance path; Err(PathNotFound) when absent."""
        found = self._members.get(path)
        if found is None:
            _log.debug("path not found: %s", path)
            return Err(PathNotFound(message=f"no member at `{path}`", path=path))
        return Ok(Member(found, found.value))

    def citation(self, path: str) -> Result[ir.Citation, PathNotFound]:
        """Look up a citation by path (a member's `citation.cite`); Err when absent."""
        found = self._citations.get(path)
        if found is None:
            _log.debug("citation not found: %s", path)
            return Err(PathNotFound(message=f"no citation at `{path}`", path=path))
        return Ok(found)

    def __getattr__(self, name: str):  # noqa: ANN204
        """Read-through to the generated IR model's fields."""
        return getattr(self.model, name)


def _normalise_name(dir_name: str) -> str:
    """The CLI's package-name rule for a missing `[package] name`."""
    out = re.sub(r"[^A-Za-z0-9_.]+", "_", dir_name).lower().strip("_.")
    return out if out[:1].isalpha() else f"pkg_{out}"


def _read_table(config: Path) -> Result[dict, ProjectError]:
    """Parse a `cadetta.toml`; Err on I/O or TOML errors."""
    try:
        return Ok(tomllib.loads(config.read_text(encoding="utf-8")))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        return Err(ProjectError(message=f"cannot read {config}: {exc}"))


def _discover(start: Path) -> Result[tuple[Path, dict], ProjectError]:
    """Walk up from `start` to the nearest `cadetta.toml` with a project table."""
    for directory in (start, *start.parents):
        config = directory / CONFIG_FILE
        if not config.is_file():
            continue
        table = _read_table(config)
        if isinstance(table, Err):
            return table
        data = table.unwrap()
        if "package" in data or "workspace" in data:
            _log.debug("project root found: %s", directory)
            return Ok((directory, data))
    return Err(ProjectError(message=f"no cadetta.toml project found from {start}"))


def _module_paths(root: Path, package: str, src: str) -> list[str]:
    """Sorted canonical module paths under `src`, with implicit parents."""
    found: set[str] = set()
    base = root / src
    for file in base.rglob("*.cdt"):
        parts = list(file.relative_to(base).with_suffix("").parts)
        if parts == ["lib"]:
            parts = []
        for i in range(len(parts) + 1):
            found.add("::".join([package, *parts[:i]]))
    return sorted(found)


class Project:
    """A snapshot of a package or workspace read at `open` (bindings.md 3.2)."""

    def __init__(self, root: Path, table: dict, config: Path | None) -> None:
        """Hold the discovered root and its parsed `cadetta.toml`."""
        self.root = root
        self.config = config
        self._table = table

    @classmethod
    def open(
        cls, path: str | Path | None = None, config: str | Path | None = None
    ) -> Result[Project, ProjectError]:
        """Discover the project root (config-cli.md 1.6) and open it."""
        if config is not None:
            start = Path(config).resolve().parent
        else:
            start = Path(path if path is not None else Path.cwd()).resolve()
        if not start.exists():
            return Err(ProjectError(message=f"no such path: {start}"))
        found = _discover(start)
        if isinstance(found, Err):
            return found
        root, table = found.unwrap()
        _log.info("project opened: %s", root)
        return Ok(cls(root, table, Path(config).resolve() if config else None))

    @property
    def is_workspace(self) -> bool:
        """True when the root carries a `[workspace]` table."""
        return "workspace" in self._table

    @property
    def package(self) -> ir.PackageId | None:
        """The `[package]` name and version; None at a workspace root."""
        pkg = self._table.get("package")
        if pkg is None:
            return None
        name = pkg.get("name") or _normalise_name(self.root.name)
        return ir.PackageId(name=name, version=pkg.get("version", DEFAULT_VERSION))

    @property
    def modules(self) -> list[str]:
        """Sorted module paths under `src` (packages.md 2); [] at a workspace root."""
        package = self.package
        if package is None:
            return []
        src = self._table["package"].get("src", DEFAULT_SRC)
        return _module_paths(self.root, package.name, src)

    def reload(self) -> Result[Project, ProjectError]:
        """Re-read `cadetta.toml` from the root, replacing the snapshot in place."""
        found = _discover(self.root)
        if isinstance(found, Err):
            _log.error("reload failed: %s", found.unwrap_err().message)
            return found
        self.root, self._table = found.unwrap()
        _log.info("project reloaded: %s", self.root)
        return Ok(self)

    def explain(self, code: str) -> Result[explain.ExplainOutput, ProjectError]:
        """One lint catalog entry (`explain CODE`)."""
        return self._model(explain.ExplainOutput, ["explain", code])

    def _spawn(
        self, verb: list[str], *, global_format: bool
    ) -> subprocess.CompletedProcess[str]:
        """Run a CLI verb at this project and capture its output."""
        argv = [sys.executable, "-m", "cadetta"]
        if global_format:
            argv += ["--format", "json"]
        argv += ["--cwd", str(self.root)]
        if self.config is not None:
            argv += ["--config", str(self.config)]
        argv += verb
        _log.debug("cli: %s", argv)
        done = subprocess.run(argv, capture_output=True, text=True, check=False)
        if done.stderr:
            _log.debug("cadetta %s stderr: %s", verb[0], done.stderr.strip()[:2000])
        return done

    def _run(
        self, verb: list[str], *, global_format: bool = True
    ) -> Result[str, ProjectError]:
        """Run a CLI verb and return its text stdout; exit 0 and 1 are results.

        Any other exit is `ProjectError`.
        """
        done = self._spawn(verb, global_format=global_format)
        if done.returncode not in JSON_EXITS:
            _log.error("cadetta %s exited %d", verb[0], done.returncode)
            return Err(
                ProjectError(
                    message=done.stderr.strip() or f"{verb[0]} exited {done.returncode}"
                )
            )
        return Ok(done.stdout)

    def _model(
        self, model: type[_M], verb: list[str], *, global_format: bool = True
    ) -> Result[_M, ProjectError]:
        """Run a verb and map its exit and JSON stdout via `report_from_exit`."""
        done = self._spawn(verb, global_format=global_format)
        return report_from_exit(
            model,
            verb[0],
            done.returncode,
            done.stdout,
            done.stderr,
            lambda message: ProjectError(message=message),
        )

    def cite_list(
        self, format: str = "json", module: str | None = None
    ) -> Result[cite_list.CadettaCiteListFormatJson | str, ProjectError]:
        """The numbered reference list (`cite list`); json is a model, else text."""
        verb = ["cite", "list", "--format", format]
        if module is not None:
            verb += ["--module", module]
        if format != "json":
            return self._run(verb, global_format=False)
        return self._model(
            cite_list.CadettaCiteListFormatJson, verb, global_format=False
        )

    def names(
        self, adapter: str | None = None, module: str | None = None
    ) -> Result[names.NamesReport, ProjectError]:
        """Every in-scope bindable path per adapter with its binding state (`names`)."""
        verb = ["names"]
        if adapter is not None:
            verb.append(adapter)
        if module is not None:
            verb += ["--module", module]
        return self._model(names.NamesReport, verb)

    def bind(
        self, path: str, adapter: str, locator: str, kind: str, **fields: str
    ) -> Result[bind_model.BindReport, ProjectError]:
        """Add one explicit locator to the adapter's bindings file (`bind`).

        `fields` are the optional flags: mode, native_unit, for_alternate, config,
        sign, component, tolerance.
        """
        unknown = sorted(set(fields) - _BIND_FIELDS)
        if unknown:
            return Err(
                ProjectError(message=f"unknown bind fields: {', '.join(unknown)}")
            )
        verb = ["bind", path, "--adapter", adapter, "--locator", locator]
        verb += ["--kind", kind]
        for name, value in fields.items():
            flag = _BIND_FLAGS.get(name, "--" + name.replace("_", "-"))
            verb += [flag, str(value)]
        return self._model(bind_model.BindReport, verb)

    def unbind(
        self, path: str, adapter: str, locator: str
    ) -> Result[bind_model.BindReport, ProjectError]:
        """Remove one explicit locator from the adapter's bindings file."""
        verb = ["bind", "--remove", path, "--adapter", adapter, "--locator", locator]
        return self._model(bind_model.BindReport, verb)

    def evaluate(self) -> Result[Design, ProjectError]:
        """Run `ir` and wrap the design IR; Err at a workspace root (bindings 3.2)."""
        if self.package is None:
            _log.error("evaluate at workspace root %s: usage error", self.root)
            return Err(
                ProjectError(message="evaluate needs a package, not a workspace root")
            )
        model = self._model(ir.Project, ["ir"])
        if isinstance(model, Err):
            return model
        return Ok(Design(model.unwrap()))

    def check(
        self,
        release: bool = False,
        as_of: str | None = None,
        module: str | None = None,
    ) -> Result[queries.CheckReport, ProjectError]:
        """Diagnostics from `check`; error findings are still an Ok report."""
        return queries.check(self, release, as_of, module)

    def why(self, path: str) -> Result[why.Why, ProjectError]:
        """The derivation of one member (`why PATH`)."""
        return queries.why_path(self, path)

    def why_stale(
        self, obligation: str, role: str | None = None
    ) -> Result[why.Why, ProjectError]:
        """Why an obligation is stale (`why --stale`)."""
        return queries.why_stale(self, obligation, role)

    def impact(self, path: str) -> Result[impact.ImpactOutput, ProjectError]:
        """The downstream dependents of a member (`impact PATH`)."""
        return queries.impact_of(self, path)

    def impact_since(
        self, since: str | None = None
    ) -> Result[since_model.ImpactSince, ProjectError]:
        """Diff two evaluations: REV vs the working tree, or HEAD vs it when None."""
        return queries.impact_since(self, since)

    def impact_swap(self, slot: str, alternate: str) -> Result[swap.Swap, ProjectError]:
        """Preview swapping `slot` to `alternate` (`impact --swap`)."""
        return queries.impact_swap(self, slot, alternate)

    def swap(
        self, slot: str, alternate: str, reason: str, as_: str | None = None
    ) -> Result[swap.Swap, ProjectError]:
        """Apply a part swap with a recorded reason (`swap`)."""
        return queries.swap_part(self, slot, alternate, reason, as_)

    def compare(self, interface: str) -> Result[compare.Compare, ProjectError]:
        """The alternates of an interface side by side (`compare`)."""
        return queries.compare_interface(self, interface)

    def obligations(
        self,
        stale: bool = False,
        role: str | None = None,
        release: bool = False,
        as_of: str | None = None,
    ) -> Result[obligations.ObligationsOutput, ProjectError]:
        """The obligation list (`obligations`)."""
        return queries.obligations_of(self, stale, role, release, as_of)

    def update(self, name: str | None = None) -> Result[update.Update, ProjectError]:
        """Re-resolve dependencies and rewrite the lock (`update [NAME]`)."""
        return project_ops.update_lock(self, name)

    def solve(
        self, names: list[str] | None = None, variants: bool = False
    ) -> Result[project_ops.SolveReport, ProjectError]:
        """Run solvers and refresh result files (`solve`)."""
        return project_ops.solve_project(self, names, variants)

    def emit(
        self, adapters: list[str] | None = None
    ) -> Result[emit.EmitReport, ProjectError]:
        """Write the configured adapters' outputs (`emit`)."""
        return project_ops.emit_adapters(self, adapters)

    def sync(
        self,
        adapters: list[str] | None = None,
        dry_run: bool = False,
        only: str | None = None,
        force_design: bool = False,
        no_regen: bool = False,
        module: str | None = None,
    ) -> Result[project_ops.SyncReport, ProjectError]:
        """The three-way CAD sync loop (`sync`)."""
        return project_ops.sync_project(
            self, adapters, dry_run, only, force_design, no_regen, module
        )

    def import_(
        self,
        adapter: str,
        files: list[str],
        accept: bool = False,
        update: bool = False,
        write: bool = False,
    ) -> Result[import_.ImportReport, ProjectError]:
        """Propose, accept or diff an import from CAD files (`import`)."""
        return project_ops.import_adapter(self, adapter, files, accept, update, write)

    def ack(
        self,
        selector: str,
        role: str,
        reason: str | None = None,
        baseline: bool = False,
        release: bool = False,
        dry_run: bool = False,
        as_: str | None = None,
    ) -> Result[ack.AckOutput, ProjectError]:
        """Attest obligations at their current digests (`ack`)."""
        return project_ops.ack_obligations(
            self, selector, role, reason, baseline, release, dry_run, as_
        )
