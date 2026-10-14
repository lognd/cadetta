"""Live round trip: emit, read, push, regen, save and read again under FreeCAD.

Skipped unless the `FreeCAD` module imports, so plain `pytest` stays green;
the freecad CI job runs it through run_under_freecadcmd.py.
"""

# frob:ticket 01M4SPWSQQNCC23KA25TCFVQ98

import sys
from pathlib import Path

import pytest

pytest.importorskip("FreeCAD")

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "unit"))

import _ir  # noqa: E402
from cadetta.adapter import (  # noqa: E402
    EmitParams,
    OpenParams,
    PushParams,
    ReadParams,
    RegenParams,
    SessionParams,
)

from cadetta_freecad.adapter import FreeCadAdapter  # noqa: E402
from cadetta_freecad.backend import LiveBackend  # noqa: E402


def _observed(adapter, source):
    result = adapter.read(ReadParams.model_validate({"source": source})).unwrap()
    (comp,) = result.observed.components
    return {m.path.root.split(".", 1)[1]: m.value for m in comp.members}


# frob:tests adapters/freecad/src/cadetta_freecad/backend.py::LiveBackend.freecad_version kind=integration
def test_live_backend_reaches_freecad():
    assert LiveBackend().freecad_version().is_ok


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.emit kind=integration
# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.read kind=integration
# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.push kind=integration
# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.regen kind=integration
# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.save kind=integration
def test_emit_push_regen_save_read_round_trip(tmp_path):
    out = str(tmp_path / "rover.FCStd")
    design = _ir.project(
        _ir.member("Rover/axle.d", _ir.length("12")),
        _ir.member("Rover/axle.w", _ir.length("127/20")),
    )
    bindings = [
        _ir.binding("Rover/axle.d", _ir.locator("axle_d")),
        _ir.binding("Rover/axle.w", _ir.locator("axle_w")),
    ]
    adapter = FreeCadAdapter()

    emitted = adapter.emit(
        EmitParams(design=design, bindings=bindings, policy={"out": out})
    ).unwrap()
    assert emitted.diagnostics == []
    assert Path(out).is_file()

    before = _observed(adapter, {"files": [out]})
    (comp,) = design.components
    assert before["axle_d"].nominal == comp.members[0].value.nominal
    assert before["axle_w"].nominal == comp.members[1].value.nominal

    handle = adapter.open(OpenParams(files=[out])).unwrap().session.handle
    pushed = adapter.push(
        PushParams.model_validate(
            {
                "session": handle,
                "file": out,
                "params": [
                    {"path": "a", "value": 13.5, "locator": "axle_d", "mode": "value"}
                ],
            }
        )
    ).unwrap()
    assert [s.status.value for s in pushed.statuses] == ["pushed"]
    assert (
        adapter.regen(RegenParams(session=handle, targets=[])).unwrap().diagnostics
        == []
    )
    (saved,) = adapter.save(SessionParams(session=handle)).unwrap().files
    assert saved.digest.startswith("blake3:")
    assert adapter.close(SessionParams(session=handle)).is_ok

    after = _observed(adapter, {"files": [out]})
    assert after["axle_d"].nominal != before["axle_d"].nominal
    assert after["axle_w"].nominal == before["axle_w"].nominal
