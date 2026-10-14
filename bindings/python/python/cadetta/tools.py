"""Module-level helpers that need no project: `units` and `fmt` (bindings.md 3.2)."""

# frob:ticket 01M4SWP8BCQ00WCDJ5FQ2QMDX9

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

from pydantic import ValidationError
from typani import Err, Ok, Result

from cadetta.logging import get_logger
from cadetta.models import units as units_model
from cadetta.project import ProjectError

__all__ = ["format_source", "units"]

_log = get_logger(__name__)

# CLI exit codes whose stdout is a result: 0 clean, 1 findings.
_OK_EXITS = (0, 1)
_FMT_PACKAGE = '[package]\nname = "fmt.scratch"\nversion = "0.0.0"\n'


def _cadetta(argv: list[str], cwd: Path | None = None) -> Result[str, ProjectError]:
    """Run the cadetta CLI and return stdout; exit 2 or worse is `ProjectError`."""
    full = [sys.executable, "-m", "cadetta", *argv]
    _log.debug("cli: %s", full)
    done = subprocess.run(full, capture_output=True, text=True, check=False, cwd=cwd)
    if done.returncode not in _OK_EXITS:
        _log.error("cadetta %s exited %d", argv[-1], done.returncode)
        return Err(
            ProjectError(
                message=done.stderr.strip() or f"{argv[-1]} exited {done.returncode}"
            )
        )
    return Ok(done.stdout)


def units(kind: str | None = None) -> Result[units_model.UnitsOutput, ProjectError]:
    """Built-in kinds with their exact units (`units [KIND]`); all kinds when None."""
    verb = ["--format", "json", "units"]
    if kind is not None:
        verb.append(kind)
    out = _cadetta(verb)
    if isinstance(out, Err):
        return out
    try:
        return Ok(units_model.UnitsOutput.model_validate(json.loads(out.unwrap())))
    except (json.JSONDecodeError, ValidationError) as exc:
        _log.error("unreadable units output: %s", exc)
        return Err(ProjectError(message=f"unreadable units output: {exc}"))


def format_source(text: str) -> Result[str, ProjectError]:
    """Canonical formatting of cadetta source (`fmt`); Err when it does not parse."""
    with tempfile.TemporaryDirectory(prefix="cadetta-fmt-") as tmp:
        root = Path(tmp)
        (root / "src").mkdir()
        (root / "cadetta.toml").write_text(_FMT_PACKAGE, encoding="utf-8")
        lib = root / "src" / "lib.cdt"
        lib.write_bytes(text.encode("utf-8"))
        ran = _cadetta(["--cwd", str(root), "fmt"], cwd=root)
        if isinstance(ran, Err):
            return ran
        formatted = lib.read_bytes().decode("utf-8")
    _log.debug("formatted %d chars to %d", len(text), len(formatted))
    return Ok(formatted)
