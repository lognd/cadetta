"""Smoke tests: the extension imports and the console script runs the Rust CLI."""

import subprocess
import sys

import cadetta
from cadetta._core import run_cli


# frob:tests bindings/python/python/cadetta/_core.pyi::core_version kind=unit
def test_core_version_names_cadetta():
    assert cadetta.core_version().startswith("cadetta ")


# frob:tests bindings/python/python/cadetta/__main__.py::main kind=unit
def test_console_script_prints_version():
    done = subprocess.run(
        [sys.executable, "-m", "cadetta", "--version"], capture_output=True, text=True
    )
    assert done.returncode == 0
    assert done.stdout == cadetta.core_version() + "\n"


# frob:tests bindings/python/python/cadetta/_core.pyi::run_cli kind=unit
def test_unknown_command_is_usage_error():
    assert run_cli(["no-such-verb-xyzzy"]) == 2
