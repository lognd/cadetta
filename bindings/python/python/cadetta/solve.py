"""Write a solver as a decorated Python function (bindings.md 3.3, solvers.md 3)."""

# frob:ticket 01M4SV2NW9YYPTWAJQ9P7MKG5E

from __future__ import annotations

import json
import re
import sys
from collections.abc import Callable, Mapping
from decimal import ROUND_HALF_EVEN, Context, Decimal
from typing import Any, TextIO

from pydantic import BaseModel, ConfigDict, Field, ValidationError
from typani import Err, Ok, Result

from cadetta._core import Quantity, QuantityError
from cadetta.logging import get_logger, use_protocol_logging
from cadetta.quantity import quantity

__all__ = ["SCHEMA", "Solver", "SolverProtocolError", "solver"]

SCHEMA = "cadetta.solve/1"
DEFAULT_SIGNIFICANT = 6

_log = get_logger(__name__)

# SI base units in the order of `Quantity.dim` (L, M, T, I, Theta, N, J).
_BASE_UNITS = ("m", "kg", "s", "A", "K", "mol", "cd")
_NUMBER_UNIT = re.compile(r"^([+-]?\d+(?:\.\d+)?)(\S+)$")


class SolverProtocolError(Exception):
    """A request, kind or output broke cadetta.solve/1; the message names it."""


class _InputValue(BaseModel):
    """One request input: `{lit, si, min?, max?}` (solvers.md 3)."""

    model_config = ConfigDict(extra="forbid")

    lit: str
    si: float
    min: float | None = None
    max: float | None = None


class _Request(BaseModel):
    """A cadetta.solve/1 request."""

    model_config = ConfigDict(extra="forbid")

    schema_: str = Field(alias="schema")
    solver: str
    inputs: dict[str, _InputValue]


def _fail(msg: str) -> Err[SolverProtocolError]:
    """Log and build the protocol-error result."""
    _log.error("protocol error: %s", msg)
    return Err(SolverProtocolError(msg))


def _decimal_text(value: Decimal) -> str:
    """Fixed-point text of a decimal (no exponent), trailing zeros dropped."""
    text = format(value.normalize(), "f")
    return "0" if text in ("-0", "") else text


def _tolerance_delta(si: float, bound: float, number: Decimal) -> str:
    """Delta from the nominal to `bound`, in the literal's own unit (12 digits)."""
    si_d = Decimal(repr(si))
    delta = abs(Decimal(repr(bound)) - si_d) / abs(si_d) * abs(number)
    return _decimal_text(Context(prec=12).create_decimal(delta))


def _build_quantity(
    name: str, spec: _InputValue
) -> Result[Quantity, SolverProtocolError]:
    """Turn a request input into a Quantity, rebuilding any tolerance."""
    text = spec.lit
    if spec.min is not None and spec.max is not None:
        m = _NUMBER_UNIT.match(spec.lit.strip())
        if m is None or spec.si == 0:
            return _fail(
                f"input `{name}`: cannot rebuild the tolerance of literal `{spec.lit}`"
            )
        number, unit = Decimal(m.group(1)), m.group(2)
        up = _tolerance_delta(spec.si, spec.max, number)
        dn = _tolerance_delta(spec.si, spec.min, number)
        text = f"{spec.lit.strip()} [+{up}{unit} -{dn}{unit}]"
    parsed = quantity(text)
    if parsed.is_err:
        return _fail(f"input `{name}`: {parsed.unwrap_err()}")
    return Ok(parsed.unwrap())


def _check_kind(
    what: str, name: str, q: Quantity, kind: str
) -> Result[Quantity, SolverProtocolError]:
    """Confirm `q` has the declared kind (retyping anonymous compounds)."""
    try:
        return Ok(q.as_kind(kind))
    except QuantityError as exc:
        return _fail(f"{what} `{name}` is not of kind {kind}: {exc}")


def _si_unit(q: Quantity) -> Result[str, SolverProtocolError]:
    """SI base-unit expression for the dimension of `q`, e.g. `kg*m^-1*s^-2`."""
    parts = [
        u if e == 1 else f"{u}^{e}"
        for u, e in zip(_BASE_UNITS, q.dim, strict=True)
        if e
    ]
    if q.angle:
        parts.append("rad" if q.angle == 1 else f"rad^{q.angle}")
    if not parts:
        return _fail("dimensionless value has no unit literal")
    return Ok("*".join(parts))


def format_output(
    name: str, q: Quantity, significant: int
) -> Result[str, SolverProtocolError]:
    """Render `q` as a decimal literal string rounded to `significant` digits."""
    unit = _si_unit(q)
    if unit.is_err:
        return _fail(f"output `{name}`: {unit.unwrap_err()}")
    u = unit.unwrap()
    ctx = Context(prec=significant, rounding=ROUND_HALF_EVEN)
    exact = q.to_exact(u)
    if exact is not None:
        value = ctx.divide(Decimal(exact.numerator), Decimal(exact.denominator))
    else:
        value = ctx.create_decimal(repr(q.to_float(u)))
    text = f"{_decimal_text(value)}{u}"
    # The consumer lexes this string; prove it is one valid literal now.
    back = quantity(text)
    if back.is_err:
        return _fail(f"output `{name}`: `{text}` is not a quantity literal")
    _log.debug("output %s = %s", name, text)
    return Ok(text)


class Solver:
    """A decorated solver function that can answer cadetta.solve/1 requests."""

    def __init__(
        self,
        func: Callable[..., Mapping[str, Any]],
        inputs: Mapping[str, str],
        outputs: Mapping[str, str],
        significant: int | Mapping[str, int],
        version: str | None,
    ) -> None:
        """Wrap `func` with its declared input/output kinds and rounding."""
        self.func = func
        self.inputs = dict(inputs)
        self.outputs = dict(outputs)
        self.version = version
        default = significant if isinstance(significant, int) else DEFAULT_SIGNIFICANT
        overrides = {} if isinstance(significant, int) else dict(significant)
        self.significant = {n: overrides.get(n, default) for n in self.outputs}
        self.__name__ = getattr(func, "__name__", "solver")
        self.__doc__ = func.__doc__

    def __call__(self, *args: Any, **kwargs: Any) -> Mapping[str, Any]:
        """Call the underlying function directly (no protocol, no checks)."""
        return self.func(*args, **kwargs)

    def handle(self, request_text: str) -> Result[str, SolverProtocolError]:
        """Answer one request text with the response JSON, or a protocol error."""
        try:
            req = _Request.model_validate_json(request_text)
        except ValidationError as exc:
            return _fail(f"malformed request: {exc.error_count()} problem(s): {exc}")
        if req.schema_ != SCHEMA:
            return _fail(f"wrong schema `{req.schema_}`, expected `{SCHEMA}`")
        if set(req.inputs) != set(self.inputs):
            return _fail(
                f"inputs {sorted(req.inputs)} != declared {sorted(self.inputs)}"
            )
        args: dict[str, Quantity] = {}
        for name, kind in self.inputs.items():
            built = _build_quantity(name, req.inputs[name])
            if built.is_err:
                return Err(built.unwrap_err())
            checked = _check_kind("input", name, built.unwrap(), kind)
            if checked.is_err:
                return Err(checked.unwrap_err())
            args[name] = checked.unwrap()
        _log.info("solving %s with %d inputs", req.solver, len(args))
        try:
            result = self.func(**args)
        except QuantityError as exc:
            return _fail(f"solver raised a quantity error: {exc} (code={exc.code})")
        if set(result) != set(self.outputs):
            return _fail(
                f"outputs {sorted(result)} do not match declared {sorted(self.outputs)}"
            )
        out: dict[str, str] = {}
        for name, kind in self.outputs.items():
            value = result[name]
            if not isinstance(value, Quantity):
                return _fail(
                    f"output `{name}` must be a Quantity, got {type(value).__name__}"
                )
            checked = _check_kind("output", name, value, kind)
            if checked.is_err:
                return Err(checked.unwrap_err())
            text = format_output(name, checked.unwrap(), self.significant[name])
            if text.is_err:
                return Err(text.unwrap_err())
            out[name] = text.unwrap()
        response: dict[str, Any] = {"schema": SCHEMA, "outputs": out}
        if self.version is not None:
            response["version"] = self.version
        return Ok(json.dumps(response, sort_keys=True))

    def serve(self, stdin: TextIO | None = None, stdout: TextIO | None = None) -> int:
        """Speak cadetta.solve/1 on stdin/stdout; returns the exit code (0 or 1)."""
        use_protocol_logging()
        src = stdin if stdin is not None else sys.stdin
        dst = stdout if stdout is not None else sys.stdout
        result = self.handle(src.read())
        if result.is_err:
            _log.error("serve failed: %s", result.unwrap_err())
            return 1
        dst.write(result.unwrap() + "\n")
        dst.flush()
        return 0


def solver(
    *,
    inputs: Mapping[str, str],
    outputs: Mapping[str, str],
    significant: int | Mapping[str, int] = DEFAULT_SIGNIFICANT,
    version: str | None = None,
) -> Callable[[Callable[..., Mapping[str, Any]]], Solver]:
    """Declare a function as a solver with kinds for each input and output."""

    def wrap(func: Callable[..., Mapping[str, Any]]) -> Solver:
        return Solver(func, inputs, outputs, significant, version)

    return wrap
