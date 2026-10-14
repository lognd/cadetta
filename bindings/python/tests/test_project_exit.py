"""Project._model maps CLI exits through result.report_from_exit (bindings.md 3)."""

# frob:ticket 01M4WE56PSQJHJRG3BHYA5SB8A

import subprocess
from pathlib import Path

import pytest
from cadetta import Project, ProjectError
from cadetta.models import names
from typani import Err, Ok


def make_pkg(root: Path) -> Project:
    """Write a minimal package and open it."""
    (root / "src").mkdir(parents=True)
    (root / "cadetta.toml").write_text(
        '[package]\nname = "acme.rover"\nversion = "1.2.3"\n', encoding="utf-8"
    )
    (root / "src" / "lib.cdt").write_text(
        "model Axle {\n  od: Length = 8mm\n}\n", encoding="utf-8"
    )
    return Project.open(root).unwrap()


def _stub(project: Project, monkeypatch: pytest.MonkeyPatch, code: int) -> None:
    """Make the next CLI run exit `code` with a real `names` report on stdout."""
    real = project._spawn(["names"], global_format=True)

    def fake(
        verb: list[str], *, global_format: bool
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(verb, code, real.stdout, "boom")

    monkeypatch.setattr(project, "_spawn", fake)


# frob:tests bindings/python/python/cadetta/project.py::Project._model kind=unit
def test_model_exit_one_is_ok_report(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Exit 1 (error diagnostics) still yields Ok(report)."""
    project = make_pkg(tmp_path)
    _stub(project, monkeypatch, 1)
    got = project._model(names.NamesReport, ["names"])
    assert isinstance(got, Ok)


# frob:tests bindings/python/python/cadetta/project.py::Project._model kind=unit
def test_model_exit_two_is_err(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Exit 2 (usage) is Err(ProjectError) carrying stderr."""
    project = make_pkg(tmp_path)
    _stub(project, monkeypatch, 2)
    got = project._model(names.NamesReport, ["names"])
    assert isinstance(got, Err) and isinstance(got.unwrap_err(), ProjectError)
    assert got.unwrap_err().message == "boom"
