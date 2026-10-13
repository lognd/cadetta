"""Project.cite_list, names, bind, unbind and Design.citation (bindings.md 3.1, 3.2)."""

# frob:ticket 01M4VB64G50QYHE89QNHNC09JV

from pathlib import Path

from cadetta import PathNotFound, Project, ProjectError
from cadetta.models import bind, cite_list, names
from typani import Err, Ok

SRC = (
    "cite colson: web {\n"
    '    title = "Colson 2 series"\n'
    '    url = "https://colson.example/2.pdf"\n'
    "}\n"
    "model Axle {\n"
    "  od: Length = 8mm @colson\n"
    "  id: Length = 4mm\n"
    "}\n"
)
OD = "acme.rover::Axle.od"
ID = "acme.rover::Axle.id"


def make_pkg(root: Path) -> Project:
    """Write a cited, adapter-configured package and open it."""
    (root / "src").mkdir(parents=True)
    (root / "cadetta.toml").write_text(
        '[package]\nname = "acme.rover"\nversion = "1.2.3"\n\n'
        '[[adapter]]\nname = "sw-equations"\nnative_unit = "mm"\n',
        encoding="utf-8",
    )
    (root / "src" / "lib.cdt").write_text(SRC, encoding="utf-8")
    return Project.open(root).unwrap()


# frob:tests bindings/python/python/cadetta/project.py::Design.citation kind=unit
def test_design_citation_lookup(tmp_path: Path) -> None:
    """design.citation(m.citation.cite) returns the Citation; unknown is Err."""
    design = make_pkg(tmp_path).evaluate().unwrap()
    cite = design.get(OD).unwrap().citation
    assert cite is not None and cite.cite is not None
    found = design.citation(cite.cite)
    assert isinstance(found, Ok)
    assert found.unwrap().type.value == "web"
    assert found.unwrap().fields["title"] == "Colson 2 series"
    missing = design.citation("nope")
    assert isinstance(missing, Err) and isinstance(missing.unwrap_err(), PathNotFound)


# frob:tests bindings/python/python/cadetta/project.py::Project.cite_list kind=unit
def test_cite_list_json_model_and_text(tmp_path: Path) -> None:
    """cite_list returns the schema model for json and plain text otherwise."""
    project = make_pkg(tmp_path)
    got = project.cite_list()
    assert isinstance(got, Ok)
    model = got.unwrap()
    assert isinstance(model, cite_list.CadettaCiteListFormatJson)
    assert [c.number for c in model.citations] == [1]
    assert model.citations[0].fields["title"] == "Colson 2 series"
    text = project.cite_list(format="ieee").unwrap()
    assert isinstance(text, str) and "Colson 2 series" in text
    scoped = project.cite_list(module="acme.rover").unwrap()
    assert len(scoped.citations) == 1


# frob:tests bindings/python/python/cadetta/project.py::Project.names kind=unit
def test_names_lists_unbound_paths(tmp_path: Path) -> None:
    """names returns the NamesReport with each path unbound before any bind."""
    project = make_pkg(tmp_path)
    report = project.names().unwrap()
    assert isinstance(report, names.NamesReport)
    rows = {n.path: n for a in report.adapters for n in a.names}
    # "unknown" when the adapter cannot read CAD on this machine.
    assert rows[OD].state.value in {"unbound", "unknown"}
    one = project.names(adapter="sw-equations", module="acme.rover").unwrap()
    assert [a.id for a in one.adapters] == ["sw-equations"]


# frob:tests bindings/python/python/cadetta/project.py::Project.names kind=unit
def test_names_unknown_adapter_is_err(tmp_path: Path) -> None:
    """An adapter the config lacks is Err(ProjectError), not an exception."""
    err = make_pkg(tmp_path).names(adapter="nope")
    assert isinstance(err, Err) and isinstance(err.unwrap_err(), ProjectError)


# frob:tests bindings/python/python/cadetta/project.py::Project.bind kind=unit
# frob:tests bindings/python/python/cadetta/project.py::Project.unbind kind=unit
def test_bind_then_unbind_round_trip(tmp_path: Path) -> None:
    """bind adds an explicit locator, names sees it, unbind removes it."""
    project = make_pkg(tmp_path)
    added = project.bind(OD, "sw-equations", "AxleOd", "global", mode="value")
    assert isinstance(added, Ok)
    report = added.unwrap()
    assert isinstance(report, bind.BindReport)
    assert report.action == bind.Action.add and report.changed
    rows = {n.path: n for a in project.names().unwrap().adapters for n in a.names}
    assert rows[OD].state.value == "explicit"
    assert [loc.locator for loc in rows[OD].locators] == ["AxleOd"]
    again = project.bind(OD, "sw-equations", "AxleOd", "global", mode="value")
    assert again.unwrap().changed is False
    removed = project.unbind(OD, "sw-equations", "AxleOd").unwrap()
    assert removed.action == bind.Action.remove and removed.changed


# frob:tests bindings/python/python/cadetta/project.py::Project.bind kind=unit
def test_bind_errors_are_results(tmp_path: Path) -> None:
    """A bad kind or an unknown field is Err(ProjectError)."""
    project = make_pkg(tmp_path)
    bad_kind = project.bind(ID, "sw-equations", "X", "bogus")
    assert isinstance(bad_kind, Err) and isinstance(bad_kind.unwrap_err(), ProjectError)
    bad_field = project.bind(ID, "sw-equations", "X", "global", nope="1")
    assert isinstance(bad_field, Err)
    assert "nope" in bad_field.unwrap_err().message
