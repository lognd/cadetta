"""Protocol tests for FreeCadAdapter, run without FreeCAD."""

import io
import json

from typani import Err, Ok

from cadetta_freecad.adapter import FreeCadAdapter
from cadetta_freecad.backend import LiveBackend


class _FakeBackend:
    def __init__(self, version):
        self._version = version

    def freecad_version(self):
        return self._version


def _serve(adapter, *requests):
    stdin = io.StringIO("".join(json.dumps(r) + "\n" for r in requests))
    stdout = io.StringIO()
    assert adapter.serve(stdin, stdout) == 0
    return [json.loads(line) for line in stdout.getvalue().splitlines()]


_INIT = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "protocol": "cadetta.adapter/1",
        "client": {"name": "cadetta", "version": "0.1.0"},
    },
}


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.initialize kind=unit
def test_initialize_returns_all_three_capabilities():
    (reply,) = _serve(FreeCadAdapter(_FakeBackend(Ok("1.0.0"))), _INIT)
    result = reply["result"]
    assert sorted(result["capabilities"]) == ["drive", "emit", "read"]
    assert result["name"] == "freecad"
    assert result["native_units"] == "mm"
    assert result["precision"] == 6


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.initialize kind=unit
def test_initialize_succeeds_without_freecad():
    (reply,) = _serve(FreeCadAdapter(_FakeBackend(Err("missing"))), _INIT)
    assert "result" in reply


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.initialize kind=unit
def test_initialize_rejects_unknown_protocol():
    bad = json.loads(json.dumps(_INIT))
    bad["params"]["protocol"] = "other/9"
    (reply,) = _serve(FreeCadAdapter(_FakeBackend(Ok("1"))), bad)
    assert reply["error"]["code"] == -32602


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter kind=unit
def test_served_by_adapter_base_and_shutdown_stops_loop():
    from cadetta.adapter import Adapter

    assert issubclass(FreeCadAdapter, Adapter)
    replies = _serve(
        FreeCadAdapter(_FakeBackend(Ok("1"))),
        _INIT,
        {"jsonrpc": "2.0", "id": 2, "method": "shutdown"},
        {"jsonrpc": "2.0", "id": 3, "method": "initialize"},
    )
    assert [r["id"] for r in replies] == [1, 2]


# frob:tests adapters/freecad/src/cadetta_freecad/backend.py::LiveBackend.freecad_version kind=unit
def test_live_backend_errs_when_freecad_missing():
    assert isinstance(LiveBackend().freecad_version(), Err)


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter kind=integration
def test_run_keeps_stdout_for_protocol_only(monkeypatch, capsys):
    msgs = [_INIT, {"jsonrpc": "2.0", "id": 2, "method": "shutdown"}]
    monkeypatch.setattr(
        "sys.stdin", io.StringIO("".join(json.dumps(m) + "\n" for m in msgs))
    )
    assert FreeCadAdapter.run() == 0
    out = capsys.readouterr().out
    replies = [json.loads(line) for line in out.splitlines()]
    assert sorted(replies[0]["result"]["capabilities"]) == ["drive", "emit", "read"]
    assert replies[1]["result"] is None
