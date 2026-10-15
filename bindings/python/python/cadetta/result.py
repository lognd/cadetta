"""Result contract: findings are Ok; only I/O, config or usage is Err (bindings 3)."""

# frob:ticket 01M4SWP8D2254TD7A8558R6ZY3

from __future__ import annotations

import json
from collections.abc import Callable
from typing import TypeVar

from pydantic import BaseModel, ValidationError
from typani import Err, Ok, Result

from cadetta.logging import get_logger

__all__ = ["EXIT_FINDINGS", "EXIT_OK", "EXIT_USAGE", "JSON_EXITS", "report_from_exit"]

_log = get_logger(__name__)

_M = TypeVar("_M", bound=BaseModel)
_E = TypeVar("_E")

EXIT_OK = 0
EXIT_FINDINGS = 1
EXIT_USAGE = 2
# Exit codes whose stdout is a report: clean (0) or error diagnostics (1).
JSON_EXITS = (EXIT_OK, EXIT_FINDINGS)


def report_from_exit(
    model: type[_M],
    verb: str,
    returncode: int,
    stdout: str,
    stderr: str,
    make_error: Callable[[str], _E],
) -> Result[_M, _E]:
    """Map a CLI run to a Result: exit 0 or 1 is Ok(report); anything else is Err."""
    if returncode not in JSON_EXITS:
        _log.error("cadetta %s exited %d", verb, returncode)
        return Err(make_error(stderr.strip() or f"{verb} exited {returncode}"))
    if returncode == EXIT_FINDINGS:
        _log.debug("cadetta %s reported findings (exit 1): Ok(report)", verb)
    try:
        return Ok(model.model_validate(json.loads(stdout)))
    except (json.JSONDecodeError, ValidationError) as exc:
        _log.error("unreadable %s output: %s", verb, exc)
        return Err(make_error(f"unreadable {verb} output: {exc}"))
