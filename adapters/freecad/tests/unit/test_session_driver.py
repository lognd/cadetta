"""Tests for the session conformance driver: the adapter attaches and closes only its own."""

# frob:ticket 01M52SAFHK1694NYSMG6NNC66V

import importlib.util
import json
from pathlib import Path

_PATH = Path(__file__).resolve().parent.parent / "conformance" / "session_driver.py"
_SPEC = importlib.util.spec_from_file_location("session_driver", _PATH)
driver = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(driver)

_BASE = {"document": {"file": "cad/plate.SLDPRT", "dirty": False, "read_only": False}}


# frob:tests adapters/freecad/tests/conformance/session_driver.py::drive_fixture kind=unit
def test_running_instance_is_attached_and_left_open():
    facts = driver.drive_fixture({**_BASE, "running_instance": True})
    assert facts["attached"] and not facts["started"]
    assert not facts["closes_tool"] and facts["left_open"] == ["cad/plate.SLDPRT"]


# frob:tests adapters/freecad/tests/conformance/session_driver.py::drive_fixture kind=unit
def test_no_running_instance_opens_hidden_and_closes_it():
    facts = driver.drive_fixture({**_BASE, "running_instance": False})
    assert facts["started"] and facts["hidden"] and not facts["attached"]
    assert facts["closed"] == ["cad/plate.SLDPRT"] and facts["left_open"] == []


# frob:tests adapters/freecad/tests/conformance/session_driver.py::drive_fixture kind=unit
def test_close_leaves_preopened_documents_alone():
    spec = {
        **_BASE,
        "running_instance": True,
        "opened_by_adapter": ["cad/plate.SLDPRT"],
        "preopened": ["cad/other.SLDPRT"],
    }
    facts = driver.drive_fixture(spec)
    assert facts["closed"] == ["cad/plate.SLDPRT"]
    assert facts["left_open"] == ["cad/other.SLDPRT"]


# frob:tests adapters/freecad/tests/conformance/session_driver.py::main kind=unit
def test_main_prints_json_facts(tmp_path, capsys):
    fixture = tmp_path / "f.json"
    fixture.write_text(json.dumps({**_BASE, "running_instance": True}))
    assert driver.main([str(fixture)]) == 0
    assert json.loads(capsys.readouterr().out)["attached"] is True
