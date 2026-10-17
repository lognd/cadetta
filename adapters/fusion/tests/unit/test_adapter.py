"""Fake-session transcript tests for FusionAdapter; no Fusion needed."""

import io
import json

from cadetta_fusion.adapter import FusionAdapter

from ._fake import FakeSession, param

DESIGN = "/w/bracket.f3d"


def _serve(adapter, *requests):
    stdin = io.StringIO("".join(json.dumps(r) + "\n" for r in requests))
    stdout = io.StringIO()
    assert adapter.serve(stdin, stdout) == 0
    return [json.loads(line) for line in stdout.getvalue().splitlines()]


def _req(i, method, params):
    return {"jsonrpc": "2.0", "id": i, "method": method, "params": params}


_INIT = _req(
    1,
    "initialize",
    {"protocol": "cadetta.adapter/1", "client": {"name": "cadetta", "version": "0"}},
)


def _fake():
    return FakeSession({DESIGN: [param("width", "10 mm"), param("h", "width*2")]})


def _change(locator, value, mode="value", **kw):
    return {
        "path": f"b::{locator}",
        "value": value,
        "locator": locator,
        "mode": mode,
        **kw,
    }


# frob:tests adapters/fusion/src/cadetta_fusion/adapter.py::FusionAdapter.initialize kind=unit
def test_initialize_reports_read_and_drive():
    (reply,) = _serve(FusionAdapter(_fake()), _INIT)
    res = reply["result"]
    assert res["name"] == "fusion"
    assert sorted(res["capabilities"]) == ["drive", "read"]
    assert res["native_units"] == "mm"


# frob:tests adapters/fusion/src/cadetta_fusion/adapter.py::FusionAdapter.initialize kind=unit
def test_initialize_rejects_unknown_protocol():
    bad = _req(
        1, "initialize", {"protocol": "x/9", "client": {"name": "c", "version": "0"}}
    )
    (reply,) = _serve(FusionAdapter(_fake()), bad)
    assert reply["error"]["code"] == -32602


# frob:tests adapters/fusion/src/cadetta_fusion/adapter.py::FusionAdapter.read kind=unit
def test_read_files_opens_reads_and_closes_without_saving():
    fake = _fake()
    _, reply = _serve(
        FusionAdapter(fake), _INIT, _req(2, "read", {"source": {"files": [DESIGN]}})
    )
    members = reply["result"]["observed"]["components"][0]["members"]
    assert [m["path"].rsplit(".", 1)[1] for m in members] == ["width", "h"]
    assert members[0]["value"]["exact"] is True
    assert members[1]["value"]["exact"] is False
    assert ("close", DESIGN) in fake.transcript
    assert ("save", DESIGN) not in fake.transcript
    assert [d["code"] for d in reply["result"]["diagnostics"]] == ["Q006"]


# frob:tests adapters/fusion/src/cadetta_fusion/adapter.py::FusionAdapter.read kind=unit
def test_read_unknown_session_is_invalid_params():
    _, reply = _serve(
        FusionAdapter(_fake()), _INIT, _req(2, "read", {"source": {"session": "nope"}})
    )
    assert reply["error"]["code"] == -32602


# frob:tests adapters/fusion/src/cadetta_fusion/adapter.py::FusionAdapter.push kind=unit
def test_push_applies_atomically_saves_and_closes(tmp_path):
    design = str(tmp_path / "b.f3d")
    (tmp_path / "b.f3d").write_bytes(b"x")
    fake = FakeSession({design: [param("width", "10 mm")]})
    out = _serve(
        FusionAdapter(fake),
        _INIT,
        _req(2, "open", {"files": [design]}),
        _req(
            3,
            "push",
            {"session": "s1", "file": design, "params": [_change("width", 12.5)]},
        ),
        _req(4, "regen", {"session": "s1", "targets": []}),
        _req(5, "save", {"session": "s1"}),
        _req(6, "close", {"session": "s1"}),
    )
    push = out[2]["result"]["statuses"][0]
    assert (push["status"], push["old"], push["new"]) == (
        "pushed",
        "10 mm",
        "12.5 mm",
    )
    assert out[3]["result"]["targets"][0]["status"] == "ok"
    assert out[4]["result"]["files"][0]["digest"].startswith("blake3:")
    assert out[5]["result"] is None
    assert [t[0] for t in fake.transcript if t[0] in ("apply", "close")] == [
        "apply",
        "close",
    ]


# frob:tests adapters/fusion/src/cadetta_fusion/adapter.py::FusionAdapter.push kind=unit
def test_push_missing_locator_is_r003_and_no_apply():
    fake = _fake()
    out = _serve(
        FusionAdapter(fake),
        _INIT,
        _req(2, "open", {"files": [DESIGN]}),
        _req(
            3, "push", {"session": "s1", "file": DESIGN, "params": [_change("zz", 1)]}
        ),
    )
    res = out[2]["result"]
    assert res["statuses"][0]["status"] == "skipped"
    assert res["diagnostics"][0]["code"] == "R003"
    assert not [t for t in fake.transcript if t[0] == "apply"]


# frob:tests adapters/fusion/src/cadetta_fusion/drive.py::DriveSessions.push kind=unit
def test_push_failed_apply_is_r004_and_changes_nothing():
    fake = _fake()
    fake.fail_apply = True
    out = _serve(
        FusionAdapter(fake),
        _INIT,
        _req(2, "open", {"files": [DESIGN]}),
        _req(
            3,
            "push",
            {
                "session": "s1",
                "file": DESIGN,
                "params": [_change("width", 3), _change("h", 4)],
            },
        ),
        _req(5, "save", {"session": "s1"}),
    )
    res = out[2]["result"]
    assert [s["status"] for s in res["statuses"]] == ["failed", "failed"]
    assert res["diagnostics"][-1]["code"] == "R004"
    assert fake.docs[DESIGN][0].expression == "10 mm"
    assert out[3]["result"]["files"] == []
    assert not [t for t in fake.transcript if t[0] == "save"]


# frob:tests adapters/fusion/src/cadetta_fusion/drive.py::DriveSessions.push kind=unit
def test_push_overwriting_an_expression_warns_r006_and_ref_links():
    fake = _fake()
    out = _serve(
        FusionAdapter(fake),
        _INIT,
        _req(2, "open", {"files": [DESIGN]}),
        _req(
            3,
            "push",
            {
                "session": "s1",
                "file": DESIGN,
                "params": [_change("h", 0, mode="ref", ref_to="width")],
            },
        ),
    )
    res = out[2]["result"]
    assert res["statuses"][0]["new"] == "width"
    assert [d["code"] for d in res["diagnostics"]] == ["R006"]


# frob:tests adapters/fusion/src/cadetta_fusion/drive.py::DriveSessions.push kind=unit
def test_push_bad_value_is_r010_and_unchanged_value_is_skipped():
    fake = _fake()
    out = _serve(
        FusionAdapter(fake),
        _INIT,
        _req(2, "open", {"files": [DESIGN]}),
        _req(
            3,
            "push",
            {
                "session": "s1",
                "file": DESIGN,
                "params": [_change("width", "abc"), _change("h", "width*2")],
            },
        ),
    )
    res = out[2]["result"]
    assert res["statuses"][0]["status"] == "rejected"
    assert res["diagnostics"][0]["code"] == "R010"
    out2 = _serve(
        FusionAdapter(_fake()),
        _INIT,
        _req(2, "open", {"files": [DESIGN]}),
        _req(
            3,
            "push",
            {"session": "s1", "file": DESIGN, "params": [_change("width", "10 mm")]},
        ),
    )
    assert out2[2]["result"]["statuses"][0]["status"] == "skipped"


# frob:tests adapters/fusion/src/cadetta_fusion/drive.py::DriveSessions.open kind=unit
def test_open_dirty_document_is_r009_and_push_is_rejected():
    fake = _fake()
    fake.state[DESIGN] = __import__("cadetta_fusion.session", fromlist=["x"]).DocState(
        path=DESIGN, dirty=True, readonly=False, attached=True
    )
    out = _serve(
        FusionAdapter(fake),
        _INIT,
        _req(2, "open", {"files": [DESIGN]}),
        _req(
            3,
            "push",
            {"session": "s1", "file": DESIGN, "params": [_change("width", 1)]},
        ),
        _req(4, "close", {"session": "s1"}),
    )
    assert out[1]["result"]["diagnostics"][0]["code"] == "R009"
    assert out[2]["result"]["statuses"][0]["status"] == "rejected"
    assert not [t for t in fake.transcript if t[0] == "apply"]
    assert ("close", DESIGN) not in fake.transcript  # attached docs stay open


# frob:tests adapters/fusion/src/cadetta_fusion/adapter.py::FusionAdapter.open kind=unit
def test_open_missing_design_is_an_error():
    out = _serve(
        FusionAdapter(_fake()), _INIT, _req(2, "open", {"files": ["/nope.f3d"]})
    )
    assert out[1]["error"]["code"] == -32000
