"""The Adapter base class: JSON-RPC dispatch, schema validation, stderr-only logging."""

# frob:ticket 01M4SV2NZT32CAT33D13T86646

import io
import json
import logging
import sys

import pytest
from cadetta.adapter import (
    Adapter,
    Capability,
    Feature,
    InitializeResult,
    OpenResult,
    PushResult,
    ReadResult,
    RpcError,
    SaveResult,
    Session,
)
from cadetta.models import ir
from pydantic import ValidationError
from typani import Err, Ok

OBSERVED = {
    "schema": "cadetta.ir/1",
    "flavour": "observed",
    "package": {"name": "p", "version": "0.1.0"},
    "modules": [],
    "components": [],
    "bindings": [],
    "checks": [],
    "citations": [],
    "dependencies": [],
    "solvers": [],
}


class Fake(Adapter):
    capabilities = frozenset({Capability.EMIT, Capability.READ, Capability.DRIVE})

    def __init__(self) -> None:
        self.closed: list[str] = []

    def initialize(self, params):
        return Ok(
            InitializeResult(
                name="fake",
                version="1.0.0",
                capabilities=[Capability.READ, Capability.DRIVE],
                native_units="mm",
                precision=4,
                platforms=["linux"],
            )
        )

    def open(self, params):
        return Ok(OpenResult(session=Session(handle="s1", files=[])))

    def read(self, params):
        return Ok(ReadResult(observed=ir.Project.model_validate(OBSERVED)))

    def push(self, params):
        return Ok(PushResult(statuses=[]))

    def save(self, params):
        return Ok(SaveResult(files=[]))

    def close(self, params):
        self.closed.append(params.session)
        return Ok(None)


def call(adapter: Adapter, *msgs: dict) -> list[dict]:
    """Feed requests through serve and return the parsed replies."""
    stdin = io.StringIO("".join(json.dumps(m) + "\n" for m in msgs))
    out = io.StringIO()
    assert adapter.serve(stdin, out) == 0
    return [json.loads(x) for x in out.getvalue().splitlines()]


def rpc(method: str, params=None, id_: int = 1) -> dict:
    msg = {"jsonrpc": "2.0", "id": id_, "method": method}
    if params is not None:
        msg["params"] = params
    return msg


INIT = {
    "protocol": "cadetta.adapter/1",
    "client": {"name": "cadetta", "version": "0.1.0"},
}


# frob:tests bindings/python/python/cadetta/adapter.py::Adapter.serve kind=unit
def test_initialize_dispatches_and_result_is_schema_shaped():
    (r,) = call(Fake(), rpc("initialize", INIT))
    assert r["id"] == 1
    assert r["result"]["name"] == "fake"
    assert r["result"]["capabilities"] == ["read", "drive"]
    assert r["result"]["precision"] == 4


# frob:tests bindings/python/python/cadetta/adapter.py::Adapter._dispatch kind=unit
def test_invalid_params_and_unknown_fields_are_refused():
    bad = dict(INIT, extra=1)
    (r,) = call(Fake(), rpc("initialize", bad))
    assert r["error"]["code"] == -32602
    (r,) = call(Fake(), rpc("nope"))
    assert r["error"]["code"] == -32601


# frob:tests bindings/python/python/cadetta/adapter.py::Adapter._dispatch kind=unit
def test_invalid_handler_result_is_internal_error():
    class Bad(Fake):
        def initialize(self, params):
            return Ok({"name": "x"})

    (r,) = call(Bad(), rpc("initialize", INIT))
    assert r["error"]["code"] == -32603


# frob:tests bindings/python/python/cadetta/adapter.py::Adapter._handle_line kind=unit
def test_parse_error_and_notifications():
    out = io.StringIO()
    stdin = io.StringIO('{bad\n{"jsonrpc":"2.0","method":"shutdown"}\n')
    assert Fake().serve(stdin, out) == 0
    replies = [json.loads(x) for x in out.getvalue().splitlines()]
    assert len(replies) == 1  # the notification gets no reply, and ends the loop
    assert replies[0]["error"]["code"] == -32700


# frob:tests bindings/python/python/cadetta/adapter.py::Adapter.shutdown kind=unit
def test_shutdown_ends_loop():
    replies = call(Fake(), rpc("shutdown", id_=1), rpc("initialize", INIT, id_=2))
    assert replies == [{"jsonrpc": "2.0", "id": 1, "result": None}]


# frob:tests bindings/python/python/cadetta/adapter.py::Adapter.open kind=unit
def test_drive_methods_are_overridable_and_open_save_close_work():
    a = Fake()
    replies = call(
        a,
        rpc("open", {"files": ["a.FCStd"]}, 1),
        rpc("save", {"session": "s1"}, 2),
        rpc("close", {"session": "s1"}, 3),
    )
    assert replies[0]["result"]["session"]["handle"] == "s1"
    assert replies[1]["result"] == {"files": [], "diagnostics": []}
    assert replies[2]["result"] is None
    assert a.closed == ["s1"]


# frob:tests bindings/python/python/cadetta/adapter.py::Adapter.regen kind=unit
def test_unoverridden_and_non_drive_methods_are_method_not_found():
    (r,) = call(Fake(), rpc("regen", {"session": "s1", "targets": []}))
    assert r["error"]["code"] == -32601

    class ReadOnly(Fake):
        capabilities = frozenset({Capability.READ})

    (r,) = call(ReadOnly(), rpc("open", {"files": []}))
    assert r["error"]["code"] == -32601


# frob:tests bindings/python/python/cadetta/adapter.py::Adapter.read kind=unit
def test_observed_ir_uses_the_ir_models():
    (r,) = call(Fake(), rpc("read", {"source": {"files": ["a"]}}))
    obs = r["result"]["observed"]
    assert obs["schema"] == "cadetta.ir/1"
    assert obs["flavour"] == "observed"
    assert ir.Project.model_validate(obs).flavour == ir.Flavour.observed


# frob:tests bindings/python/python/cadetta/adapter.py::Adapter._dispatch kind=unit
def test_handler_err_becomes_rpc_error_and_exception_is_internal():
    class Failing(Fake):
        def read(self, params):
            return Err(RpcError(code=-32000, message="tool gone"))

        def emit(self, params):
            raise RuntimeError("bug")

    (r,) = call(Failing(), rpc("read", {"source": {"files": []}}))
    assert r["error"] == {"code": -32000, "message": "tool gone"}
    (r,) = call(
        Failing(), rpc("emit", {"design": OBSERVED, "bindings": {}, "policy": {}})
    )
    assert r["error"]["code"] == -32603


# frob:tests bindings/python/python/cadetta/adapter.py::Adapter.run kind=unit
def test_run_keeps_stdout_for_protocol_and_logs_to_stderr(monkeypatch, capfd):
    class Chatty(Fake):
        def initialize(self, params):
            print("stray print")
            logging.getLogger("cadetta.test").warning("a log line")
            from cadetta.logging import get_logger

            get_logger("cadetta.test").info("info line")
            return super().initialize(params)

    stdin = io.StringIO(json.dumps(rpc("initialize", INIT)) + "\n")
    monkeypatch.setattr(sys, "stdin", stdin)
    assert Chatty.run() == 0
    cap = capfd.readouterr()
    lines = cap.out.splitlines()
    assert len(lines) == 1 and json.loads(lines[0])["result"]["name"] == "fake"
    assert "stray print" in cap.err


def test_initialize_result_features_round_trip():
    # frob:ticket 01M4W6RRX0W5SZ2SNWN26XYW5F
    """features serializes when set and stays absent when not (adapters.md 5)."""
    base = dict(
        name="x",
        version="1",
        capabilities=[Capability.READ],
        native_units="mm",
        precision=4,
        platforms=["linux"],
    )
    absent = InitializeResult(**base).model_dump(mode="json", exclude_none=True)
    assert "features" not in absent
    got = InitializeResult(**base, features=[Feature.FRAMES, "tolerances"])
    assert got.model_dump(mode="json", exclude_none=True)["features"] == [
        "frames",
        "tolerances",
    ]
    with pytest.raises(ValidationError):
        InitializeResult(**base, features=["bogus"])
