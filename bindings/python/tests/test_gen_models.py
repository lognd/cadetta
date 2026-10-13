"""Tests for scripts/gen_models.py: the models match docs/schemas, drift is named."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

_SCRIPT = Path(__file__).resolve().parent.parent / "scripts" / "gen_models.py"


@pytest.fixture(scope="module")
def gen_models():
    """Load scripts/gen_models.py as a module (scripts/ is not a package)."""
    spec = importlib.util.spec_from_file_location("gen_models", _SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules["gen_models"] = module
    spec.loader.exec_module(module)
    return module


# frob:tests bindings/python/scripts/gen_models.py::main
def test_committed_models_are_not_stale(gen_models):
    """The committed models regenerate byte-for-byte; stale ones fail CI."""
    assert gen_models.main(["--check"]) == 0


# frob:tests bindings/python/scripts/gen_models.py::run
def test_check_names_stale_file(gen_models, tmp_path, monkeypatch):
    """A drifted committed model makes --check exit 1 and name the file."""
    stale = tmp_path / "ir.py"
    stale.write_text("# stale\n")
    monkeypatch.setattr(gen_models, "MODELS_DIR", tmp_path)
    monkeypatch.setattr(gen_models, "REPO_ROOT", tmp_path)
    assert gen_models.run(check=True).unwrap() == ["ir.py"]
    assert stale.read_text() == "# stale\n"
