"""Open a project and read its evaluated design (bindings.md 3.1, 3.2)."""

# frob:ticket 01M4SV2NMPSM42YZT9MXT4YVMW

from __future__ import annotations

import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

from pydantic import BaseModel, ConfigDict, ValidationError
from typani import Err, Ok, Result

from cadetta._core import Quantity
from cadetta.logging import get_logger
from cadetta.models import ir

__all__ = [
    "CadettaError",
    "Design",
    "Member",
    "PathNotFound",
    "Project",
    "ProjectError",
]

_log = get_logger(__name__)

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
    def citation(self) -> ir.Citation | None:
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
        _log.debug("design indexed: %d members", len(self._members))

    def get(self, path: str) -> Result[Member, PathNotFound]:
        """Look up a member by instance path; Err(PathNotFound) when absent."""
        found = self._members.get(path)
        if found is None:
            _log.debug("path not found: %s", path)
            return Err(PathNotFound(message=f"no member at `{path}`", path=path))
        return Ok(Member(found, found.value))

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

    def evaluate(self) -> Result[Design, ProjectError]:
        """Run `ir` and wrap the design IR as generated pydantic models."""
        argv = [sys.executable, "-m", "cadetta", "--format", "json"]
        argv += ["--cwd", str(self.root)]
        if self.config is not None:
            argv += ["--config", str(self.config)]
        argv.append("ir")
        _log.debug("evaluate: %s", argv)
        done = subprocess.run(argv, capture_output=True, text=True, check=False)
        if done.stderr:
            _log.debug("cadetta ir stderr: %s", done.stderr.strip()[:2000])
        if done.returncode not in (0, 1):
            return Err(
                ProjectError(
                    message=done.stderr.strip() or f"ir exited {done.returncode}"
                )
            )
        try:
            model = ir.Project.model_validate(json.loads(done.stdout))
        except (json.JSONDecodeError, ValidationError) as exc:
            return Err(ProjectError(message=f"unreadable IR: {exc}"))
        return Ok(Design(model))
