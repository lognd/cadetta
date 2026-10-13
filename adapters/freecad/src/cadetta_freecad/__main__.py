"""Command-line entry point: `cadetta-freecad [--version]`."""

from __future__ import annotations

import argparse
import sys

from cadetta.logging import get_logger, use_protocol_logging

from cadetta_freecad import __version__

# stdout is the JSON-RPC channel, so every log record goes to stderr.
use_protocol_logging()
_log = get_logger(__name__)


def _build_parser() -> argparse.ArgumentParser:
    """Build the argument parser; protocol serving arrives with `initialize`."""
    parser = argparse.ArgumentParser(prog="cadetta-freecad")
    parser.add_argument("--version", action="store_true", help="print the version")
    return parser


def main(argv: list[str] | None = None) -> int:
    """Run the adapter; returns the process exit code."""
    args = _build_parser().parse_args(argv)
    _log.debug("parsed arguments: %s", args)
    if args.version:
        # Intentional command output, not logging (stdout is the protocol channel).
        sys.stdout.write(f"cadetta-freecad {__version__}\n")
        return 0
    _log.error("protocol serving is not implemented yet; only --version works")
    return 2


if __name__ == "__main__":
    sys.exit(main())
