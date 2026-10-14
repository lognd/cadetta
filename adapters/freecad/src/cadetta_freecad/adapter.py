"""FreeCADAdapter: the reference external adapter (adapters.md 5, 6)."""

# frob:ticket 01M4SPWSCZ06W3EW6B1RDPEZRQ

from __future__ import annotations

from typing import ClassVar

from cadetta.adapter import (
    PROTOCOL,
    Adapter,
    Capability,
    InitializeParams,
    InitializeResult,
    RpcError,
)
from cadetta.logging import get_logger
from typani import Err, Ok, Result

from cadetta_freecad import __version__
from cadetta_freecad.backend import FreeCadBackend, LiveBackend

__all__ = ["NATIVE_UNITS", "PRECISION", "FreeCadAdapter"]

#: FreeCAD's internal length unit.
NATIVE_UNITS = "mm"
#: Fractional decimal digits of the Spreadsheet number format the adapter writes.
PRECISION = 6

INVALID_PARAMS = -32602

_log = get_logger(__name__)


class FreeCadAdapter(Adapter):
    """Serves emit, read and drive for FreeCAD behind a backend seam."""

    capabilities: ClassVar[frozenset[Capability]] = frozenset(
        {Capability.EMIT, Capability.READ, Capability.DRIVE}
    )

    def __init__(self, backend: FreeCadBackend | None = None) -> None:
        """Use `backend` for FreeCAD access; the live module when omitted."""
        self._backend: FreeCadBackend = (
            backend if backend is not None else LiveBackend()
        )

    def initialize(
        self, params: InitializeParams
    ) -> Result[InitializeResult, RpcError]:
        """Report identity, the three capabilities, mm and the number precision."""
        if params.protocol != PROTOCOL:
            _log.error("unsupported protocol %r", params.protocol)
            return Err(
                RpcError(
                    code=INVALID_PARAMS,
                    message=f"unsupported protocol {params.protocol!r}; "
                    f"this adapter speaks {PROTOCOL}",
                )
            )
        _log.info("initialize from %s %s", params.client.name, params.client.version)
        match self._backend.freecad_version():
            case Ok(v):
                _log.info("FreeCAD %s", v)
            case Err(why):
                # initialize still succeeds; tool-touching methods report the failure.
                _log.warning("continuing without FreeCAD: %s", why)
        return Ok(
            InitializeResult(
                name="freecad",
                version=__version__,
                capabilities=sorted(self.capabilities),
                native_units=NATIVE_UNITS,
                precision=PRECISION,
                platforms=["linux", "windows", "macos"],
            )
        )
