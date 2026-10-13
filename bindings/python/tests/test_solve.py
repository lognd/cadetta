"""The solver decorator: protocol round trip, kind checks, decimal output."""

# frob:ticket 01M4SV2NW9YYPTWAJQ9P7MKG5E

import io
import json

from cadetta import Quantity
from cadetta.solve import SolverProtocolError, solver


def req(inputs: dict) -> str:
    return json.dumps({"schema": "cadetta.solve/1", "solver": "t::s", "inputs": inputs})


LOAD = {"lit": "1200N", "si": 1200.0}
SPAN = {"lit": "300mm", "si": 0.3, "min": 0.2995, "max": 0.3005}


@solver(
    inputs={"load": "Force", "span": "Length"},
    outputs={"moment": "Torque", "half": "Length"},
    significant={"moment": 4},
    version="1.2.3",
)
def beam(load: Quantity, span: Quantity):
    return {"moment": (load * span / 4).as_kind("Torque"), "half": span / 2}


# frob:tests bindings/python/python/cadetta/solve.py::Solver.serve kind=unit
def test_serve_writes_valid_response_with_version():
    out = io.StringIO()
    code = beam.serve(io.StringIO(req({"load": LOAD, "span": SPAN})), out)
    assert code == 0
    resp = json.loads(out.getvalue())
    assert resp["schema"] == "cadetta.solve/1"
    assert resp["version"] == "1.2.3"
    assert set(resp["outputs"]) == {"moment", "half"}


# frob:tests bindings/python/python/cadetta/solve.py::Solver.handle kind=unit
def test_toleranced_input_arrives_with_tolerance():
    seen = {}

    @solver(inputs={"span": "Length"}, outputs={"w": "Length"})
    def probe(span):
        seen["min"] = span.min.to_float("mm")
        seen["max"] = span.max.to_float("mm")
        return {"w": span}

    assert probe.handle(req({"span": SPAN})).is_ok
    assert abs(seen["min"] - 299.5) < 1e-9 and abs(seen["max"] - 300.5) < 1e-9


# frob:tests bindings/python/python/cadetta/solve.py::Solver.handle kind=unit
def test_input_kind_mismatch_is_protocol_error_naming_input():
    bad = {"lit": "1200s", "si": 1200.0}
    r = beam.handle(req({"load": bad, "span": SPAN}))
    assert r.is_err
    assert isinstance(r.unwrap_err(), SolverProtocolError)
    assert "load" in str(r.unwrap_err())


# frob:tests bindings/python/python/cadetta/solve.py::Solver.handle kind=unit
def test_output_kind_mismatch_is_protocol_error_naming_output():
    @solver(inputs={"span": "Length"}, outputs={"t": "Time"})
    def wrong(span):
        return {"t": span}

    r = wrong.handle(req({"span": SPAN}))
    assert r.is_err and "`t`" in str(r.unwrap_err())
    out = io.StringIO()
    assert wrong.serve(io.StringIO(req({"span": SPAN})), out) == 1
    assert out.getvalue() == ""


# frob:tests bindings/python/python/cadetta/solve.py::format_output kind=unit
def test_outputs_are_rounded_decimal_strings_not_repr_float():
    third = {"lit": "1m", "si": 1.0}

    @solver(
        inputs={"x": "Length"},
        outputs={"a": "Length", "b": "Length"},
        significant={"b": 3},
    )
    def thirds(x):
        return {"a": x / 3, "b": x / 3}

    out = json.loads(thirds.handle(req({"x": third})).unwrap())["outputs"]
    assert out["a"] == "0.333333m"
    assert out["b"] == "0.333m"


# frob:tests bindings/python/python/cadetta/solve.py::format_output kind=unit
def test_inexact_and_large_values_have_no_exponent():
    @solver(inputs={"x": "Area"}, outputs={"r": "Length", "big": "Length"})
    def f(x):
        return {"r": x.sqrt(), "big": x * 1000000 / x.nominal * x.sqrt()}

    r = f.handle(req({"x": {"lit": "2m^2", "si": 2.0}}))
    out = json.loads(r.unwrap())["outputs"]
    assert out["r"] == "1.41421m"
    assert "e" not in out["big"].lower().replace("m", "")


# frob:tests bindings/python/python/cadetta/solve.py::Solver.handle kind=unit
def test_bad_requests_are_protocol_errors():
    assert beam.handle("not json").is_err
    assert beam.handle(
        json.dumps({"schema": "cadetta.solve/2", "solver": "s", "inputs": {}})
    ).is_err
    assert beam.handle(req({"load": LOAD})).is_err
