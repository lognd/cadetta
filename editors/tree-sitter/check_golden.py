# /// script
# requires-python = ">=3.11"
# ///
"""Fail when tree-sitter parses any tests/golden/*/src/*.cdt with ERROR or MISSING nodes.

Run from anywhere: `uv run editors/tree-sitter/check_golden.py [path/to/tree-sitter]`.
Needs the tree-sitter CLI and a prior `tree-sitter generate` in this directory.
"""

from __future__ import annotations

import logging
import subprocess
import sys
from pathlib import Path

log = logging.getLogger("cadetta.editors.tree_sitter.check_golden")

HERE = Path(__file__).resolve().parent
GOLDEN = HERE.parents[1] / "tests" / "golden"


def parse_errors(cli: str, path: Path) -> bool:
    """True when tree-sitter reports a syntax error (or fails) for the file."""
    proc = subprocess.run(
        [cli, "parse", str(path)], cwd=HERE, capture_output=True, text=True, check=False
    )
    if proc.returncode != 0:
        log.error("%s: tree-sitter exited %d: %s", path, proc.returncode, proc.stderr)
        return True
    bad = "ERROR" in proc.stdout or "MISSING" in proc.stdout
    if bad:
        log.error("%s: parse tree has ERROR/MISSING nodes", path)
    return bad


def main(argv: list[str]) -> int:
    """Parse every golden source; exit 1 if any has error nodes or none were found."""
    logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
    cli = argv[1] if len(argv) > 1 else "tree-sitter"
    files = sorted(GOLDEN.glob("*/src/*.cdt"))
    if not files:
        log.error("no golden sources under %s", GOLDEN)
        return 1
    failed = [f for f in files if parse_errors(cli, f)]
    log.info("%d files parsed, %d with errors", len(files), len(failed))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
