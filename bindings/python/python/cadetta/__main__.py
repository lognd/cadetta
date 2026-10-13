"""The `cadetta` console script: hands argv to the Rust CLI."""

from __future__ import annotations

import sys

from cadetta._core import run_cli


def main() -> None:
    """Run the Rust command line and exit with its code."""
    sys.exit(run_cli(sys.argv[1:]))


if __name__ == "__main__":
    main()
