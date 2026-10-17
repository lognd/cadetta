"""Command-line entry point: `cadetta-fusion [--version]`."""

# frob:ticket 01M555TVRF47KEJ406NPQMSJDX

from __future__ import annotations

import argparse
import sys

from cadetta.logging import get_logger, use_protocol_logging

from cadetta_fusion import __version__
from cadetta_fusion.adapter import FusionAdapter

# stdout is the JSON-RPC channel, so every log record goes to stderr.
use_protocol_logging()
_log = get_logger(__name__)


def _build_parser() -> argparse.ArgumentParser:
    """Build the argument parser."""
    parser = argparse.ArgumentParser(prog="cadetta-fusion")
    parser.add_argument("--version", action="store_true", help="print the version")
    return parser


def main(argv: list[str] | None = None) -> int:
    """Serve JSON-RPC on stdio, or print the version; returns the exit code."""
    args = _build_parser().parse_args(argv)
    if args.version:
        # Intentional command output, not logging (stdout is the protocol channel).
        sys.stdout.write(f"cadetta-fusion {__version__}\n")
        return 0
    _log.info("serving the adapter protocol on stdio")
    return FusionAdapter.run()


if __name__ == "__main__":
    sys.exit(main())
