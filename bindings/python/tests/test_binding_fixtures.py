"""Replay the `kind = binding` conformance fixtures against the Python API.

Every pair under tests/conformance/bindings is `entry = python`: one API call
(`api`, `args`, optional staged `files`, call context) with a `must_fire` and
a `must_not_fire` expectation block (tests/conformance/README.md, "Binding
fixtures"). Each side is executed, reduced to an observation dict, and every
key the expectation block names must match the observation.

A few `must_fire` blocks describe what a *violating* binding would show (a
`cli-only` verb that is a method, a reshaped query document, a float-rounded
solver output). A correct binding cannot reproduce those, so for the
fixtures in VIOLATIONS the must_fire side asserts the binding does NOT show
the described outcome.
"""

# frob:ticket 01M4XB33RBF6FWZ1GJ3CDRJTN9

from __future__ import annotations

import io
import json
import math
import operator
import re
import subprocess
import sys
from fractions import Fraction
from pathlib import Path
from typing import Any

import pytest
from cadetta import Project, Quantity, format_source, pint_interop, quantity
from cadetta.adapter import Adapter
from cadetta.errors import QuantityError
from cadetta.logging import get_logger
from cadetta.solve import solver
from typani import Err, Ok

_log = get_logger(__name__)

CORPUS = Path(__file__).resolve().parents[3] / "tests" / "conformance" / "bindings"
SIDES = ("must-fire", "must-not-fire")
IGNORED_KEYS = {"diagnostics", "why", "note"}
# must_fire blocks that describe a violating binding (see module docstring).
# "name/side" -> follow-up ticket: the binding genuinely diverges from the fixture.
KNOWN_DIVERGENCES = {
    "pint-currency-rejected/must-fire": "~98TQVDQ",
}
VIOLATIONS = {"cli-only-verbs", "query-parity-with-cli", "solver-decimal-strings"}

Obs = dict[str, Any]


def _fixtures() -> list[str]:
    """Names of every binding fixture in the corpus, sorted."""
    return sorted(
        p.name.removesuffix(".expect.json") for p in CORPUS.glob("*.expect.json")
    )


def _load(name: str, side: str) -> tuple[dict, dict]:
    """The call spec and the expectation block of one side of a fixture."""
    expect = json.loads((CORPUS / f"{name}.expect.json").read_text(encoding="utf-8"))
    spec = json.loads((CORPUS / f"{name}.{side}.json").read_text(encoding="utf-8"))
    return spec, expect[side.replace("-", "_")]


def _stage(spec: dict, root: Path) -> None:
    """Write the fixture's package tree under `root`."""
    for rel, text in spec.get("files", {}).items():
        target = root / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")


def _q(text: str) -> Quantity:
    """Parse a fixture quantity; a trailing ` as Kind` applies `as_kind`."""
    literal, _, kind = text.partition(" as ")
    parsed = quantity(literal).unwrap()
    return parsed.as_kind(kind) if kind else parsed


def _arg(value: Any) -> Any:
    """Quantity source literals become quantities; numbers pass through raw."""
    return _q(value) if isinstance(value, str) else value


_SI = re.compile(r"si_nominal=([^,]+),")
_OPS = {
    "__add__": operator.add,
    "__sub__": operator.sub,
    "__mul__": operator.mul,
    "__truediv__": operator.truediv,
}


def _si(q: Quantity) -> float:
    """The SI nominal magnitude, read from the Quantity repr (no float accessor w/o unit)."""
    return float(_SI.search(repr(q)).group(1))  # type: ignore[union-attr]


def _same(got: Quantity, want: str) -> bool:
    """Same dimension and SI nominal magnitude as the expected literal."""
    expected = _q(want)
    return got.dim == expected.dim and math.isclose(
        _si(got), _si(expected), rel_tol=1e-9
    )


def _describe(value: Any) -> Obs:
    """Observation keys for a successful value."""
    obs: Obs = {"value": value}
    if hasattr(value, "nominal") and hasattr(value, "min"):
        obs |= {
            "kind": value.kind,
            "nominal": value.nominal,
            "min": value.min,
            "max": value.max,
            "has_tolerance": _si(value.min) != _si(value.max),
        }
        obs["value"] = None
    elif isinstance(value, Fraction):
        obs["value"] = f"{value.numerator}/{value.denominator}"
    return obs


def _from_result(result: Any) -> Obs:
    """Map a typani Result to `result`, `exit`, `error` and the value keys."""
    if isinstance(result, Err):
        err = result.unwrap_err()
        message = getattr(err, "message", str(err))
        usage = (
            "usage"
            if "usage" in message.lower() or "workspace" in message.lower()
            else ""
        )
        return {
            "result": "Err",
            "exit": 2,
            "error": type(err).__name__,
            "error_text": f"{type(err).__name__} {usage} {message}".lower(),
        }
    value = result.unwrap()
    return {"result": "Ok", "exit": 0} | _describe_ok(value)


def _describe_ok(value: Any) -> Obs:
    """Ok-side keys: quantity/member kind, package, schema, check findings."""
    obs: Obs = {}
    if hasattr(value, "diagnostics"):
        errors = any(d.severity.value == "error" for d in value.diagnostics)
        obs |= {"exit": 1 if errors else 0, "report_has_error_diagnostic": errors}
    elif hasattr(value, "schema_"):
        obs["schema"] = value.schema_
    elif isinstance(value, Project):
        pkg = value.package
        obs |= {"package": pkg.name if pkg else None, "modules": value.modules}
    elif isinstance(value, str):
        obs["value"] = value
    elif hasattr(value, "kind"):
        obs["kind"] = value.kind
    return obs


def _quantity_call(spec: dict) -> Obs:
    """Drive `Quantity.<method>` on the first argument."""
    method = spec["api"].split(".", 1)[1]
    recv, *rest = spec["args"]
    q = _q(recv)
    for step in spec.get("pre", []):
        q = getattr(q, step)()
    if spec.get("role"):
        q = q.with_role(spec["role"])
    try:
        if method in {"nominal", "min", "max", "mid"}:
            out = getattr(q, method)
        elif method == "mmc":
            return {"result": "value", "mmc": q.mmc}
        elif method in _OPS:
            out = _OPS[method](q, _arg(rest[0]))
        else:
            out = getattr(q, method)(*rest)
    except QuantityError as exc:
        return {"result": "raises", "QuantityError": exc.code, "code": exc.code}
    except TypeError as exc:
        return {"result": "raises", "TypeError": str(exc)}
    except AttributeError as exc:
        return {"result": "raises", "AttributeError": exc.name}
    return {"result": "value"} | _describe(out)


def _project_obs(spec: dict, root: Path) -> Obs:
    """Drive `Project.*`, `Design.*` and `Member.*` calls on the staged root."""
    api = spec["api"]
    args = spec["args"]
    if api == "Project.open":
        opened = Project.open(root / args[0])
        obs = _from_result(opened)
        if opened.is_ok:
            obs["evaluate_exit"] = 0 if opened.unwrap().evaluate().is_ok else 2
        return obs
    opened = Project.open(root)
    project = opened.unwrap()
    if api == "Project.open(.).check":
        return _from_result(project.check())
    if api == "Project.check":
        return _parity_obs(project, root)
    if api == "Project.modules":
        return {"result": "value", "value": project.modules}
    if api == "Project.evaluate":
        return _from_result(project.evaluate())
    if api == "Project.reload":
        return _reload_obs(spec, project, root)
    design = project.evaluate().unwrap()
    if api == "Design.__getitem__":
        try:
            design[args[0]]
        except TypeError as exc:
            return {"result": "raises", "TypeError": str(exc)}
        return _from_result(design.get(args[0]))
    if api == "Design.get":
        return _from_result(design.get(args[0]))
    member = design.get(args[0]).unwrap()
    if api == "Member.kind":
        return {"result": "value", "value": member.kind}
    if api == "Member.value.kind":
        kind = member.value.kind
        return {"result": "value", "value": kind, "anonymous": kind is None}
    raise AssertionError(f"no replay for {api}")


def _parity_obs(project: Project, root: Path) -> Obs:
    """Compare the binding's check document with `cadetta check --format json`."""
    report = project.check().unwrap()
    mine = report.model_dump(mode="json", by_alias=True, exclude_unset=True)
    done = subprocess.run(
        [
            sys.executable,
            "-m",
            "cadetta",
            "--format",
            "json",
            "--cwd",
            str(root),
            "check",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    return {
        "result": "Ok",
        "exit": 0,
        "equals_cli_json": mine == json.loads(done.stdout),
    }


def _reload_obs(spec: dict, project: Project, root: Path) -> Obs:
    """A held Design is a snapshot: stale after an edit, fresh after reload."""
    held = project.evaluate().unwrap()
    path = "acme.demo::Rover.w"
    for rel, text in spec["edit"].items():
        (root / rel).write_text(text, encoding="utf-8")
    stale = held.get(path).unwrap().value
    project.reload().unwrap()
    fresh = project.evaluate().unwrap().get(path).unwrap().value
    return {"result": "value", "value": stale if "before_reload" in spec else fresh}


def _module_obs(spec: dict) -> Obs:
    """Module-level functions: `cadetta.quantity`, `format_source`, `to_pint`."""
    api, args = spec["api"], spec["args"]
    if api == "cadetta.quantity":
        return _from_result(quantity(args[0]))
    if api == "cadetta.format_source":
        out = format_source(args[0]).unwrap()
        again = format_source(out).unwrap()
        return {
            "result": "Ok",
            "exit": 0,
            "changed": out != args[0],
            "idempotent": again == out,
        }
    if api == "pint_interop.to_pint":
        pint = pytest.importorskip("pint")
        q = quantity(args[0]).unwrap()
        try:
            res = pint_interop.to_pint(q, pint.UnitRegistry())
        except QuantityError as exc:
            return {"result": "raises", "error": type(exc).__name__}
        obs = _from_result(res)
        if res.is_ok:
            obs["tolerance_dropped"] = q.min != q.max
        return obs
    if api == "hasattr":
        owner = {"Project": Project}[args[0]]
        return {"result": "value", "value": hasattr(owner, args[1])}
    raise AssertionError(f"no replay for {api}")


def _serve_obs(spec: dict) -> Obs:
    """Drive a decorated solver through one cadetta.solve/1 request."""
    ((out_name, out_kind),) = spec["outputs"].items()
    returns = spec["returns"]
    inputs = spec.get("inputs", {})

    @solver(
        inputs=inputs,
        outputs=spec["outputs"],
        significant=spec.get("significant", 12),
    )
    def run(**_: Any) -> dict[str, Any]:
        return {out_name: quantity(returns[out_name]).unwrap()}

    request = {
        "schema": "cadetta.solve/1",
        "solver": "t",
        "inputs": {n: {"lit": "1N", "si": 1.0} for n in inputs},
    }
    res = run.handle(json.dumps(request))
    if res.is_err:
        msg = str(res.unwrap_err())
        return {
            "result": "protocol-error",
            "exit": 1,
            "names": out_name if out_name in msg else "",
        }
    text = json.loads(res.unwrap())["outputs"][out_name]
    return {"result": "value", "exit": 0, "wall_output": text}


def _adapter_obs(spec: dict) -> Obs:
    """Run an adapter whose handler prints; see whether stdout stays pure JSON-RPC."""

    class Chatty(Adapter):
        def shutdown(self):  # noqa: ANN202
            print("hello from the adapter")
            return Ok(None)

    line = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "shutdown"}) + "\n"
    out = io.StringIO()
    real_out, real_in = sys.stdout, sys.stdin
    try:
        sys.stdin, sys.stdout = io.StringIO(line), out
        if spec["logs"] == "stdout":
            Chatty().serve(sys.stdin, sys.stdout)
        else:
            Chatty.run()
    finally:
        sys.stdin, sys.stdout = real_in, real_out
    lines = [ln for ln in out.getvalue().splitlines() if ln.strip()]

    def rpc(text: str) -> bool:
        try:
            return json.loads(text).get("jsonrpc") == "2.0"
        except (json.JSONDecodeError, AttributeError):
            return False

    return {"result": "value", "stdout_all_json_rpc": all(rpc(ln) for ln in lines)}


def observe(spec: dict, root: Path) -> Obs:
    """Execute one fixture side and reduce it to an observation."""
    _stage(spec, root)
    api = spec["api"]
    if api.startswith("Quantity."):
        return _quantity_call(spec)
    if api == "solver.serve":
        return _serve_obs(spec)
    if api == "Adapter.serve":
        return _adapter_obs(spec)
    if api.split(".")[0] in {"Project", "Design", "Member"}:
        return _project_obs(spec, root)
    return _module_obs(spec)


def _norm(key: str, value: Any) -> Any:
    """Compare units/spacing loosely where the spec writes `3.14 mm`."""
    return (
        value.replace(" ", "")
        if key == "wall_output" and isinstance(value, str)
        else value
    )


def _matches(obs: Obs, expect: dict) -> list[str]:
    """Keys of the expectation that the observation does not satisfy."""
    bad = []
    for key, want in expect.items():
        if key in IGNORED_KEYS:
            continue
        got = obs.get(key)
        if key == "TypeError":
            ok = (
                got is not None
            )  # CPython words the message; the spec only fixes the type
        elif key == "error":
            ok = want in str(got) or want in obs.get("error_text", "") or got == want
        elif key == "wall_output":
            ok = _same_quantity(got, want)
        elif isinstance(got, Quantity) and isinstance(want, str):
            ok = _same(got, want)
        else:
            ok = got == want
        if not ok:
            bad.append(f"{key}: expected {want!r}, observed {got!r}")
    return bad


def _same_quantity(got: Any, want: str) -> bool:
    """True when two decimal literals denote the same SI length."""
    if not isinstance(got, str):
        return False
    a, b = quantity(got).unwrap(), quantity(want.replace(" ", "")).unwrap()
    return a.to_exact("m") == b.to_exact("m")


@pytest.mark.parametrize("side", SIDES)
@pytest.mark.parametrize("name", _fixtures())
def test_binding_fixture(name: str, side: str, tmp_path: Path) -> None:
    """Each binding fixture side behaves as its expect.json block states."""
    spec, expect = _load(name, side)
    obs = observe(spec, tmp_path)
    _log.info("%s/%s observed %r", name, side, obs)
    bad = _matches(obs, expect)
    if f"{name}/{side}" in KNOWN_DIVERGENCES:
        ticket = KNOWN_DIVERGENCES[f"{name}/{side}"]
        assert bad, (
            f"{name}/{side} now conforms: drop its KNOWN_DIVERGENCES entry ({ticket})"
        )
        pytest.xfail(f"known binding divergence, see {ticket}: {bad}")
    if name in VIOLATIONS and side == "must-fire":
        assert bad, f"{name}: a correct binding must not show the violation {expect}"
    else:
        assert not bad, f"{name}/{side}: " + "; ".join(bad)


def test_every_fixture_is_replayed() -> None:
    """The corpus holds exactly the 35 binding pairs, each with both sides."""
    names = _fixtures()
    assert len(names) == 35
    for name in names:
        for side in SIDES:
            assert (CORPUS / f"{name}.{side}.json").is_file()
