"""Tests for the in-Fusion add-in's local-archive branch, against a stub adsk."""

from __future__ import annotations

import importlib.util
import sys
import types
from pathlib import Path

import pytest

ADDIN = Path(__file__).parents[2] / "addin" / "CadettaFusion" / "CadettaFusion.py"


class _Export:
    def __init__(self, log):
        self.log = log

    def createFusionArchiveExportOptions(self, path):
        return ("opts", path)

    def execute(self, options):
        self.log.append(("export", options[1]))
        return True


class _Design:
    def __init__(self, log):
        self.exportManager = _Export(log)


class _Products:
    def __init__(self, design):
        self.design = design

    def itemByProductType(self, _kind):
        return self.design


class _Doc:
    def __init__(self, name, log):
        self.name = name
        self.isModified = False
        self.products = _Products(_Design(log))
        self.log = log

    def close(self, save):
        self.log.append(("close", self.name, save))


class _Docs(list):
    def open(self, data_file, _visible):
        raise AssertionError("cloud open not expected: %r" % (data_file,))


class _Import:
    def __init__(self, log):
        self.log = log

    def createFusionArchiveImportOptions(self, path):
        return ("import", path)

    def importToNewDocument(self, options):
        self.log.append(("import", options[1]))
        return _Doc(Path(options[1]).stem, self.log)


@pytest.fixture
def addin(monkeypatch):
    """Load the add-in module with stub adsk and a recording app."""
    log: list[tuple] = []
    adsk = types.ModuleType("adsk")
    core = types.ModuleType("adsk.core")
    fusion = types.ModuleType("adsk.fusion")
    core.CustomEventHandler = object
    fusion.Design = types.SimpleNamespace(cast=lambda d: d)
    adsk.core, adsk.fusion = core, fusion
    for name, mod in (("adsk", adsk), ("adsk.core", core), ("adsk.fusion", fusion)):
        monkeypatch.setitem(sys.modules, name, mod)
    spec = importlib.util.spec_from_file_location("cadetta_fusion_addin", ADDIN)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    mod._app = types.SimpleNamespace(
        documents=_Docs(),
        importManager=_Import(log),
        data=types.SimpleNamespace(),
        log=lambda msg: log.append(("log", msg)),
    )
    mod.log = log
    return mod


def _req(method, **params):
    return {"jsonrpc": "2.0", "id": 1, "method": method, "params": params}


# frob:tests adapters/fusion/addin/CadettaFusion/CadettaFusion.py::_is_local kind=unit
def test_is_local_needs_an_existing_archive(addin, tmp_path):
    real = tmp_path / "a.f3d"
    real.write_bytes(b"x")
    assert addin._is_local(str(real))
    assert not addin._is_local(str(tmp_path / "missing.f3d"))
    assert not addin._is_local("urn:adsk.wipprod:dm.lineage:abc")


# frob:tests adapters/fusion/addin/CadettaFusion/CadettaFusion.py::_handle kind=unit
def test_open_save_close_local_archive(addin, tmp_path):
    f = tmp_path / "t.f3d"
    f.write_bytes(b"x")
    path = str(f)
    opened = addin._handle(_req("doc.open", path=path))
    assert opened["result"] == {
        "path": "t",
        "dirty": False,
        "readonly": False,
        "attached": False,
    }
    again = addin._handle(_req("doc.open", path=path))
    assert again["result"]["attached"] is True
    assert addin._handle(_req("doc.save", path=path))["result"] is None
    assert addin._handle(_req("doc.close", path=path))["result"] is None
    assert ("import", path) in addin.log
    assert ("export", path) in addin.log
    assert ("close", "t", False) in addin.log
    assert addin._local == {} and addin._opened == set()


# frob:tests adapters/fusion/addin/CadettaFusion/CadettaFusion.py::_handle kind=unit
def test_non_archive_path_still_uses_cloud_lookup(addin):
    def boom(_id):
        raise RuntimeError("3 : file not found")

    addin._app.data.findFileById = boom
    reply = addin._handle(_req("doc.open", path="urn:x"))
    assert reply["error"]["message"] == "3 : file not found"
    assert addin._local == {}


# frob:tests adapters/fusion/addin/CadettaFusion/CadettaFusion.py::_handle kind=unit
def test_documents_lists_open_document_names(addin):
    addin._app.documents.extend(
        [types.SimpleNamespace(name="a"), types.SimpleNamespace(name="b")]
    )
    assert addin._handle(_req("fusion.documents"))["result"] == ["a", "b"]
