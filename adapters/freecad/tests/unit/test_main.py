"""Smoke tests for the adapter entry point."""

from cadetta_freecad.__main__ import main


# frob:tests adapters/freecad/src/cadetta_freecad/__main__.py::main kind=unit
def test_version_prints_to_stdout(capsys):
    assert main(["--version"]) == 0
    assert capsys.readouterr().out == "cadetta-freecad 0.0.0\n"


# frob:tests adapters/freecad/src/cadetta_freecad/__main__.py::main kind=unit
def test_no_arguments_serves_the_adapter(monkeypatch):
    from cadetta_freecad import __main__ as m

    monkeypatch.setattr(m.FreeCadAdapter, "run", classmethod(lambda cls: 0))
    assert main([]) == 0
