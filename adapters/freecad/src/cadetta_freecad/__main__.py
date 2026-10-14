"""Command-line entry point: `cadetta-freecad [--version]`."""

from __future__ import annotations

import argparse
import sys

from cadetta.logging import get_logger, use_protocol_logging

from cadetta_freecad import __version__
from cadetta_freecad.adapter import FreeCadAdapter

# stdout is the JSON-RPC channel, so every log record goes to stderr.
use_protocol_logging()
_log = get_logger(__name__)


def _build_parser() -> argparse.ArgumentParser:
    """Build the argument parser."""
    parser = argparse.ArgumentParser(prog="cadetta-freecad")
    parser.add_argument("--version", action="store_true", help="print the version")
    return parser


def main(argv: list[str] | None = None) -> int:
    """Serve JSON-RPC on stdio, or print the version; returns the process exit code."""
    args = _build_parser().parse_args(argv)
    _log.debug("parsed arguments: %s", args)
    if args.version:
        # Intentional command output, not logging (stdout is the protocol channel).
        sys.stdout.write(f"cadetta-freecad {__version__}\n")
        return 0
    _log.info("serving the adapter protocol on stdio")
    return FreeCadAdapter.run()


if __name__ == "__main__":
    sys.exit(main())
