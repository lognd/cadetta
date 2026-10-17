"""Smoke tests for the adapter entry point."""

from cadetta_fusion.__main__ import main


# frob:tests adapters/fusion/src/cadetta_fusion/__main__.py::main kind=unit
def test_version_prints_to_stdout(capsys):
    assert main(["--version"]) == 0
    assert capsys.readouterr().out == "cadetta-fusion 0.0.0\n"


# frob:tests adapters/fusion/src/cadetta_fusion/__main__.py::main kind=unit
def test_no_arguments_serves_the_adapter(monkeypatch):
    from cadetta_fusion import __main__ as m

    monkeypatch.setattr(m.FusionAdapter, "run", classmethod(lambda cls: 0))
    assert main([]) == 0
