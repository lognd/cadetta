"""Smoke tests for the adapter entry point."""

from cadetta_freecad.__main__ import main


# frob:tests adapters/freecad/src/cadetta_freecad/__main__.py::main kind=unit
def test_version_prints_to_stdout(capsys):
    assert main(["--version"]) == 0
    assert capsys.readouterr().out == "cadetta-freecad 0.0.0\n"


# frob:tests adapters/freecad/src/cadetta_freecad/__main__.py::main kind=unit
def test_no_arguments_is_a_usage_error_and_logged(capsys, caplog):
    assert main([]) == 2
    assert capsys.readouterr().out == ""
    assert "not implemented" in caplog.text
