"""Write a CAD adapter by subclassing Adapter (bindings.md 3.4, adapters.md 5)."""

# frob:ticket 01M4SV2NZT32CAT33D13T86646
# frob:ticket 01M557E9DXNA27TQJG7HXSWJEE

from __future__ import annotations

import json
import re
import sys
from collections.abc import Callable
from enum import StrEnum
from fractions import Fraction
from pathlib import PurePath
from typing import Any, ClassVar, TextIO

from pydantic import BaseModel, ConfigDict, Field, ValidationError
from typani import Err, Ok, Result

from cadetta import Quantity, QuantityError
from cadetta.logging import get_logger, use_protocol_logging
from cadetta.models import ir

__all__ = [
    "PROTOCOL",
    "Adapter",
    "Capability",
    "ChangedParam",
    "ClientInfo",
    "Diagnostic",
    "EmitParams",
    "EmitResult",
    "Feature",
    "FileDigest",
    "FileState",
    "InitializeParams",
    "InitializeResult",
    "LocatorKind",
    "LocatorStatus",
    "OpenParams",
    "OpenResult",
    "PushMode",
    "PushParams",
    "PushResult",
    "PushStatus",
    "ReadParams",
    "ReadResult",
    "ReadSource",
    "RegenParams",
    "RegenResult",
    "RegenState",
    "RpcError",
    "SaveResult",
    "Session",
    "SessionParams",
    "TargetStatus",
    "ZERO_DIM",
    "observed_binding",
    "observed_member",
    "observed_project",
    "package_name",
    "ratio",
    "safe_name",
    "unit_scale",
]

PROTOCOL = "cadetta.adapter/1"
JSONRPC = "2.0"

# JSON-RPC 2.0 reserved error codes.
PARSE_ERROR = -32700
INVALID_REQUEST = -32600
METHOD_NOT_FOUND = -32601
INVALID_PARAMS = -32602
INTERNAL_ERROR = -32603

_log = get_logger(__name__)


class _Msg(BaseModel):
    """Base of protocol messages: unknown fields are refused, like the host does."""

    model_config = ConfigDict(extra="forbid")


class Capability(StrEnum):
    """An adapter capability as declared at initialize."""

    EMIT = "emit"
    READ = "read"
    DRIVE = "drive"


class PushMode(StrEnum):
    """How push writes a dimension: keep a link (ref) or set the number (value)."""

    REF = "ref"
    VALUE = "value"


class LocatorKind(StrEnum):
    """What a pushed locator addresses (adapters.md 3); a binding kind on the wire."""

    GLOBAL = "global"
    DIMENSION = "dimension"
    PROPERTY = "property"
    FRAME = "frame"


class PushStatus(StrEnum):
    """Outcome of one pushed locator."""

    PUSHED = "pushed"
    SKIPPED = "skipped"
    REJECTED = "rejected"
    FAILED = "failed"


class RegenState(StrEnum):
    """Outcome of regenerating one target."""

    OK = "ok"
    FAILED = "failed"


class Feature(StrEnum):
    """An optional adapter feature declared at initialize (adapters.md 5)."""

    FRAMES = "frames"
    INSTANCES = "instances"
    CREATE_FRAMES = "create_frames"
    TOLERANCES = "tolerances"
    RENAME_GLOBALS = "rename_globals"


class Diagnostic(BaseModel):
    """A cadetta.diag/1 record; location, notes and fixes pass through as data."""

    model_config = ConfigDict(extra="forbid", populate_by_name=True)

    schema_: str = Field(default="cadetta.diag/1", alias="schema")
    code: str
    severity: str
    message: str
    notes: list[Any] = Field(default_factory=list)
    fixes: list[Any] = Field(default_factory=list)
    path: str | None = None
    location: Any = None
    alt: str | None = None
    waived: Any = None


class ClientInfo(_Msg):
    """Name and version of the calling host."""

    name: str
    version: str


class InitializeParams(_Msg):
    """Params of initialize."""

    protocol: str
    client: ClientInfo


class InitializeResult(_Msg):
    """Result of initialize: identity, capabilities, native unit and precision."""

    name: str
    version: str
    capabilities: list[Capability]
    native_units: str
    precision: int
    platforms: list[str]
    # frob:ticket 01M4W6RRX0W5SZ2SNWN26XYW5F
    features: list[Feature] | None = None  # None is omitted on the wire: no features


class EmitParams(_Msg):
    """Params of emit: the design IR, the bindings table and the upstream policy."""

    design: ir.Project
    bindings: Any
    policy: Any


class FileDigest(_Msg):
    """A file path with the blake3 digest of its bytes."""

    path: str
    digest: str


class EmitResult(_Msg):
    """Result of emit."""

    files: list[FileDigest]
    diagnostics: list[Diagnostic] = []


class OpenParams(_Msg):
    """Params of open."""

    files: list[str]


class FileState(_Msg):
    """Dirty and read-only state of one opened file (R009)."""

    path: str
    dirty: bool
    readonly: bool


class Session(_Msg):
    """An open session: an opaque handle and per-file state."""

    handle: str
    files: list[FileState]


class OpenResult(_Msg):
    """Result of open."""

    session: Session
    diagnostics: list[Diagnostic] = []


class ReadSource(_Msg):
    """What read reads: file paths, or an open session (exactly one)."""

    files: list[str] | None = None
    session: str | None = None


class ReadParams(_Msg):
    """Params of read."""

    source: ReadSource


class ReadResult(_Msg):
    """Result of read: observed IR built with the same models as the design IR."""

    observed: ir.Project
    diagnostics: list[Diagnostic] = []


class ChangedParam(_Msg):
    """One changed parameter in a push."""

    path: str
    value: Any
    locator: str
    mode: PushMode
    ref_to: str | None = None
    kind: LocatorKind | None = None


class PushParams(_Msg):
    """Params of push."""

    session: str
    file: str
    params: list[ChangedParam]


class LocatorStatus(_Msg):
    """Per-locator result of a push."""

    locator: str
    status: PushStatus
    old: Any = None
    new: Any = None


class PushResult(_Msg):
    """Result of push: one status per locator, never only a summary."""

    statuses: list[LocatorStatus]
    diagnostics: list[Diagnostic] = []


class RegenParams(_Msg):
    """Params of regen."""

    session: str
    targets: list[str]


class TargetStatus(_Msg):
    """Per-target result of a regen."""

    target: str
    status: RegenState


class RegenResult(_Msg):
    """Result of regen."""

    targets: list[TargetStatus]
    diagnostics: list[Diagnostic] = []


class SessionParams(_Msg):
    """Params of save and close."""

    session: str


class SaveResult(_Msg):
    """Result of save."""

    files: list[FileDigest]
    diagnostics: list[Diagnostic] = []


class RpcError(BaseModel):
    """A JSON-RPC error a handler returns in Err (code, message, optional data)."""

    model_config = ConfigDict(frozen=True)

    code: int
    message: str
    data: Any = None


# method -> (params model, handler attribute, result model or None, drive only)
_METHODS: dict[
    str, tuple[type[BaseModel] | None, str, type[BaseModel] | None, bool]
] = {
    "initialize": (InitializeParams, "initialize", InitializeResult, False),
    "emit": (EmitParams, "emit", EmitResult, False),
    "open": (OpenParams, "open", OpenResult, True),
    "read": (ReadParams, "read", ReadResult, False),
    "push": (PushParams, "push", PushResult, True),
    "regen": (RegenParams, "regen", RegenResult, True),
    "save": (SessionParams, "save", SaveResult, True),
    "close": (SessionParams, "close", None, True),
    "shutdown": (None, "shutdown", None, False),
}


def _unimplemented(method: str) -> Err[RpcError]:
    """Build the method-not-found result for a method the subclass did not override."""
    return Err(
        RpcError(code=METHOD_NOT_FOUND, message=f"method not implemented: {method}")
    )


class Adapter:
    """Base of every external adapter: override the protocol methods you support.

    Each handler takes the parsed params model and returns
    `Result[<ResultModel>, RpcError]` (`close` and `shutdown` return
    `Result[None, RpcError]`). Unoverridden methods answer method-not-found.
    """

    #: Capabilities this adapter declared; drive-only methods are refused otherwise.
    capabilities: ClassVar[frozenset[Capability]] = frozenset()

    def initialize(
        self, params: InitializeParams
    ) -> Result[InitializeResult, RpcError]:
        """Report name, version, capabilities, native unit and precision."""
        return _unimplemented("initialize")

    def emit(self, params: EmitParams) -> Result[EmitResult, RpcError]:
        """Write the design's bound values to files."""
        return _unimplemented("emit")

    def open(self, params: OpenParams) -> Result[OpenResult, RpcError]:
        """Open documents and report per-file dirty and read-only state (drive)."""
        return _unimplemented("open")

    def read(self, params: ReadParams) -> Result[ReadResult, RpcError]:
        """Extract observed IR from files or a session."""
        return _unimplemented("read")

    def push(self, params: PushParams) -> Result[PushResult, RpcError]:
        """Write changed parameters, one status per locator (drive)."""
        return _unimplemented("push")

    def regen(self, params: RegenParams) -> Result[RegenResult, RpcError]:
        """Rebuild targets and report rebuild failures as diagnostics (drive)."""
        return _unimplemented("regen")

    def save(self, params: SessionParams) -> Result[SaveResult, RpcError]:
        """Save the documents the session modified (drive)."""
        return _unimplemented("save")

    def close(self, params: SessionParams) -> Result[None, RpcError]:
        """Close a session, closing only what the adapter opened (drive)."""
        return _unimplemented("close")

    def shutdown(self) -> Result[None, RpcError]:
        """Release the tool before the process exits."""
        return Ok(None)

    @classmethod
    def run(cls) -> int:
        """Serve on process stdio with stderr-only logging; return the exit code."""
        use_protocol_logging()
        out = sys.stdout
        # Anything else that prints lands on stderr, never in the protocol stream.
        sys.stdout = sys.stderr
        try:
            return cls().serve(sys.stdin, out)
        finally:
            sys.stdout = out

    def serve(self, stdin: TextIO, stdout: TextIO) -> int:
        """Run the newline-delimited JSON-RPC loop until shutdown or EOF; return 0."""
        _log.info("adapter %s serving", type(self).__name__)
        for line in stdin:
            if not line.strip():
                continue
            reply, stop = self._handle_line(line)
            if reply is not None:
                stdout.write(json.dumps(reply, separators=(",", ":")) + "\n")
                stdout.flush()
            if stop:
                _log.info("shutdown received; leaving the loop")
                return 0
        _log.info("stdin closed; leaving the loop")
        return 0

    def _handle_line(self, line: str) -> tuple[dict[str, Any] | None, bool]:
        """Parse one message and return (reply or None, whether to stop)."""
        try:
            msg = json.loads(line)
        except json.JSONDecodeError as e:
            _log.error("parse error: %s", e)
            return _error_reply(
                None, RpcError(code=PARSE_ERROR, message="parse error")
            ), False
        if (
            not isinstance(msg, dict)
            or msg.get("jsonrpc") != JSONRPC
            or "method" not in msg
        ):
            _log.error("invalid request: %r", msg)
            return _error_reply(
                None, RpcError(code=INVALID_REQUEST, message="invalid request")
            ), False
        rid = msg.get("id")
        method = msg["method"]
        _log.debug("request id=%r method=%s", rid, method)
        result = self._dispatch(method, msg.get("params"))
        stop = method == "shutdown" and isinstance(result, Ok)
        if "id" not in msg:
            return None, stop
        match result:
            case Ok(value):
                return {"jsonrpc": JSONRPC, "id": rid, "result": value}, stop
            case Err(err):
                return _error_reply(rid, err), False

    def _dispatch(self, method: Any, raw: Any) -> Result[Any, RpcError]:
        """Validate params, call the handler, validate its result against the schema."""
        spec = _METHODS.get(method) if isinstance(method, str) else None
        if spec is None:
            return Err(
                RpcError(code=METHOD_NOT_FOUND, message=f"unknown method: {method}")
            )
        params_model, attr, result_model, drive_only = spec
        if drive_only and Capability.DRIVE not in self.capabilities:
            return Err(
                RpcError(code=METHOD_NOT_FOUND, message=f"{method} requires drive")
            )
        handler: Callable[..., Result[Any, RpcError]] = getattr(self, attr)
        args: tuple[Any, ...] = ()
        if params_model is not None:
            try:
                args = (params_model.model_validate(raw),)
            except ValidationError as e:
                _log.warning("invalid params for %s: %s", method, e)
                return Err(
                    RpcError(
                        code=INVALID_PARAMS,
                        message=f"invalid params: {e.error_count()} error(s)",
                        data=e.errors(
                            include_url=False,
                            include_context=False,
                            include_input=False,
                        ),
                    )
                )
        try:
            outcome = handler(*args)
        except Exception:  # a handler raising is a programmer bug
            _log.exception("handler %s raised", method)
            return Err(
                RpcError(code=INTERNAL_ERROR, message=f"internal error in {method}")
            )
        match outcome:
            case Err(err):
                _log.warning("%s failed: %s", method, err.message)
                return Err(err)
            case Ok(value) if result_model is None:
                return Ok(None)
            case Ok(value):
                try:
                    checked = result_model.model_validate(
                        value.model_dump(by_alias=True)
                        if isinstance(value, BaseModel)
                        else value
                    )
                except ValidationError as e:
                    _log.error("%s produced an invalid result: %s", method, e)
                    return Err(
                        RpcError(
                            code=INTERNAL_ERROR, message=f"invalid result from {method}"
                        )
                    )
                return Ok(
                    checked.model_dump(mode="json", by_alias=True, exclude_none=True)
                )
        raise AssertionError("unreachable")  # pragma: no cover


def _error_reply(rid: Any, err: RpcError) -> dict[str, Any]:
    """Build a JSON-RPC error response."""
    body = err.model_dump(exclude_none=True)
    return {"jsonrpc": JSONRPC, "id": rid, "error": body}


# -- shared observed-IR helpers (read side of every adapter) -----------------

_SI_UNITS = ("m", "kg", "s", "A", "K", "mol", "cd")
#: The all-zero IR `dim` object; copy before filling in exponents.
ZERO_DIM: dict[str, Any] = {
    "A": 0,
    "K": 0,
    "angle": 0,
    "cd": 0,
    "currency": {},
    "kg": 0,
    "m": 0,
    "mol": 0,
    "s": 0,
}


def ratio(frac: Fraction, pi: int = 0) -> dict[str, Any]:
    """An exact nominal `{r, pi}` from a Fraction."""
    return {"r": f"{frac.numerator}/{frac.denominator}", "pi": pi}


def unit_scale(unit: str) -> tuple[dict[str, Any], Fraction, int, int]:
    """Return (dim, SI value of 1 `unit`, pi multiple, angle exponent) or raise."""
    one = Quantity._parse(f"1{unit}")
    angle = one.angle
    dim = dict(ZERO_DIM)
    if angle:
        deg = one.to_exact("deg")
        if deg is None:
            raise QuantityError(f"angle unit {unit} is not a rational of a degree")
        dim["angle"] = angle
        return dim, deg / 180, 1, angle
    parts = []
    for sym, e in zip(_SI_UNITS, list(one.dim), strict=True):
        if e:
            parts.append(f"{sym}^{e}")
            dim[sym] = e
    if not parts:
        raise QuantityError(f"unit {unit} is dimensionless")
    factor = one.to_exact("*".join(parts))
    if factor is None:
        raise QuantityError(f"unit {unit} has no exact SI scale")
    return dim, factor, 0, 0


def safe_name(raw: str, taken: set[str]) -> str:
    """Path-safe member name; non-alphanumerics become `_`, collisions get `_2`."""
    base = re.sub(r"[^A-Za-z0-9]", "_", raw) or "_"
    if base[0].isdigit():
        base = "_" + base
    name, n = base, 1
    while name in taken:
        n += 1
        name = f"{base}_{n}"
    taken.add(name)
    return name


def package_name(path: str | None) -> str:
    """Lowercase identifier-safe stem of `path`; `observed` when none."""
    name = (
        re.sub(r"[^a-z0-9_]", "_", PurePath(path).stem.lower()).strip("_")
        if path
        else ""
    )
    if not name:
        return "observed"
    return "p" + name if name[0].isdigit() else name


def observed_member(
    path: str,
    value: dict[str, Any],
    adapter: str,
    file: str,
    native_name: str,
    expr: str | None = None,
) -> dict[str, Any]:
    """An imported `param` member of the observed component."""
    member: dict[str, Any] = {
        "analysis": False,
        "attrs": {},
        "deps": [],
        "kind": "param",
        "path": path,
        "priv": False,
        "provenance": {
            "imported": {"adapter": adapter, "file": file, "native_name": native_name}
        },
        "value": value,
        "was": [],
    }
    if expr is not None:
        member["expr"] = expr
    return member


def observed_binding(
    adapter: str, path: str, kind: str, locator: str, native_unit: str
) -> dict[str, Any]:
    """A one-locator value binding for an observed member."""
    return {
        "adapter": adapter,
        "path": path,
        "locators": [
            {
                "kind": kind,
                "locator": locator,
                "mode": "value",
                "native_unit": native_unit,
            }
        ],
    }


def observed_project(
    pkg: str, comp: str, members: list[dict[str, Any]], bindings: list[dict[str, Any]]
) -> ir.Project:
    """Assemble and validate the observed IR project around one component."""
    return ir.Project.model_validate(
        {
            "schema": "cadetta.ir/1",
            "bindings": bindings,
            "checks": [],
            "citations": [],
            "components": [
                {
                    "attrs": {},
                    "class": "model",
                    "fits": [],
                    "implements": [],
                    "members": members,
                    "parts": [],
                    "path": comp,
                }
            ],
            "dependencies": [],
            "flavour": "observed",
            "modules": [],
            "package": {"name": pkg, "version": "0.0.0"},
            "solvers": [],
        }
    )
