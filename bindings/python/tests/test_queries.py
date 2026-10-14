"""Project query methods return the CLI's JSON as generated models (bindings.md 3.2)."""

# frob:ticket 01M4SV2NRTBH99MYAY6EWBDP66

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest
from cadetta import Project, ProjectError
from cadetta.models import compare, impact, obligations, swap, why
from cadetta.models.diag import Severity
from cadetta.queries import CheckReport
from typani import Err, Ok

SOURCE = """model Axle {
  od: Length = 8mm
  id: Length = od / 2
  require thin: id < 3mm
}
"""

OD = "acme.rover::Axle.od"
ID = "acme.rover::Axle.id"

SWAP = {
    "schema": "cadetta.swap/1",
    "swap": {
        "slot": "acme.rover::Rover/wheel",
        "alternate": "acme.shop::W2",
        "approved": True,
    },
    "verdict": "ok with warnings",
    "values_changed": [{"path": ID, "class": "derived", "old": "4mm", "new": "5mm"}],
    "verdicts_flipped": [
        {"path": "acme.rover::Axle.thin", "old": "pass", "new": "fail"}
    ],
    "suggested_edit": "part wheel: acme.shop::W2",
    "cad_resync": {"stale": [], "not_stale": [], "replace_component": []},
    "attestations_invalidated": [
        {"id": "A1", "role": "review", "covered_nodes_affected": 1}
    ],
    "commercial_deltas": {
        "assembly": "acme.rover::Rover",
        "deltas": [],
        "moq": None,
        "missing": [],
    },
}

COMPARE = {
    "schema": "cadetta.compare/1",
    "iface": "acme.rover::Wheel",
    "slots": ["acme.rover::Rover/wheel"],
    "columns": [
        {
            "component": "acme.shop::W2",
            "lifecycle": None,
            "slots": ["acme.rover::Rover/wheel"],
            "members": [{"name": "od", "value": "8mm", "bound": "<= 9mm"}],
            "commercial": [],
            "verdicts": [
                {"check": "fit", "slot": "acme.rover::Rover/wheel", "verdict": "pass"}
            ],
        }
    ],
}


@pytest.fixture
def project(tmp_path: Path) -> Project:
    """A minimal package with one violated requirement, opened."""
    (tmp_path / "src").mkdir()
    (tmp_path / "cadetta.toml").write_text(
        '[package]\nname = "acme.rover"\nversion = "1.2.3"\n', encoding="utf-8"
    )
    (tmp_path / "src" / "lib.cdt").write_text(SOURCE, encoding="utf-8")
    return Project.open(tmp_path).unwrap()


def _canned(
    project: Project, monkeypatch: pytest.MonkeyPatch, payload: dict
) -> list[list[str]]:
    """Make every CLI run succeed with `payload`; return the recorded verbs."""
    seen: list[list[str]] = []

    def fake(
        verb: list[str], *, global_format: bool
    ) -> subprocess.CompletedProcess[str]:
        seen.append(verb)
        return subprocess.CompletedProcess(verb, 0, json.dumps(payload), "")

    monkeypatch.setattr(project, "_spawn", fake)
    return seen


# frob:tests bindings/python/python/cadetta/project.py::Project.check kind=unit
# frob:tests bindings/python/python/cadetta/queries.py::check kind=unit
def test_check_diagnostics_are_typed_and_exit_one_is_ok(project: Project) -> None:
    """A violated requirement (exit 1) is Ok(report) with full diagnostics."""
    got = project.check()
    assert isinstance(got, Ok)
    report = got.unwrap()
    assert isinstance(report, CheckReport)
    err = next(d for d in report.diagnostics if d.code == "C001")
    assert err.severity == Severity.error
    assert err.path == "acme.rover::Axle.thin"
    assert err.location is not None and "start" in err.location.root.model_dump()
    assert "violated" in err.message


# frob:tests bindings/python/python/cadetta/queries.py::check kind=unit
def test_check_flags(project: Project, monkeypatch: pytest.MonkeyPatch) -> None:
    """release, as_of and module become CLI flags."""
    seen = _canned(project, monkeypatch, {"diagnostics": []})
    assert isinstance(project.check(True, "2026-10-13", "acme.rover"), Ok)
    assert seen == [
        ["check", "--release", "--as-of", "2026-10-13", "--module", "acme.rover"]
    ]


# frob:tests bindings/python/python/cadetta/project.py::Project.why kind=unit
# frob:tests bindings/python/python/cadetta/queries.py::why_path kind=unit
def test_why_returns_model(project: Project) -> None:
    """`why PATH` is a Why model whose chain names the member."""
    got = project.why(ID).unwrap()
    assert isinstance(got, why.Why)
    assert got.path == ID


# frob:tests bindings/python/python/cadetta/project.py::Project.why_stale kind=unit
# frob:tests bindings/python/python/cadetta/queries.py::why_stale kind=unit
def test_why_stale_flags(project: Project, monkeypatch: pytest.MonkeyPatch) -> None:
    """`why --stale OBLIGATION --role R` and its schema model."""
    payload = json.loads(project._spawn(["why", ID], global_format=True).stdout)
    seen = _canned(project, monkeypatch, payload)
    assert isinstance(project.why_stale("ob1", role="review").unwrap(), why.Why)
    assert seen == [["why", "--stale", "ob1", "--role", "review"]]


# frob:tests bindings/python/python/cadetta/project.py::Project.impact kind=unit
# frob:tests bindings/python/python/cadetta/queries.py::impact_of kind=unit
def test_impact_returns_model(project: Project) -> None:
    """`impact PATH` lists the dependents as ImpactOutput."""
    got = project.impact(OD).unwrap()
    assert isinstance(got, impact.ImpactOutput)
    assert [d.path for d in got.dependents] == [ID]


# frob:tests bindings/python/python/cadetta/queries.py::impact_of kind=unit
def test_impact_unknown_path_is_err(project: Project) -> None:
    """An unknown path is a usage error: Err(ProjectError)."""
    got = project.impact("acme.rover::Nope.x")
    assert isinstance(got, Err) and isinstance(got.unwrap_err(), ProjectError)


# frob:tests bindings/python/python/cadetta/project.py::Project.obligations kind=unit
# frob:tests bindings/python/python/cadetta/queries.py::obligations_of kind=unit
def test_obligations_model_and_flags(
    project: Project, monkeypatch: pytest.MonkeyPatch
) -> None:
    """`obligations` returns its model; options become flags."""
    got = project.obligations().unwrap()
    assert isinstance(got, obligations.ObligationsOutput)
    seen = _canned(project, monkeypatch, got.model_dump(mode="json", by_alias=True))
    assert isinstance(project.obligations(True, "r", True, "2026-10-13"), Ok)
    assert seen == [
        ["obligations", "--stale", "--role", "r", "--release", "--as-of", "2026-10-13"]
    ]


# frob:tests bindings/python/python/cadetta/project.py::Project.impact_swap kind=unit
# frob:tests bindings/python/python/cadetta/queries.py::impact_swap kind=unit
def test_impact_swap_model(project: Project, monkeypatch: pytest.MonkeyPatch) -> None:
    """`impact --swap SLOT ALT` is a Swap model."""
    seen = _canned(project, monkeypatch, SWAP)
    got = project.impact_swap("acme.rover::Rover/wheel", "acme.shop::W2").unwrap()
    assert isinstance(got, swap.Swap)
    assert got.verdict == swap.Verdict.ok_with_warnings
    assert seen == [["impact", "--swap", "acme.rover::Rover/wheel", "acme.shop::W2"]]


# frob:tests bindings/python/python/cadetta/queries.py::impact_swap kind=unit
def test_impact_swap_unknown_slot_is_err(project: Project) -> None:
    """A slot that does not exist is Err(ProjectError), not a report."""
    got = project.impact_swap("acme.rover::Nope/x", "acme.rover::Axle")
    assert isinstance(got, Err) and isinstance(got.unwrap_err(), ProjectError)


# frob:tests bindings/python/python/cadetta/project.py::Project.swap kind=unit
# frob:tests bindings/python/python/cadetta/queries.py::swap_part kind=unit
def test_swap_model_and_flags(
    project: Project, monkeypatch: pytest.MonkeyPatch
) -> None:
    """`swap SLOT ALT --reason R --as E` is a Swap model."""
    seen = _canned(project, monkeypatch, SWAP)
    got = project.swap("s", "a", "cheaper", as_="me@x.org")
    assert isinstance(got.unwrap(), swap.Swap)
    assert seen == [["swap", "s", "a", "--reason", "cheaper", "--as", "me@x.org"]]


# frob:tests bindings/python/python/cadetta/project.py::Project.compare kind=unit
# frob:tests bindings/python/python/cadetta/queries.py::compare_interface kind=unit
def test_compare_model(project: Project, monkeypatch: pytest.MonkeyPatch) -> None:
    """`compare IFACE` is a Compare model with one column per implementer."""
    seen = _canned(project, monkeypatch, COMPARE)
    got = project.compare("acme.rover::Wheel").unwrap()
    assert isinstance(got, compare.Compare)
    assert got.columns[0].component == "acme.shop::W2"
    assert seen == [["compare", "acme.rover::Wheel"]]


# frob:tests bindings/python/python/cadetta/queries.py::compare_interface kind=unit
def test_compare_unknown_interface_is_err(project: Project) -> None:
    """An unknown interface is Err(ProjectError)."""
    assert isinstance(project.compare("acme.rover::Nope"), Err)
