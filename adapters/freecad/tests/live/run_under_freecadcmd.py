"""Run the live pytest suite inside freecadcmd's embedded interpreter.

Usage: CADETTA_SITE=<venv site-packages> freecadcmd run_under_freecadcmd.py

freecadcmd runs this file with the `FreeCAD` module importable. CADETTA_SITE
adds the venv holding pytest, cadetta and cadetta-freecad to sys.path. The
pytest exit code becomes the process exit code, which freecadcmd would
otherwise swallow.
"""

# frob:ticket 01M4SPWSQQNCC23KA25TCFVQ98

import logging
import os
import site
import sys
from pathlib import Path

_log = logging.getLogger("cadetta_freecad.live_runner")


def main() -> int:
    """Run pytest over the live tests and return its exit code."""
    logging.basicConfig(level=logging.INFO, stream=sys.stderr)
    extra = os.environ.get("CADETTA_SITE")
    if extra:
        _log.info("adding %s to sys.path", extra)
        # Front of the path: the system typing_extensions is too old for
        # pydantic_core and would otherwise shadow the venv's copy.
        sys.path.insert(0, extra)
        site.addsitedir(extra)
    import pytest

    here = Path(__file__).resolve().parent if "__file__" in globals() else Path.cwd()
    _log.info("running live tests in %s", here)
    return int(pytest.main(["-v", "-p", "no:cacheprovider", str(here)]))


code = main()
sys.stdout.flush()
sys.stderr.flush()
os._exit(code)
