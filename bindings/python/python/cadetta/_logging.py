"""Bridge Rust tracing events to stdlib logging (docs/spec/bindings.md 3)."""

# frob:ticket 01M4SV2P2ZX7BMWKB98CJ14FY0

from __future__ import annotations

import logging
import sys
from pathlib import Path

from cadetta import _core

# Plain getLogger: get_logger would configure the root logger before typani imports.
_log = logging.getLogger(__name__)


def logger_name(target: str) -> str:
    """Map a Rust tracing target (`a::b`) to a dotted logger name (`a.b`)."""
    return target.replace("::", ".")


def emit(level: int, target: str, message: str) -> None:
    """Turn one tracing event into a logging record, keeping level and target."""
    logger = logging.getLogger(logger_name(target))
    if logger.isEnabledFor(level):
        logger.handle(
            logger.makeRecord(
                logger.name,
                level,
                "<rust>",
                0,
                message,
                (),
                None,
                extra={"target": target},
            )
        )


def install() -> bool:
    """Route Rust tracing events to logging; False if a subscriber already exists."""
    installed = _core.install_log_bridge(emit)
    _log.debug("rust tracing bridge installed=%s", installed)
    return installed


def running_as_cli() -> bool:
    """True for the `cadetta` script or `python -m cadetta` (Rust owns logging)."""
    if Path(sys.argv[0]).name in {"cadetta", "cadetta.exe"}:
        return True
    argv = sys.orig_argv
    return "-m" in argv and argv[argv.index("-m") + 1 : argv.index("-m") + 2] in (
        ["cadetta"],
        ["cadetta.__main__"],
    )


def install_unless_cli() -> bool:
    """Install the bridge for library use; the CLI keeps its own subscriber."""
    if running_as_cli():
        # No log line here: the CLI's stdout carries its output (or JSON).
        return False
    return install()
