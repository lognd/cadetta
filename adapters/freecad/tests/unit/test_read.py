"""Read tests for FreeCadAdapter: a fake backend stands in for FreeCAD documents."""

# frob:ticket 01M4SPWSJ81C9Q9BV5AYXHEWE6

import _ir
from cadetta.adapter import EmitParams, ReadParams
from typani import Err, Ok

from cadetta_freecad.adapter import FreeCadAdapter
from cadetta_freecad.backend import LiveBackend, NativeDocument, NativeItem


class _DocBackend:
    """Serves canned documents, or the cells of a previous emit as aliases."""

    def __init__(self, docs=None):
        self.docs = docs or {}
        self.cells = []

    def freecad_version(self):
        return Ok("1.0.0")

    def write_sheet(self, out, sheet, cells):
        self.cells = list(cells)
        items = [
            NativeItem(kind="alias", name=c.alias, content=c.content)
            for c in cells
            if c.alias
        ]
        self.docs[out] = NativeDocument(path=out, label="rover", items=items)
        return Ok(None)

    def read_document(self, path):
        if path not in self.docs:
            return Err(f"no such file {path}")
        return Ok(self.docs[path])


def _read(backend, *files):
    params = ReadParams.model_validate({"source": {"files": list(files)}})
    return FreeCadAdapter(backend).read(params)


def _values(result):
    (comp,) = result.observed.components
    return {m.path.root.split(".", 1)[1]: m.value for m in comp.members}


# frob:tests adapters/freecad/src/cadetta_freecad/read.py::observed_from_documents kind=unit
def test_driven_dimension_is_inexact_with_a_reason():
    doc = NativeDocument(
        path="part.FCStd",
        label="part",
        items=[
            NativeItem(
                kind="dimension", name="Sketch.Ref", value=25.0, unit="mm", driven=True
            ),
            NativeItem(kind="dimension", name="Sketch.Len", value=25.0, unit="mm"),
        ],
    )
    result = _read(_DocBackend({"part.FCStd": doc}), "part.FCStd").unwrap()
    values = _values(result)
    assert values["Sketch_Ref"].exact is False
    assert "driven" in values["Sketch_Ref"].reason
    assert values["Sketch_Len"].exact is True
    assert [d.code for d in result.diagnostics] == ["Q006"]
    assert result.observed.flavour.value == "observed"


# frob:tests adapters/freecad/src/cadetta_freecad/read.py::observed_from_documents kind=unit
def test_emit_then_read_round_trips_ir_values():
    design = _ir.project(
        _ir.member("Rover/axle.d", _ir.length("12029/1000")),
        _ir.member("Rover/axle.w", _ir.length("127/20")),
        _ir.member("Rover/axle.n", _ir.count(4)),
    )
    bindings = [
        _ir.binding("Rover/axle.d", _ir.locator("axle_d")),
        _ir.binding("Rover/axle.w", _ir.locator("axle_w")),
        _ir.binding("Rover/axle.n", _ir.locator("axle_n")),
    ]
    backend = _DocBackend()
    adapter = FreeCadAdapter(backend)
    params = EmitParams(design=design, bindings=bindings, policy={"out": "r.FCStd"})
    assert adapter.emit(params).is_ok
    observed = _values(_read(backend, "r.FCStd").unwrap())
    (comp,) = design.components
    by_alias = {"d": "axle_d", "w": "axle_w", "n": "axle_n"}
    for m in comp.members:
        want = m.value
        got = observed[by_alias[m.path.root.rsplit(".", 1)[1]]]
        assert got.exact and want.exact
        assert got.nominal == want.nominal
        assert got.dim == want.dim
        assert got.angle == want.angle


# frob:tests adapters/freecad/src/cadetta_freecad/read.py::observed_from_documents kind=unit
def test_long_binary64_is_inexact_and_unit_less_dimension_is_assumed():
    doc = NativeDocument(
        path="p.FCStd",
        label="p",
        items=[
            NativeItem(kind="dimension", name="S.A", value=0.1 + 0.2, unit="mm"),
            NativeItem(kind="dimension", name="S.B", value=3.0),
            NativeItem(kind="alias", name="S.A", content="=GLOBAL"),
            NativeItem(kind="dimension", name="S-B", value=1.5, unit="deg"),
        ],
    )
    result = _read(_DocBackend({"p.FCStd": doc}), "p.FCStd").unwrap()
    values = _values(result)
    assert values["S_A"].exact is False
    assert "short decimal" in values["S_A"].reason
    assert values["S_B"].unit_assumed == "mm"
    assert values["S_B_2"].angle == 1
    codes = sorted(d.code for d in result.diagnostics)
    assert codes == ["Q006", "R002"]
    (binding,) = [b for b in result.observed.bindings if b.path.root.endswith("S_B")]
    assert binding.locators[0].native_unit == "mm"


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.read kind=unit
def test_read_rejects_sessions_and_reports_backend_errors():
    adapter = FreeCadAdapter(_DocBackend())
    session = ReadParams.model_validate({"source": {"session": "s1"}})
    assert adapter.read(session).danger_err.code == -32602
    assert adapter.read(ReadParams.model_validate({"source": {"files": ["x"]}})).is_err


# frob:tests adapters/freecad/src/cadetta_freecad/backend.py::LiveBackend.read_document kind=unit
def test_live_backend_read_errs_when_freecad_missing():
    assert isinstance(LiveBackend().read_document("x.FCStd"), Err)
