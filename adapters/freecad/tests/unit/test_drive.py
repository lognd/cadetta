"""Drive tests for FreeCadAdapter: an in-memory fake backend stands in for FreeCAD."""

# frob:ticket 01M4SPWSMZASH05AGR95PGAZKD

from cadetta.adapter import (
    OpenParams,
    PushParams,
    ReadParams,
    RegenParams,
    SessionParams,
)
from typani import Err, Ok

from cadetta_freecad.adapter import FreeCadAdapter
from cadetta_freecad.backend import DocState, LiveBackend, NativeDocument, NativeItem


class _DriveBackend:
    """Documents as dicts of name to NativeItem; records every call."""

    def __init__(self, items, dirty=False, readonly=False, fail_rebuild=False):
        self.items = {"p.FCStd": {i.name: i for i in items}}
        self.dirty, self.readonly, self.fail_rebuild = dirty, readonly, fail_rebuild
        self.calls = []
        self.saved = []

    def freecad_version(self):
        return Ok("1.0.0")

    def read_document(self, path):
        return Ok(
            NativeDocument(path=path, label="p", items=list(self.items[path].values()))
        )

    def open_document(self, path):
        self.calls.append(("open", path))
        return Ok(DocState(path=path, dirty=self.dirty, readonly=self.readonly))

    def get_item(self, path, locator):
        return Ok(self.items[path].get(locator))

    def set_item(self, path, item):
        self.calls.append(("set", item.name))
        self.items[path][item.name] = item
        return Ok(None)

    def recompute(self, path):
        self.calls.append(("recompute", path))
        if self.fail_rebuild and ("recompute", path) == self.calls[-1]:
            bad = any(
                c[0] == "set" and c[1] == "Sketch.Len" for c in self.calls
            ) and not any(c == ("restored",) for c in self.calls)
            if bad:
                return Ok(["Sketch: broken constraint"])
        return Ok([])

    def save_document(self, path):
        self.saved.append(path)
        return Ok(None)

    def close_document(self, path):
        self.calls.append(("close", path))
        return Ok(None)


def _items():
    return [
        NativeItem(kind="alias", name="axle_d", content="=12 mm"),
        NativeItem(kind="alias", name="axle_w", content="=6 mm"),
        NativeItem(kind="dimension", name="Sketch.Len", value=25.0, unit="mm"),
        NativeItem(
            kind="dimension",
            name="Sketch.Eq",
            value=5.0,
            unit="mm",
            expression="axle_w",
        ),
        NativeItem(kind="dimension", name="Sketch.Ref", value=1.0, driven=True),
    ]


def _open(adapter, path="p.FCStd"):
    return adapter.open(OpenParams(files=[path]))


def _push(adapter, handle, *changes):
    params = [
        {"path": f"a.{loc}", "value": v, "locator": loc, "mode": mode}
        for loc, v, mode in changes
    ]
    return adapter.push(
        PushParams.model_validate(
            {"session": handle, "file": "p.FCStd", "params": params}
        )
    )


# frob:tests adapters/freecad/src/cadetta_freecad/drive.py::DriveSessions.push kind=unit
# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.open kind=unit
# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.push kind=unit
def test_push_updates_only_the_changed_alias():
    backend = _DriveBackend(_items())
    adapter = FreeCadAdapter(backend)
    handle = _open(adapter).unwrap().session.handle
    result = _push(
        adapter, handle, ("axle_d", 13.5, "value"), ("axle_w", "6", "value")
    ).unwrap()
    assert [(s.locator, s.status.value) for s in result.statuses] == [
        ("axle_d", "pushed"),
        ("axle_w", "skipped"),
    ]
    assert backend.items["p.FCStd"]["axle_d"].content == "=13.5 mm"
    assert backend.items["p.FCStd"]["axle_w"].content == "=6 mm"
    assert [c for c in backend.calls if c[0] == "set"] == [("set", "axle_d")]


# frob:tests adapters/freecad/src/cadetta_freecad/drive.py::DriveSessions.push kind=unit
def test_push_dimension_rejects_driven_skips_missing_and_warns_on_expression():
    backend = _DriveBackend(_items())
    adapter = FreeCadAdapter(backend)
    handle = _open(adapter).unwrap().session.handle
    result = _push(
        adapter,
        handle,
        ("Sketch.Len", 30, "value"),
        ("Sketch.Eq", 7, "value"),
        ("Sketch.Ref", 2, "value"),
        ("nope", 1, "value"),
        ("Sketch.Len", "x", "value"),
    ).unwrap()
    assert [s.status.value for s in result.statuses] == [
        "pushed",
        "pushed",
        "rejected",
        "skipped",
        "rejected",
    ]
    assert sorted(d.code for d in result.diagnostics) == [
        "R003",
        "R006",
        "R010",
        "R010",
    ]
    assert backend.items["p.FCStd"]["Sketch.Eq"].expression is None
    assert backend.items["p.FCStd"]["Sketch.Len"].value == 30.0


# frob:tests adapters/freecad/src/cadetta_freecad/drive.py::DriveSessions.push kind=unit
def test_push_ref_mode_links_alias_and_dimension_to_ref_to():
    backend = _DriveBackend(_items())
    adapter = FreeCadAdapter(backend)
    handle = _open(adapter).unwrap().session.handle
    params = PushParams.model_validate(
        {
            "session": handle,
            "file": "p.FCStd",
            "params": [
                {
                    "path": "a",
                    "value": 1,
                    "locator": "axle_d",
                    "mode": "ref",
                    "ref_to": "axle_w",
                },
                {
                    "path": "b",
                    "value": 1,
                    "locator": "Sketch.Len",
                    "mode": "ref",
                    "ref_to": "axle_w",
                },
            ],
        }
    )
    assert adapter.push(params).is_ok
    assert backend.items["p.FCStd"]["axle_d"].content == "=axle_w"
    assert backend.items["p.FCStd"]["Sketch.Len"].expression == "axle_w"
    assert adapter.push(params.model_copy(update={"session": "zz"})).is_err


# frob:tests adapters/freecad/src/cadetta_freecad/drive.py::DriveSessions.regen kind=unit
# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.regen kind=unit
# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.save kind=unit
def test_regen_failure_is_r004_and_restores_the_snapshot():
    backend = _DriveBackend(_items(), fail_rebuild=True)
    adapter = FreeCadAdapter(backend)
    handle = _open(adapter).unwrap().session.handle
    _push(adapter, handle, ("Sketch.Len", 99, "value"), ("axle_d", 20, "value"))
    original_set = backend.set_item

    def restoring(path, item):
        backend.calls.append(("restored",))
        return original_set(path, item)

    backend.set_item = restoring
    result = adapter.regen(RegenParams(session=handle, targets=[])).unwrap()
    assert [t.status.value for t in result.targets] == ["failed"]
    (diag,) = result.diagnostics
    assert diag.code == "R004" and diag.severity == "error"
    assert "document restored from snapshot" in diag.message
    assert backend.items["p.FCStd"]["Sketch.Len"].value == 25.0
    assert backend.items["p.FCStd"]["axle_d"].content == "=12 mm"
    saved = adapter.save(SessionParams(session=handle)).unwrap()
    assert saved.files == [] and backend.saved == []


# frob:tests adapters/freecad/src/cadetta_freecad/drive.py::DriveSessions.regen kind=unit
def test_regen_restore_failure_is_reported_in_the_diagnostic():
    backend = _DriveBackend(_items(), fail_rebuild=True)
    adapter = FreeCadAdapter(backend)
    handle = _open(adapter).unwrap().session.handle
    _push(adapter, handle, ("Sketch.Len", 99, "value"))
    backend.set_item = lambda path, item: Err("cannot write")
    result = adapter.regen(RegenParams(session=handle, targets=["p.FCStd"])).unwrap()
    assert "restore FAILED" in result.diagnostics[0].message
    assert adapter.regen(RegenParams(session=handle, targets=["x"])).is_err
    assert adapter.regen(RegenParams(session="zz", targets=[])).is_err


# frob:tests adapters/freecad/src/cadetta_freecad/drive.py::DriveSessions.open kind=unit
def test_dirty_or_readonly_document_is_refused_with_r009_before_any_push():
    for kwargs, word in (
        ({"dirty": True}, "unsaved"),
        ({"readonly": True}, "read-only"),
    ):
        backend = _DriveBackend(_items(), **kwargs)
        adapter = FreeCadAdapter(backend)
        opened = _open(adapter).unwrap()
        assert [d.code for d in opened.diagnostics] == ["R009"]
        assert word in opened.diagnostics[0].message
        result = _push(adapter, opened.session.handle, ("axle_d", 1, "value")).unwrap()
        assert [s.status.value for s in result.statuses] == ["rejected"]
        assert result.diagnostics[0].code == "R009"
        assert not [c for c in backend.calls if c[0] == "set"]


# frob:tests adapters/freecad/src/cadetta_freecad/drive.py::DriveSessions.save kind=unit
# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.close kind=unit
def test_save_after_regen_ok_reports_digest_and_close_releases(tmp_path):
    path = tmp_path / "p.FCStd"
    path.write_bytes(b"fcstd")
    backend = _DriveBackend(_items())
    backend.items[str(path)] = backend.items.pop("p.FCStd")
    adapter = FreeCadAdapter(backend)
    handle = adapter.open(OpenParams(files=[str(path)])).unwrap().session.handle
    adapter.push(
        PushParams.model_validate(
            {
                "session": handle,
                "file": str(path),
                "params": [
                    {"path": "a", "value": 2, "locator": "axle_d", "mode": "value"}
                ],
            }
        )
    )
    assert (
        adapter.regen(RegenParams(session=handle, targets=[])).unwrap().diagnostics
        == []
    )
    read = adapter.read(ReadParams.model_validate({"source": {"session": handle}}))
    assert read.is_ok
    (saved,) = adapter.save(SessionParams(session=handle)).unwrap().files
    assert saved.path == str(path) and saved.digest.startswith("blake3:")
    assert adapter.save(SessionParams(session=handle)).unwrap().files == []
    assert adapter.close(SessionParams(session=handle)).is_ok
    assert ("close", str(path)) in backend.calls
    assert adapter.close(SessionParams(session=handle)).is_err


# frob:tests adapters/freecad/src/cadetta_freecad/drive.py::DriveSessions.close kind=unit
def test_close_leaves_attached_documents_open():
    backend = _DriveBackend(_items())
    backend.open_document = lambda p: Ok(DocState(path=p, attached=True))
    adapter = FreeCadAdapter(backend)
    handle = _open(adapter).unwrap().session.handle
    assert adapter.close(SessionParams(session=handle)).is_ok
    assert not [c for c in backend.calls if c[0] == "close"]


# frob:tests adapters/freecad/src/cadetta_freecad/backend.py::LiveBackend.open_document kind=unit
# frob:tests adapters/freecad/src/cadetta_freecad/backend.py::LiveBackend.get_item kind=unit
# frob:tests adapters/freecad/src/cadetta_freecad/backend.py::LiveBackend.set_item kind=unit
# frob:tests adapters/freecad/src/cadetta_freecad/backend.py::LiveBackend.recompute kind=unit
# frob:tests adapters/freecad/src/cadetta_freecad/backend.py::LiveBackend.save_document kind=unit
# frob:tests adapters/freecad/src/cadetta_freecad/backend.py::LiveBackend.close_document kind=unit
def test_live_backend_drive_methods_err_when_freecad_missing():
    live = LiveBackend()
    assert isinstance(live.open_document("x.FCStd"), Err)
    assert isinstance(live.get_item("x.FCStd", "a"), Err)
    assert isinstance(live.set_item("x.FCStd", NativeItem(kind="alias", name="a")), Err)
    assert isinstance(live.recompute("x.FCStd"), Err)
    assert isinstance(live.save_document("x.FCStd"), Err)
    assert isinstance(live.close_document("x.FCStd"), Err)


# frob:tests adapters/freecad/src/cadetta_freecad/drive.py::DriveSessions.files kind=unit
def test_read_by_session_reads_the_files_the_session_opened():
    adapter = FreeCadAdapter(_DriveBackend(_items()))
    handle = _open(adapter).unwrap().session.handle
    params = ReadParams.model_validate({"source": {"session": handle}})
    result = adapter.read(params).unwrap()
    assert result.observed is not None
