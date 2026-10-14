"""Emit tests for FreeCadAdapter: a fake backend records the Spreadsheet writes."""

# frob:ticket 01M4SPWSFF7Z2K6FSGSZ1N30JH

import io
import json
from fractions import Fraction

import _ir
from cadetta.adapter import EmitParams
from typani import Err, Ok

from cadetta_freecad.adapter import FreeCadAdapter
from cadetta_freecad.backend import LiveBackend
from cadetta_freecad.emit import CellWrite, SheetPlan, digest_plan, to_decimal


class _RecordingBackend:
    """Records every write_sheet call instead of touching FreeCAD."""

    def __init__(self):
        self.calls = []

    def freecad_version(self):
        return Ok("1.0.0")

    def write_sheet(self, out, sheet, cells):
        self.calls.append((out, sheet, list(cells)))
        return Ok(None)


def _emit(design, bindings, **policy):
    backend = _RecordingBackend()
    policy.setdefault("out", "rover.FCStd")
    params = EmitParams(design=design, bindings=[b for b in bindings], policy=policy)
    return FreeCadAdapter(backend).emit(params), backend


def _by_alias(backend):
    (_out, _sheet, cells) = backend.calls[-1]
    return {c.alias: c.content for c in cells if c.alias}


def _design():
    return _ir.project(
        _ir.member("Rover/axle.d", _ir.length("12", authored="12mm")),
        _ir.member("Rover/axle.n", _ir.count(4)),
    )


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.emit kind=unit
def test_emit_twice_is_deterministic():
    bindings = [_ir.binding("Rover/axle.d", _ir.locator("axle_d"))]
    first, b1 = _emit(_design(), bindings)
    second, b2 = _emit(_design(), bindings)
    assert first == second
    assert b1.calls == b2.calls
    assert first.unwrap().files[0].digest.startswith("blake3:")


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_emit_twice_cells_are_identical_and_headed_by_digests():
    bindings = [_ir.binding("Rover/axle.d", _ir.locator("axle_d"))]
    _emit(_design(), bindings)
    _r, backend = _emit(_design(), bindings)
    cells = backend.calls[0][2]
    assert cells[0].content.startswith("cadetta emit freecad ")
    assert cells[1].content == "ir=blake3:ab12"
    assert cells[2].content.startswith("bindings=blake3:")


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::to_decimal kind=unit
def test_to_decimal_exact_and_trimmed():
    assert to_decimal(Fraction(127, 20), 6) == Ok("6.35")
    assert to_decimal(Fraction(12), 6) == Ok("12")
    assert to_decimal(Fraction(-1, 2), 3) == Ok("-0.5")
    assert to_decimal(Fraction(1, 1000), 3) == Ok("0.001")


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::to_decimal kind=unit
def test_to_decimal_never_rounds():
    assert isinstance(to_decimal(Fraction(1, 3), 6), Err)
    assert isinstance(to_decimal(Fraction(1, 1000), 2), Err)


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_q004_withholds_the_file():
    design = _ir.project(_ir.member("Rover/axle.d", _ir.length("1/3")))
    result, backend = _emit(design, [_ir.binding("Rover/axle.d", _ir.locator("d"))])
    emitted = result.unwrap()
    assert [d.code for d in emitted.diagnostics] == ["Q004"]
    assert emitted.diagnostics[0].path == "Rover/axle.d"
    assert emitted.files == []
    assert backend.calls == []


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_q004_respects_policy_precision():
    design = _ir.project(_ir.member("Rover/axle.d", _ir.length("127/20")))
    bindings = [_ir.binding("Rover/axle.d", _ir.locator("d"))]
    result, _ = _emit(design, bindings, precision=1)
    assert [d.code for d in result.unwrap().diagnostics] == ["Q004"]
    result, backend = _emit(design, bindings, precision=2)
    assert result.unwrap().diagnostics == []
    assert _by_alias(backend) == {"d": "=6.35 mm"}


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.emit kind=unit
def test_several_locators_each_written_and_ref_mode_references_global():
    bindings = [
        _ir.binding(
            "Rover/axle.d",
            _ir.locator("sk_d", kind="dimension", mode="ref"),
            _ir.locator("axle_d"),
            _ir.locator("sk_d2", kind="dimension"),
        )
    ]
    result, backend = _emit(_design(), bindings)
    assert result.unwrap().diagnostics == []
    assert _by_alias(backend) == {
        "axle_d": "=12 mm",
        "sk_d": "=axle_d",
        "sk_d2": "=12 mm",
    }
    (out, sheet, cells) = backend.calls[0]
    assert (out, sheet) == ("rover.FCStd", "cadetta")
    # Globals are written before dimensions.
    aliases = [c.alias for c in cells if c.alias]
    assert aliases == ["axle_d", "sk_d", "sk_d2"]


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_ref_to_names_the_target_global():
    bindings = [
        _ir.binding(
            "Rover/axle.d",
            _ir.locator("sk_d", kind="dimension", mode="ref", ref_to="shared_d"),
        )
    ]
    _result, backend = _emit(_design(), bindings)
    assert _by_alias(backend) == {"sk_d": "=shared_d"}


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_ref_mode_without_global_creates_the_mangled_global_once():
    bindings = [
        _ir.binding(
            "Rover/axle.d",
            _ir.locator("sk_a", kind="dimension", mode="ref"),
            _ir.locator("sk_b", kind="dimension", mode="ref"),
        )
    ]
    _result, backend = _emit(
        _design(), bindings, names={"Rover/axle.d": "Rover__axle__d"}
    )
    assert _by_alias(backend) == {
        "Rover__axle__d": "=12 mm",
        "sk_a": "=Rover__axle__d",
        "sk_b": "=Rover__axle__d",
    }


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_ref_mode_follows_alias_of_to_the_referent_global():
    bindings = [
        _ir.binding("Rover/axle.d", _ir.locator("axle_d")),
        _ir.binding("Rover/axle.n", _ir.locator("sk_n", kind="dimension", mode="ref")),
    ]
    _result, backend = _emit(
        _design(), bindings, alias_of={"Rover/axle.n": "Rover/axle.d"}
    )
    assert _by_alias(backend)["sk_n"] == "=axle_d"


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_ref_mode_without_any_global_is_a_diagnostic():
    bindings = [
        _ir.binding("Rover/axle.d", _ir.locator("sk", kind="dimension", mode="ref"))
    ]
    result, backend = _emit(_design(), bindings)
    assert [d.code for d in result.unwrap().diagnostics] == ["R015"]
    assert backend.calls == []


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_sign_negates_and_dimensionless_is_bare():
    bindings = [
        _ir.binding("Rover/axle.d", _ir.locator("neg_d", sign=-1)),
        _ir.binding("Rover/axle.n", _ir.locator("n")),
    ]
    _result, backend = _emit(_design(), bindings)
    assert _by_alias(backend) == {"neg_d": "=-12 mm", "n": "4"}


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_alternate_only_and_unwritten_locators_are_skipped():
    bindings = [
        _ir.binding(
            "Rover/axle.d",
            _ir.locator("axle_d"),
            _ir.locator("alt_d", **{"for": "BEARING=B"}),
            _ir.locator("Material", kind="property"),
        )
    ]
    _result, backend = _emit(_design(), bindings)
    assert list(_by_alias(backend)) == ["axle_d"]


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_emit_point_mid_mmc_lmc_and_notes():
    design = _ir.project(
        _ir.member(
            "Rover/axle.d",
            _ir.length("12", authored="12mm +0.1/-0.2", lo="-0.2", hi="0.1"),
            role="hole",
            attrs={"emit": "mmc"},
        ),
    )
    bindings = [_ir.binding("Rover/axle.d", _ir.locator("d"))]
    _result, backend = _emit(design, bindings)
    assert _by_alias(backend) == {"d": "=11.8 mm"}
    notes = [c.content for c in backend.calls[0][2] if c.cell.startswith("C")][-1]
    assert "12mm +0.1/-0.2" in notes
    assert "emit=mmc" in notes
    _result, backend = _emit(design, bindings, point="lmc")
    # The member attribute still wins over the policy default.
    assert _by_alias(backend) == {"d": "=11.8 mm"}
    plain = _ir.project(
        _ir.member("Rover/axle.d", _ir.length("12", lo="-0.2", hi="0.1"))
    )
    _result, backend = _emit(plain, bindings, point="mid")
    assert _by_alias(backend) == {"d": "=11.95 mm"}


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_mmc_without_role_is_e010_and_withholds():
    design = _ir.project(
        _ir.member("Rover/axle.d", _ir.length("12", lo="-0.2", hi="0.1"))
    )
    result, backend = _emit(
        design, [_ir.binding("Rover/axle.d", _ir.locator("d"))], point="mmc"
    )
    assert [d.code for d in result.unwrap().diagnostics] == ["E010"]
    assert backend.calls == []


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_star_binding_emits_instance_zero():
    design = _ir.project(_ir.member("Rover/axle[0].d", _ir.length("12")))
    _result, backend = _emit(design, [_ir.binding("Rover/axle[*].d", _ir.locator("d"))])
    assert _by_alias(backend) == {"d": "=12 mm"}


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_invalid_and_colliding_aliases_are_diagnostics():
    for name in ("A1", "bad name", "1abc"):
        result, backend = _emit(
            _design(), [_ir.binding("Rover/axle.d", _ir.locator(name))]
        )
        assert [d.code for d in result.unwrap().diagnostics] == ["R015"], name
        assert backend.calls == []
    result, backend = _emit(
        _design(),
        [
            _ir.binding("Rover/axle.d", _ir.locator("x")),
            _ir.binding("Rover/axle.n", _ir.locator("x")),
        ],
    )
    assert [d.code for d in result.unwrap().diagnostics] == ["R013"]


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_paths_follow_dependencies_then_smallest_path():
    design = _ir.project(
        _ir.member("Rover/axle.a", _ir.length("1"), deps=("Rover/axle.z",)),
        _ir.member("Rover/axle.m", _ir.length("2")),
        _ir.member("Rover/axle.z", _ir.length("3")),
    )
    bindings = [
        _ir.binding("Rover/axle.a", _ir.locator("a")),
        _ir.binding("Rover/axle.m", _ir.locator("m")),
        _ir.binding("Rover/axle.z", _ir.locator("z")),
    ]
    _result, backend = _emit(design, bindings)
    assert list(_by_alias(backend)) == ["m", "z", "a"]


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_dependency_cycle_is_d003():
    design = _ir.project(
        _ir.member("Rover/axle.a", _ir.length("1"), deps=("Rover/axle.b",)),
        _ir.member("Rover/axle.b", _ir.length("2"), deps=("Rover/axle.a",)),
    )
    bindings = [
        _ir.binding("Rover/axle.a", _ir.locator("a")),
        _ir.binding("Rover/axle.b", _ir.locator("b")),
    ]
    result, backend = _emit(design, bindings)
    assert [d.code for d in result.unwrap().diagnostics] == ["D003"]
    assert backend.calls == []


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::build_plan kind=unit
def test_unknown_bound_path_is_r005():
    result, _ = _emit(_design(), [_ir.binding("Rover/axle.nope", _ir.locator("x"))])
    assert [d.code for d in result.unwrap().diagnostics] == ["R005"]


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.emit kind=unit
def test_bad_policy_and_digestless_design_are_invalid_params():
    adapter = FreeCadAdapter(_RecordingBackend())
    bad = EmitParams(design=_design(), bindings=[], policy={"nope": 1})
    assert adapter.emit(bad).unwrap_err().code == -32602
    no_digest = EmitParams(
        design=_ir.project(digest=None), bindings=[], policy={"out": "x.FCStd"}
    )
    assert adapter.emit(no_digest).unwrap_err().code == -32602


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.emit kind=unit
def test_backend_failure_is_an_rpc_error():
    class Failing(_RecordingBackend):
        def write_sheet(self, out, sheet, cells):
            return Err("disk full")

    params = EmitParams(
        design=_design(),
        bindings=[_ir.binding("Rover/axle.d", _ir.locator("d"))],
        policy={"out": "x.FCStd"},
    )
    assert FreeCadAdapter(Failing()).emit(params).unwrap_err().message == "disk full"


# frob:tests adapters/freecad/src/cadetta_freecad/adapter.py::FreeCadAdapter.emit kind=unit
def test_emit_over_json_rpc():
    backend = _RecordingBackend()
    request = {
        "jsonrpc": "2.0",
        "id": 7,
        "method": "emit",
        "params": {
            "design": json.loads(_design().model_dump_json(by_alias=True)),
            "bindings": [_ir.binding("Rover/axle.d", _ir.locator("d"))],
            "policy": {"out": "rover.FCStd"},
        },
    }
    stdin = io.StringIO(json.dumps(request) + "\n")
    stdout = io.StringIO()
    assert FreeCadAdapter(backend).serve(stdin, stdout) == 0
    reply = json.loads(stdout.getvalue().splitlines()[0])
    assert reply["result"]["files"][0]["path"] == "rover.FCStd"
    assert _by_alias(backend) == {"d": "=12 mm"}


# frob:tests adapters/freecad/src/cadetta_freecad/emit.py::digest_plan kind=unit
def test_digest_plan_is_stable_and_content_sensitive():
    a = SheetPlan(cells=[CellWrite(cell="A1", content="x", alias="a")])
    b = SheetPlan(cells=[CellWrite(cell="A1", content="y", alias="a")])
    assert digest_plan(a) == digest_plan(SheetPlan(cells=list(a.cells)))
    assert digest_plan(a) != digest_plan(b)


# frob:tests adapters/freecad/src/cadetta_freecad/backend.py::LiveBackend.write_sheet kind=unit
def test_live_backend_write_sheet_reports_missing_freecad():
    result = LiveBackend().write_sheet("x.FCStd", "cadetta", [])
    assert isinstance(result, Err)
    assert "not importable" in result.unwrap_err()
