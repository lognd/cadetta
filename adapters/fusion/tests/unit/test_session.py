"""RpcSession against a scripted in-process add-in; no Fusion needed."""

import io
import json
import socket
import threading

from typani import Err, Ok

from cadetta_fusion.session import RpcSession, connect


def _session(*replies):
    reader = io.StringIO("".join(json.dumps(r) + "\n" for r in replies))
    writer = io.StringIO()
    return RpcSession(reader, writer), writer


# frob:tests adapters/fusion/src/cadetta_fusion/session.py::RpcSession.call kind=unit
def test_call_returns_result_and_sends_request():
    s, w = _session({"jsonrpc": "2.0", "id": 1, "result": "2.0"})
    assert s.version() == Ok("2.0")
    sent = json.loads(w.getvalue())
    assert (sent["method"], sent["id"]) == ("fusion.version", 1)


# frob:tests adapters/fusion/src/cadetta_fusion/session.py::RpcSession.call kind=unit
def test_call_maps_errors_eof_and_id_mismatch():
    s, _ = _session(
        {"jsonrpc": "2.0", "id": 1, "error": {"code": 1, "message": "boom"}}
    )
    assert s.version() == Err("boom")
    s, _ = _session()
    assert s.version().is_err
    s, _ = _session({"jsonrpc": "2.0", "id": 9, "result": "x"})
    assert s.version().is_err


# frob:tests adapters/fusion/src/cadetta_fusion/session.py::RpcSession.parameters kind=unit
def test_parameters_validates_payload():
    row = {"name": "w", "expression": "1 mm", "unit": "mm", "value": 0.1}
    s, _ = _session({"jsonrpc": "2.0", "id": 1, "result": [row]})
    assert s.parameters("/a").danger_ok[0].name == "w"
    s, _ = _session({"jsonrpc": "2.0", "id": 1, "result": [{"nope": 1}]})
    assert s.parameters("/a").is_err


# frob:tests adapters/fusion/src/cadetta_fusion/session.py::connect kind=unit
def test_connect_over_a_real_socket_and_failure():
    srv = socket.socket()
    srv.bind(("127.0.0.1", 0))
    srv.listen(1)
    port = srv.getsockname()[1]

    def serve():
        conn, _ = srv.accept()
        f = conn.makefile("rw", encoding="utf-8", newline="\n")
        req = json.loads(f.readline())
        f.write(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": "9"}) + "\n")
        f.flush()
        conn.close()

    t = threading.Thread(target=serve)
    t.start()
    rpc = connect(f"127.0.0.1:{port}").danger_ok
    assert rpc.version() == Ok("9")
    t.join()
    srv.close()
    assert connect(f"127.0.0.1:{port}").is_err
