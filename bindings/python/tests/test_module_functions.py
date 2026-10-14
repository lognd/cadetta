"""Module functions units/format_source/quantity, Project.explain and reload (bindings.md 3.1, 3.2)."""

# frob:ticket 01M4SWP8BCQ00WCDJ5FQ2QMDX9

from pathlib import Path

import cadetta
from cadetta import Project
from typani import Err, Ok

UGLY = "model   M {\n  a: Length   = 1mm\n}\n"
PRETTY = "model M {\n  a: Length = 1mm\n}\n"


def make_pkg(root: Path) -> Project:
    """Write a one-module package and open it."""
    (root / "src").mkdir(parents=True)
    (root / "cadetta.toml").write_text(
        '[package]\nname = "acme.rover"\nversion = "1.2.3"\n', encoding="utf-8"
    )
    (root / "src" / "lib.cdt").write_text(PRETTY, encoding="utf-8")
    return Project.open(root).unwrap()


# frob:tests bindings/python/python/cadetta/tools.py::units kind=unit
def test_units_all_and_by_kind() -> None:
    """units() lists every kind; units("Length") one, with exact factors."""
    everything = cadetta.units()
    assert isinstance(everything, Ok) and len(everything.unwrap().kinds) > 1
    length = cadetta.units("Length").unwrap()
    assert [k.kind for k in length.kinds] == ["Length"]
    assert {u.name for u in length.kinds[0].units} >= {"mm", "m", "in"}


# frob:tests bindings/python/python/cadetta/tools.py::format_source kind=unit
def test_format_source_canonical_and_error() -> None:
    """format_source returns canonical text; unparseable source is Err."""
    assert cadetta.format_source(UGLY).unwrap() == PRETTY
    assert isinstance(cadetta.format_source("model {{{"), Err)


# frob:tests bindings/python/python/cadetta/quantity.py::quantity kind=unit
def test_quantity_returns_result() -> None:
    """cadetta.quantity returns Ok(Quantity) or Err(QuantityError)."""
    ok = cadetta.quantity("12mm [+-0.1mm]")
    assert isinstance(ok, Ok) and ok.unwrap().kind == "Length"
    assert isinstance(cadetta.quantity("12 bogus"), Err)


# frob:tests bindings/python/python/cadetta/project.py::Project.explain kind=unit
def test_project_explain(tmp_path: Path) -> None:
    """project.explain(code) returns the catalog entry."""
    entry = make_pkg(tmp_path).explain("Q002").unwrap()
    assert entry.code == "Q002" and entry.name == "dimension-mismatch"


# frob:tests bindings/python/python/cadetta/project.py::Project.reload kind=unit
def test_project_reload_rereads_snapshot(tmp_path: Path) -> None:
    """reload() picks up a changed cadetta.toml and new modules."""
    project = make_pkg(tmp_path)
    assert project.package.version == "1.2.3"
    (tmp_path / "cadetta.toml").write_text(
        '[package]\nname = "acme.rover"\nversion = "2.0.0"\n', encoding="utf-8"
    )
    (tmp_path / "src" / "extra.cdt").write_text(PRETTY, encoding="utf-8")
    assert project.package.version == "1.2.3"
    assert isinstance(project.reload(), Ok)
    assert project.package.version == "2.0.0"
    assert "acme.rover::extra" in project.modules
