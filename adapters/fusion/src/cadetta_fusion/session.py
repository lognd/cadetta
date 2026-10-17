"""The seam to Fusion: models, the session protocol and its JSON-RPC client."""

# frob:ticket 01M555TVRF47KEJ406NPQMSJDX

from __future__ import annotations

import json
import os
import socket
from typing import Any, Protocol, TextIO, TypeVar

from cadetta.logging import get_logger
from pydantic import BaseModel, ConfigDict, ValidationError
from typani import Err, Ok, Result

__all__ = [
    "ADDR_ENV",
    "DEFAULT_ADDR",
    "DocState",
    "FusionSession",
    "NativeParam",
    "ParamEdit",
    "RpcSession",
    "connect",
]

#: Environment variable naming the add-in's `host:port`.
ADDR_ENV = "CADETTA_FUSION_ADDR"
#: Where the in-Fusion add-in listens when the variable is unset.
DEFAULT_ADDR = "127.0.0.1:47360"

_log = get_logger(__name__)


class NativeParam(BaseModel):
    """One Fusion user parameter as the add-in reports it."""

    model_config = ConfigDict(frozen=True)

    name: str
    expression: str
    #: The parameter's unit text ("mm", "deg", ""); empty for unitless.
    unit: str = ""
    #: The evaluated value expressed in `unit` (not Fusion's internal cm/rad).
    value: float
    comment: str = ""


class ParamEdit(BaseModel):
    """One expression to assign to a user parameter in an atomic apply."""

    model_config = ConfigDict(frozen=True)

    name: str
    expression: str


class DocState(BaseModel):
    """An opened document: unsaved changes, read-only, and whether it was open."""

    model_config = ConfigDict(frozen=True)

    path: str
    dirty: bool = False
    readonly: bool = False
    attached: bool = False


class FusionSession(Protocol):
    """What the adapter needs from Fusion; tests substitute a fake."""

    def version(self) -> Result[str, str]:
        """Return the Fusion version, or why Fusion is unreachable."""
        ...

    def open_document(self, path: str) -> Result[DocState, str]:
        """Open (or attach to) the design archive at `path`."""
        ...

    def parameters(self, path: str) -> Result[list[NativeParam], str]:
        """List the user parameters of the open design `path`."""
        ...

    def apply(self, path: str, edits: list[ParamEdit]) -> Result[None, str]:
        """Assign all `edits` and recompute, or change nothing and say why."""
        ...

    def save_document(self, path: str) -> Result[None, str]:
        """Write the design back to its archive at `path`."""
        ...

    def close_document(self, path: str) -> Result[None, str]:
        """Close `path` without saving."""
        ...


_M = TypeVar("_M", bound=BaseModel)


def _model(model: type[_M], raw: Any) -> Result[_M, str]:
    """Validate one add-in payload into `model`."""
    try:
        return Ok(model.model_validate(raw))
    except ValidationError as e:
        return Err(f"bad reply from the Fusion add-in: {e}")


class RpcSession:
    """A FusionSession speaking newline-delimited JSON-RPC 2.0 to the add-in."""

    def __init__(self, reader: TextIO, writer: TextIO) -> None:
        """Talk over `reader` and `writer`; one request is in flight at a time."""
        self._reader = reader
        self._writer = writer
        self._next = 1

    def call(self, method: str, params: dict[str, Any]) -> Result[Any, str]:
        """Send one request and return its result, or the error text."""
        req = {"jsonrpc": "2.0", "id": self._next, "method": method, "params": params}
        self._next += 1
        _log.debug("-> %s", method)
        self._writer.write(json.dumps(req, separators=(",", ":")) + "\n")
        self._writer.flush()
        line = self._reader.readline()
        if not line:
            return Err("the Fusion add-in closed the connection")
        try:
            reply = json.loads(line)
        except json.JSONDecodeError as e:
            return Err(f"unparseable reply from the Fusion add-in: {e}")
        if reply.get("id") != req["id"]:
            return Err(f"reply id {reply.get('id')!r} does not match {req['id']}")
        if "error" in reply:
            message = reply["error"].get("message", "unknown error")
            _log.warning("%s failed: %s", method, message)
            return Err(str(message))
        return Ok(reply.get("result"))

    def version(self) -> Result[str, str]:
        """Return the Fusion version."""
        return self.call("fusion.version", {}).map(str)

    def open_document(self, path: str) -> Result[DocState, str]:
        """Open or attach to `path`."""
        return self.call("doc.open", {"path": path}).and_then(
            lambda r: _model(DocState, r)
        )

    def parameters(self, path: str) -> Result[list[NativeParam], str]:
        """List user parameters of `path`."""
        res = self.call("doc.parameters", {"path": path})
        if res.is_err:
            return Err(res.danger_err)
        out: list[NativeParam] = []
        for raw in res.danger_ok:
            match _model(NativeParam, raw):
                case Ok(p):
                    out.append(p)
                case Err(why):
                    return Err(why)
        return Ok(out)

    def apply(self, path: str, edits: list[ParamEdit]) -> Result[None, str]:
        """Apply all edits atomically."""
        payload = {"path": path, "edits": [e.model_dump() for e in edits]}
        return self.call("doc.apply", payload).map(lambda _: None)

    def save_document(self, path: str) -> Result[None, str]:
        """Save `path`."""
        return self.call("doc.save", {"path": path}).map(lambda _: None)

    def close_document(self, path: str) -> Result[None, str]:
        """Close `path` without saving."""
        return self.call("doc.close", {"path": path}).map(lambda _: None)


def connect(addr: str | None = None) -> Result[RpcSession, str]:
    """Connect to the add-in at `addr`, `$CADETTA_FUSION_ADDR` or the default."""
    target = addr or os.environ.get(ADDR_ENV) or DEFAULT_ADDR
    host, _, port = target.rpartition(":")
    try:
        sock = socket.create_connection((host, int(port)), timeout=30)
    except (OSError, ValueError) as e:
        _log.error("cannot reach the Fusion add-in at %s: %s", target, e)
        return Err(f"cannot reach the Fusion add-in at {target}: {e}")
    _log.info("connected to the Fusion add-in at %s", target)
    stream = sock.makefile("rw", encoding="utf-8", newline="\n")
    return Ok(RpcSession(stream, stream))
